//! `msc-agent` — the background service (§4 "Service mode" in
//! `msc2-engineering.md`). This step (P2.12) is the axum skeleton and the
//! dev-mode bearer-auth gate only; route handlers are wired starting
//! P2.13.

mod auth;

use std::net::SocketAddr;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
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
    // GET /v1/health is the one route the dev-mode auth gate does not
    // cover (docs/msc2/api-contract/auth-scope-phase2.md §3, item 1).
    let public = Router::new().route("/health", get(health));

    // Every other route this phase wires runs behind the bearer-token
    // check. `/status` here is a placeholder proving the gate itself
    // works; P2.13 replaces it with the real canned `RemoteApiStatus` DTO.
    let protected = Router::new()
        .route("/status", get(status_placeholder))
        .route_layer(axum::middleware::from_fn(auth::require_bearer_token));

    Router::new().nest("/v1", public.merge(protected))
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn status_placeholder() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({ "placeholder": true })),
    )
}
