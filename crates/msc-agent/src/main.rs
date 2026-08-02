//! `msc-agent` — the background service (§4 "Service mode" in
//! `msc2-engineering.md`). P2.12 built the axum skeleton and the dev-mode
//! bearer-auth gate; P2.13/P2.14 wire the route handlers behind it.

mod auth;
mod routes;
mod ws;

use std::net::SocketAddr;

use axum::Router;
use axum::routing::{get, post};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "msc-agent", about = "MSC 2 background agent")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start the agent's HTTP management API.
    Serve {
        /// Address to bind the management API to. Loopback by default
        /// (`msc2-engineering.md` §10: "the management API binds to
        /// loopback by default") — LAN/Tailscale binding is opt-in and
        /// not implemented by the skeletal agent.
        #[arg(long, default_value = "127.0.0.1:48400")]
        bind: SocketAddr,
    },
}

#[tokio::main]
async fn main() {
    let Cli { command } = Cli::parse();
    let Command::Serve { bind } = command;

    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .unwrap_or_else(|err| panic!("failed to bind {bind}: {err}"));

    println!("msc-agent listening on {bind}");

    axum::serve(listener, build_app())
        .await
        .expect("server error");
}

fn build_app() -> Router {
    let auth_state = auth::AuthState::empty_service_store();

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
        .with_state(operations_state);

    let console = Router::new()
        .route("/console/stream", get(ws::console::upgrade))
        .with_state(ws::console::ConsoleState::default());

    // Every other route this phase wires runs behind the SecretStore-backed
    // bearer-token check — including both WebSocket upgrades, since the auth
    // middleware runs on the ordinary HTTP request before the protocol
    // switch happens (websocket-v1.json: "evaluated before the WS-upgrade
    // special case is reached").
    let protected = Router::new()
        .route("/status", get(routes::status::status))
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
