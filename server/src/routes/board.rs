//! Static board geometry endpoint.

use axum::extract::State;
use axum::Json;

use crate::board_geometry::BoardDto;
use crate::state::AppState;

/// `GET /api/board` — station coordinates, edges, and image metadata. The
/// payload is static (built at startup) and small (≈200 stations, ≈470 edges),
/// so cloning it per request is cheap.
pub async fn get_board(State(state): State<AppState>) -> Json<BoardDto> {
    Json((*state.geometry).clone())
}
