//! Scotland Yard web API.
//!
//! Phase 1 (see FRONTEND_PLAN.md): an in-memory game store and the REST
//! lifecycle for playing a game over HTTP — create, read (god view), list legal
//! moves, apply a move. No UI yet; this is driven by `scripts/smoke.sh`.

mod board_geometry;
mod dto;
mod error;
mod routes;
mod state;

use std::net::SocketAddr;

use state::AppState;

#[tokio::main]
async fn main() {
    let state = AppState::new();
    let app = routes::router(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Scotland Yard API listening on http://{addr}");
    axum::serve(listener, app).await.unwrap();
}
