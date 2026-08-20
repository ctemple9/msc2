//! `msc-agent` — the background service (§4 "Service mode" in
//! `msc2-engineering.md`). P2.12 built the axum skeleton and the dev-mode
//! bearer-auth gate; P2.13/P2.14 wire the route handlers behind it.

mod auth;
mod backup_operations;
mod backup_scheduler;
mod cli;
mod routes;
mod ws;

use std::net::SocketAddr;
use std::process::ExitCode;

use axum::Router;
use axum::routing::{get, post};
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

    axum::serve(listener, build_app())
        .await
        .map_err(|err| cli::CliError::internal(format!("server error: {err}")))
}

pub(crate) fn build_app() -> Router {
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
    let auth_state =
        auth::AuthState::persistent_service_store_with_secret_store(secret_store.clone());
    let console_state = ws::console::ConsoleState::default();
    let lifecycle_state = routes::lifecycle::LifecycleRoutesState::with_app_config_and_auth(
        console_state.clone(),
        operations_state.clone(),
        app_config,
        auth_state.clone(),
    );

    // GET /v1/health is the one route the dev-mode auth gate does not
    // cover (docs/msc2/api-contract/auth-scope-phase2.md §3, item 1).
    // P7.24 replaces its Phase 2 canned card with real health-card data,
    // which needs the same `LifecycleRoutesState` every protected route
    // reads.
    let public = Router::new()
        .route("/health", get(routes::health::health))
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
    let worlds = routes::worlds::router(routes::worlds::WorldsRoutesState::new(
        lifecycle_state.clone(),
    ));
    let backups = routes::backups::router(routes::backups::BackupsRoutesState {
        lifecycle: lifecycle_state.clone(),
        scheduler: backup_scheduler,
    });
    // P7.23: the template routes get their own small `Router` (matching
    // `worlds`/`backups`'s own precedent) since `GET /v1/templates` needs
    // no route param but does need `LifecycleRoutesState`.
    let templates = routes::templates::router(lifecycle_state.clone());

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
            "/config/ram",
            get(routes::versions::get_ram_config).post(routes::versions::set_ram_config),
        )
        .route("/health/problems", get(routes::health::health_problems))
        .route("/health/repair", post(routes::health::health_repair))
        .with_state(lifecycle_state);

    // Every other route this phase wires runs behind the SecretStore-backed
    // bearer-token check — including both WebSocket upgrades, since the auth
    // middleware runs on the ordinary HTTP request before the protocol
    // switch happens (websocket-v1.json: "evaluated before the WS-upgrade
    // special case is reached").
    let protected = Router::new()
        .merge(lifecycle)
        .route("/capabilities", get(routes::capabilities::capabilities))
        .route("/me", get(routes::capabilities::me))
        .merge(operations)
        .merge(operation_progress)
        .merge(console)
        .merge(worlds)
        .merge(backups)
        .merge(templates)
        .route_layer(axum::middleware::from_fn_with_state(
            auth_state,
            auth::require_bearer_token,
        ));

    Router::new().nest("/v1", public.merge(protected))
}
