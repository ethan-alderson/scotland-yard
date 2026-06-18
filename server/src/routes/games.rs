//! Game lifecycle handlers: create, read (god view), legal moves, apply move.

use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use engine::board::StationId;
use engine::gamestate::{GameState, PlayerId, PlayerState};
use engine::history::{Game, RevealSchedule};
use engine::rules::legal_actions;

use crate::dto::{GameStateDto, LegalMovesDto, MoveRequest, NewGameRequest};
use crate::error::ApiError;
use crate::state::{
    standard_detective_tickets, standard_mr_x_tickets, AppState, STANDARD_START_CARDS,
};

const MAX_DETECTIVES: usize = 5;

/// `POST /api/games` — create a standard game and return its initial god-view
/// state.
pub async fn create_game(
    State(state): State<AppState>,
    Json(req): Json<NewGameRequest>,
) -> Result<Json<GameStateDto>, ApiError> {
    if req.detectives < 1 || req.detectives > MAX_DETECTIVES {
        return Err(ApiError::bad_request(format!(
            "detectives must be between 1 and {MAX_DETECTIVES}"
        )));
    }

    // Resolve start stations: either both explicit or both dealt.
    let (mr_x_start, detective_starts) = match (req.mr_x_start, req.detective_starts) {
        (Some(mx), Some(ds)) => {
            if ds.len() != req.detectives {
                return Err(ApiError::bad_request(
                    "detective_starts length must equal detectives",
                ));
            }
            (mx, ds)
        }
        (None, None) => deal_starts(req.detectives, req.seed),
        _ => {
            return Err(ApiError::bad_request(
                "provide both mr_x_start and detective_starts, or neither",
            ))
        }
    };

    // Validate stations: in range and all distinct (so nobody starts caught).
    let station_count = state.board.adjacency_map.len();
    let mut seen = HashSet::new();
    for s in std::iter::once(mr_x_start).chain(detective_starts.iter().copied()) {
        if s < 1 || s as usize > station_count {
            return Err(ApiError::bad_request(format!("station {s} out of range")));
        }
        if !seen.insert(s) {
            return Err(ApiError::bad_request(format!("duplicate start station {s}")));
        }
    }

    // Build the roster (MrX must be index 0) and the game.
    let mut players = Vec::with_capacity(req.detectives + 1);
    players.push(PlayerState::new(
        PlayerId::MrX,
        StationId { id: mr_x_start },
        standard_mr_x_tickets(),
    ));
    for (i, st) in detective_starts.iter().enumerate() {
        players.push(PlayerState::new(
            PlayerId::Detective((i + 1) as u8),
            StationId { id: *st },
            standard_detective_tickets(),
        ));
    }

    let game_state = GameState::new(Arc::clone(&state.board), players);
    let game = Game::new(game_state, RevealSchedule::standard());

    let id = state.insert_game(game);
    let dto = state
        .with_game(&id, |g| GameStateDto::from_game(&id, g))
        .expect("game was just inserted");
    Ok(Json(dto))
}

/// `GET /api/games/:id` — full god-view state.
pub async fn get_game(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<GameStateDto>, ApiError> {
    state
        .with_game(&id, |g| GameStateDto::from_game(&id, g))
        .map(Json)
        .ok_or_else(|| ApiError::not_found("game not found"))
}

/// `GET /api/games/:id/legal_moves` — the current player's legal moves.
pub async fn legal_moves(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<LegalMovesDto>, ApiError> {
    state
        .with_game(&id, |g| LegalMovesDto::from_game(g))
        .map(Json)
        .ok_or_else(|| ApiError::not_found("game not found"))
}

/// `POST /api/games/:id/moves` — apply one action and return the new state.
pub async fn apply_move(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<MoveRequest>,
) -> Result<Json<GameStateDto>, ApiError> {
    let action = req.to_action().map_err(ApiError::bad_request)?;

    let outcome = state.with_game_mut(&id, |game| {
        if game.state.is_terminal {
            return Err(ApiError::bad_request("game is already over"));
        }
        // Validate against the engine's own generator: a submitted action is
        // legal iff the engine would have generated it. This keeps the API in
        // lockstep with the rules with no separate validation logic.
        if !legal_actions(&game.state).contains(&action) {
            return Err(ApiError::bad_request("illegal move for the current player"));
        }
        game.apply(action);
        Ok(GameStateDto::from_game(&id, game))
    });

    match outcome {
        Some(Ok(dto)) => Ok(Json(dto)),
        Some(Err(e)) => Err(e),
        None => Err(ApiError::not_found("game not found")),
    }
}

/// Deal distinct start stations from the standard start cards: the first to
/// Mr X, the rest to the detectives. A `seed` makes the deal reproducible.
fn deal_starts(detectives: usize, seed: Option<u64>) -> (u8, Vec<u8>) {
    let mut pool = STANDARD_START_CARDS.to_vec();
    match seed {
        Some(s) => pool.shuffle(&mut rand::rngs::StdRng::seed_from_u64(s)),
        None => pool.shuffle(&mut rand::thread_rng()),
    }
    let mr_x = pool[0];
    let detective_starts = pool[1..=detectives].to_vec();
    (mr_x, detective_starts)
}
