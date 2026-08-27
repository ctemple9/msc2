//! `msc-agent` — the background service (§4 "Service mode" in
//! `msc2-engineering.md`). P2.12 built the axum skeleton and the dev-mode
//! bearer-auth gate; P2.13/P2.14 wire the route handlers behind it.

mod auth;
mod backup_operations;
mod backup_scheduler;
mod cli;
mod help;
mod routes;
mod web_ui;
mod ws;

use std::net::SocketAddr;
use std::process::ExitCode;

use axum::Extension;
use axum::Router;
use axum::routing::{delete, get, post};
use clap::Parser;

#[derive(Parser)]
#[command(name = "msc", about = "MSC 2 service and CLI")]
struct App {
    #[command(flatten)]
    common: cli::CommonArgs,
    #[command(subcommand)]
    command: cli::Command,
}

#[tokio::main]
async fn main() -> ExitCode {
    let App { common, command } = App::parse();
    let result = match command {
        cli::Command::Serve { bind } => run_service(bind).await,
        #[cfg(target_os = "linux")]
        cli::Command::CredentialHelper { command } => run_credential_helper(command),
        command => cli::run(common, command).await,
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            error.print();
            ExitCode::from(error.exit_code())
        }
    }
}

#[cfg(target_os = "linux")]
fn run_credential_helper(command: cli::CredentialHelperCommand) -> Result<(), cli::CliError> {
    match command {
        cli::CredentialHelperCommand::Serve {
            allowed_uid,
            store_dir,
            socket_path,
        } => msc_platform_linux::credential_helper::run_helper_service(
            allowed_uid,
            store_dir,
            socket_path,
        )
        .map_err(cli::CliError::internal),
    }
}

async fn run_service(bind: SocketAddr) -> Result<(), cli::CliError> {
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(|err| cli::CliError::internal(format!("failed to bind {bind}: {err}")))?;

    println!("msc listening on {bind}");

    let auth_state = auth::production_auth_state();
    #[cfg(target_os = "macos")]
    auth::spawn_local_bootstrap(auth_state.clone());

    axum::serve(listener, build_app_with_auth(auth_state))
        .await
        .map_err(|err| cli::CliError::internal(format!("server error: {err}")))
}

#[allow(dead_code)]
pub(crate) fn build_app() -> Router {
    build_app_with_auth(auth::production_auth_state())
}

