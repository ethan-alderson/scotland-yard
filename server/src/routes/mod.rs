//! HTTP routing.

pub mod board;
pub mod games;

use axum::routing::{get, post};
use axum::Router;
use tower_http::services::ServeDir;

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    // Serve map.png / pos.txt straight off disk (handles range requests + caching
    // headers, which matters for the ~13 MB map image).
    let assets = ServeDir::new(state.assets_dir.as_ref().as_path());

    Router::new()
        .route("/", get(|| async { "Scotland Yard API" }))
        .route("/api/board", get(board::get_board))
        .route("/api/games", post(games::create_game))
        .route("/api/games/{id}", get(games::get_game))
        .route("/api/games/{id}/view", get(games::view_game))
        .route("/api/games/{id}/legal_moves", get(games::legal_moves))
        .route("/api/games/{id}/moves", post(games::apply_move))
        .nest_service("/assets", assets)
        .with_state(state)
}
