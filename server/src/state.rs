//! Shared application state: the loaded board and an in-memory store of live
//! games, plus the standard Scotland Yard presets.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use engine::board::Board;
use engine::gamestate::TicketInventory;
use engine::history::Game;

/// The 18 standard Scotland Yard start cards. Both Mr X and the detectives are
/// dealt distinct stations from this pool at game start.
pub const STANDARD_START_CARDS: [u8; 18] = [
    13, 26, 29, 34, 50, 53, 91, 94, 103, 112, 117, 132, 138, 141, 155, 174, 197, 198,
];

/// Standard Mr X loadout: 4 taxi, 3 bus, 3 underground, 5 black, 2 double.
pub fn standard_mr_x_tickets() -> TicketInventory {
    TicketInventory::new(4, 3, 3, 5, 2)
}

/// Standard detective loadout: 10 taxi, 8 bus, 4 underground, no black/double.
pub fn standard_detective_tickets() -> TicketInventory {
    TicketInventory::new(10, 8, 4, 0, 0)
}

/// Cloneable handle to the server's shared state. `Clone` is cheap — everything
/// inside is an `Arc`.
#[derive(Clone)]
pub struct AppState {
    pub board: Arc<Board>,
    games: Arc<RwLock<HashMap<String, Game>>>,
    next_id: Arc<AtomicU64>,
}

impl AppState {
    /// Load the board once at startup. The connections file path can be
    /// overridden with `SY_CONNECTIONS`; otherwise it resolves relative to this
    /// crate so the server runs from any working directory.
    pub fn new() -> Self {
        let path = std::env::var("SY_CONNECTIONS").unwrap_or_else(|_| {
            concat!(env!("CARGO_MANIFEST_DIR"), "/../engine/connections.txt").to_string()
        });
        let board = Arc::new(Board::from_connections_file(path));

        Self {
            board,
            games: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Store a new game and return its freshly allocated id.
    pub fn insert_game(&self, game: Game) -> String {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed).to_string();
        self.games.write().unwrap().insert(id.clone(), game);
        id
    }

    /// Read a game under a shared lock. Returns `None` if the id is unknown.
    pub fn with_game<R>(&self, id: &str, f: impl FnOnce(&Game) -> R) -> Option<R> {
        let games = self.games.read().unwrap();
        games.get(id).map(f)
    }

    /// Mutate a game under an exclusive lock. A move is a read-modify-write, so it
    /// takes the write lock for the whole operation. We never `.await` while
    /// holding the lock, so a std `RwLock` is fine.
    pub fn with_game_mut<R>(&self, id: &str, f: impl FnOnce(&mut Game) -> R) -> Option<R> {
        let mut games = self.games.write().unwrap();
        games.get_mut(id).map(f)
    }
}