fn build_app_with_auth(auth_state: auth::AuthState) -> Router {
    let secret_store = auth::production_secret_store()
        .unwrap_or_else(|error| panic!("failed to initialize production secret store: {error}"));

    // Shared by the HTTP operation-lifecycle routes and the
    // operation-progress WebSocket route below — both must observe the
    // same in-memory map, not two independent ones.
    let operations_state = routes::operations::OperationsState::default();

    let operations = Router::new()
        .route("/operations", post(routes::operations::create))
        .route("/operations/:id", get(routes::operations::get))
        .route("/operations/:id/cancel", post(routes::operations::cancel))
        .with_state(operations_state.clone());

    let operation_progress = Router::new()
        .route("/operations/:id/stream", get(ws::operations::upgrade))
        .with_state(operations_state.clone());

    let app_config = Box::leak(Box::new(
        routes::lifecycle::AgentAppConfigStore::production_migrating_legacy_secrets(
            secret_store.as_ref(),
        )
        .expect("failed to load durable MSC 2 application config"),
    ));
    let console_state = ws::console::ConsoleState::default();
    let bedrock_runtime = routes::bedrock_runtime::BedrockRuntimeSelection::production(app_config);
    let lifecycle_state =
        routes::lifecycle::LifecycleRoutesState::with_app_config_and_auth_and_bedrock(
            console_state.clone(),
            operations_state.clone(),
            app_config,
            auth_state.clone(),
            bedrock_runtime,
        );
    let notification_state = ws::notifications::NotificationState::default();
    let networking_state = routes::networking::NetworkingState::new(
        lifecycle_state.clone(),
        operations_state.clone(),
        secret_store.clone(),
    );
    let help_content =
        help::HelpContent::embedded().expect("embedded educational content must be valid");

    // GET /v1/health is the one route the dev-mode auth gate does not
    // cover (docs/msc2/api-contract/auth-scope-phase2.md §3, item 1).
    // P7.24 replaces its Phase 2 canned card with real health-card data,
    // which needs the same `LifecycleRoutesState` every protected route
    // reads.
    let browser_pairings = Router::new()
        .route(
            "/auth/pairings",
            post(routes::browser_session::create_pairing),
        )
        .layer(Extension(auth_state.clone()))
        .route_layer(axum::middleware::from_fn_with_state(
            auth_state.clone(),
            auth::require_bearer_token,
        ));
    let browser_public = Router::new()
        .route(
            "/auth/browser-sessions",
            post(routes::browser_session::exchange_browser_session),
        )
        .layer(Extension(auth_state.clone()));
    let desktop_public = Router::new()
        .route(
            "/auth/desktop-pairings",
            post(routes::desktop_session::exchange_desktop_pairing),
        )
        .layer(Extension(auth_state.clone()));
    let browser_protected = Router::new()
        .route("/auth/csrf", get(routes::browser_session::csrf_token))
        .route(
            "/auth/browser-sessions/current",
            delete(routes::browser_session::logout_browser_session),
        );

    let public = Router::new()
        .route("/health", get(routes::health::health))
        .merge(browser_pairings)
        .merge(browser_public)
        .merge(desktop_public)
        .with_state(lifecycle_state.clone());

    // P6.17: one scheduled-backup timer per configured server, started
    // from whatever `auto_backup_*` settings exist at boot. Leaked
    // (matching every other long-lived singleton this function already
    // builds) so its background tasks outlive `build_app()`'s own stack
    // frame — see `backup_scheduler.rs`'s module doc for what "live
    // reconfiguration" does and doesn't cover yet.
    let scheduler_backend = std::sync::Arc::new(backup_scheduler::LiveSchedulerBackend::new(
        lifecycle_state.clone(),
        app_config,
    ));
    let backup_scheduler = Box::leak(Box::new(backup_scheduler::BackupScheduler::new(
        scheduler_backend,
    )));
    backup_scheduler.reconfigure(&app_config.servers());

    let console = Router::new()
        .route("/console/stream", get(ws::console::upgrade))
        .route("/console/tail", get(ws::console::tail))
        .with_state(console_state);

    // P6.21: world/backup routes, backed by the real P6.9-19 services.
    // `worlds` gets its own `WorldsRoutesState` (adds the in-process
    // staged-upload/download store); `backups` gets its own
    // `BackupsRoutesState` (adds the `&'static BackupScheduler` handle
    // `POST /v1/backups/config` reconfigures on a settings change).
    let shared_staging = routes::worlds::StagingStore::default();
    let worlds = routes::worlds::router(routes::worlds::WorldsRoutesState::with_staging(
        lifecycle_state.clone(),
        shared_staging.clone(),
    ));
    let components = routes::components::router(routes::components::ComponentsRoutesState::new(
        lifecycle_state.clone(),
        shared_staging.clone(),
    ));
    let backups = routes::backups::router(routes::backups::BackupsRoutesState {
        lifecycle: lifecycle_state.clone(),
        scheduler: backup_scheduler,
    });
    // P7.23: the template routes get their own small `Router` (matching
    // `worlds`/`backups`'s own precedent) since `GET /v1/templates` needs
    // no route param but does need `LifecycleRoutesState`.
    let templates = routes::templates::router(lifecycle_state.clone());
    let users = Router::new()
        .route(
            "/users",
            get(routes::users::list).post(routes::users::create),
        )
        .route("/users/update", post(routes::users::update))
        .route("/users/revoke", post(routes::users::revoke));

    let lifecycle = Router::new()
        .route("/servers", get(routes::servers::list))
        .route("/servers/import", post(routes::servers::import))
        .route("/servers/create", post(routes::servers::create))
        .route("/servers/delete", post(routes::servers::delete))
        .route("/servers/rename", post(routes::servers::rename))
        .route("/servers/eula", post(routes::servers::eula))
        .route("/active-server", post(routes::lifecycle::active_server))
        .route("/start", post(routes::lifecycle::start))
        .route("/stop", post(routes::lifecycle::stop))
        .route("/command", post(routes::commands::command))
        .route("/status", get(routes::status::status))
        .route("/performance", get(routes::performance::performance))
        .route(
            "/connectivity",
            get(routes::network_diagnostics::connectivity),
        )
        .route(
            "/duckdns",
            get(routes::network_diagnostics::duckdns_status)
                .post(routes::network_diagnostics::update_duckdns),
        )
        .route(
            "/config/geyser",
            get(routes::geyser::get_config).post(routes::geyser::update_config),
        )
        .route(
            "/settings",
            get(routes::settings::get_settings).post(routes::settings::update_settings),
        )
        // P7.24: version, Java runtime, and diagnostics routes.
        .route("/versions", get(routes::versions::versions))
        .route(
            "/versions/create",
            get(routes::versions::versions_for_create),
        )
        .route(
            "/components/version",
            post(routes::versions::change_version),
        )
        .route("/java-runtimes", get(routes::versions::java_runtimes))
        .route(
            "/java-runtimes/install",
            post(routes::versions::install_java_runtime),
        )
        .route(
            "/config/java-runtime",
            get(routes::versions::get_java_config).post(routes::versions::set_java_config),
        )
        .route(
            "/config/servers-root",
            get(routes::versions::get_servers_root).post(routes::versions::set_servers_root),
        )
        .route(
            "/config/ram",
            get(routes::versions::get_ram_config).post(routes::versions::set_ram_config),
        )
        .route("/health/problems", get(routes::health::health_problems))
        .route("/health/repair", post(routes::health::health_repair))
        .with_state(lifecycle_state.clone())
        // A staged modpack is created by the shared upload route but
        // redeemed by server creation, so both route groups need this one
        // in-memory, purpose-tagged store.
        .layer(Extension(shared_staging));

    let players = routes::players::router(lifecycle_state.clone());
    let session_log = routes::session_log::router(lifecycle_state.clone());
    let files = routes::files::router(lifecycle_state.clone());
    let player_mutations = Router::new()
        .route("/players/delete", post(routes::players::delete_player_data))
        .route(
            "/players/migrate-offline",
            post(routes::players::migrate_player_offline),
        )
        .route("/players/migrate", post(routes::players::migrate_player))
        .route(
            "/players/duplicate",
            post(routes::players::duplicate_player_data),
        )
        .with_state(lifecycle_state.clone());
    let bedrock = Router::new()
        .route(
            "/allowlist",
            get(routes::bedrock::get_allowlist).post(routes::bedrock::mutate_allowlist),
        )
        .with_state(lifecycle_state.clone());

    // Every other route this phase wires runs behind the SecretStore-backed
    // bearer-token check — including both WebSocket upgrades, since the auth
    // middleware runs on the ordinary HTTP request before the protocol
    // switch happens (websocket-v1.json: "evaluated before the WS-upgrade
    // special case is reached").
    let protected = Router::new()
        .merge(lifecycle)
        .merge(players)
        .merge(session_log)
        .merge(files)
        .merge(player_mutations)
        .merge(bedrock)
        .route("/capabilities", get(routes::capabilities::capabilities))
        .route("/me", get(routes::capabilities::me))
        .merge(operations)
        .merge(operation_progress)
        .merge(console)
        .merge(routes::networking::router(networking_state.clone()))
        .merge(
            Router::new()
                .route("/notifications/stream", get(ws::notifications::upgrade))
                .with_state(notification_state),
        )
        .merge(worlds)
        .merge(components)
        .merge(backups)
        .merge(templates)
        .merge(users)
        .merge(routes::help::router(help_content))
        .merge(browser_protected)
        .layer(Extension(networking_state))
        .layer(Extension(auth_state.clone()))
        .route_layer(axum::middleware::from_fn_with_state(
            auth_state,
            auth::require_management_auth,
        ));

    Router::new()
        .nest("/v1", public.merge(protected))
        .fallback(get(web_ui::serve))
        .layer(axum::middleware::from_fn(security_headers))
}

async fn security_headers(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        "Content-Security-Policy",
        axum::http::HeaderValue::from_static(
            "default-src 'self'; base-uri 'none'; object-src 'none'; frame-ancestors 'none'; form-action 'self'; script-src 'self'; style-src 'self'; img-src 'self' data: blob:; connect-src 'self'; worker-src 'self' blob:",
        ),
    );
    headers.insert(
        "X-Content-Type-Options",
        axum::http::HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        "Referrer-Policy",
        axum::http::HeaderValue::from_static("no-referrer"),
    );
    response
}
