//! `msc-agent` — the background service (§4 "Service mode" in
//! `msc2-engineering.md`). P2.12 built the axum skeleton and the dev-mode
//! bearer-auth gate; P2.13/P2.14 wire the route handlers behind it.

mod auth;
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
    let auth_state = auth::AuthState::default_persistent_service_store();

    // GET /v1/health is the one route the dev-mode auth gate does not
    // cover (docs/msc2/api-contract/auth-scope-phase2.md §3, item 1).
    let public = Router::new().route("/health", get(routes::health::health));

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

    let console_state = ws::console::ConsoleState::default();
    let lifecycle_state = routes::lifecycle::LifecycleRoutesState::new(
        console_state.clone(),
        operations_state.clone(),
    );

    let console = Router::new()
        .route("/console/stream", get(ws::console::upgrade))
        .route("/console/tail", get(ws::console::tail))
        .with_state(console_state);

    let lifecycle = Router::new()
        .route("/servers", get(routes::servers::list))
        .route("/servers/import", post(routes::servers::import))
        .route("/active-server", post(routes::lifecycle::active_server))
        .route("/start", post(routes::lifecycle::start))
        .route("/stop", post(routes::lifecycle::stop))
        .route("/command", post(routes::commands::command))
        .route("/status", get(routes::status::status))
        .route("/performance", get(routes::performance::performance))
        .with_state(lifecycle_state);

    // Every other route this phase wires runs behind the SecretStore-backed
    // bearer-token check — including both WebSocket upgrades, since the auth
    // middleware runs on the ordinary HTTP request before the protocol
    // switch happens (websocket-v1.json: "evaluated before the WS-upgrade
    // special case is reached").
    let protected = Router::new()
        .merge(lifecycle)
        .route("/capabilities", get(routes::capabilities::capabilities))
        .merge(operations)
        .merge(operation_progress)
        .merge(console)
        .route_layer(axum::middleware::from_fn_with_state(
            auth_state,
            auth::require_bearer_token,
        ));

    Router::new().nest("/v1", public.merge(protected))
}
