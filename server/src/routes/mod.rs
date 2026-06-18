//! HTTP routing.

pub mod games;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(|| async { "Scotland Yard API" }))
        .route("/api/games", post(games::create_game))
        .route("/api/games/{id}", get(games::get_game))
        .route("/api/games/{id}/legal_moves", get(games::legal_moves))
        .route("/api/games/{id}/moves", post(games::apply_move))
        .with_state(state)
}
