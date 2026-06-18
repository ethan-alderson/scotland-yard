use super::board::Board;
use super::board::TicketType;
use super::board::StationId;

use std::sync::Arc;

use serde::{Serialize, Deserialize};

#[derive(PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum PlayerId {
    MrX,
    Detective(u8),
}

// The outcome of a finished game. Detectives win as a team, so the winner is not
// a single player identity — keeping this separate from PlayerId makes the
// "detectives win" outcome representable without inventing an impossible player.
#[derive(PartialEq, Eq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Winner {
    MrX,
    Detectives,
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct TicketInventory {
    taxi: u8,
    bus: u8,
    underground: u8,
    black: u8,
    pub double: u8,
}

// CONSTRUCTORS
impl TicketInventory {
    pub fn new(taxi: u8, bus: u8, underground: u8, black: u8, double: u8) -> Self {
        Self { taxi, bus, underground, black, double }
    }
}

// GETTERS
impl TicketInventory {
    pub fn get(&self, tt: TicketType) -> &u8 {
        match tt {
            TicketType::Taxi => &self.taxi,
            TicketType::Bus => &self.bus,
            TicketType::Underground => &self.underground,
            TicketType::Black => &self.black
        }
    }

    pub fn get_mut(&mut self, tt: TicketType) -> &mut u8 {
        match tt {
            TicketType::Taxi => &mut self.taxi,
            TicketType::Bus => &mut self.bus,
            TicketType::Underground => &mut self.underground,
            TicketType::Black => &mut self.black
        }
    }
}

// Behavior
impl TicketInventory {
    // Plain `*slot -= 1` panics on underflow in debug builds but silently wraps to
    // 255 in release (where overflow checks are off) — and rollouts run in release.
    // checked_sub().expect() turns that silent corruption into a loud panic in every
    // build mode, so a spend without the matching ticket fails fast instead of
    // handing a player 255 tickets.
    pub fn spend_ticket(&mut self, tt: TicketType) {
        let slot = self.get_mut(tt);
        *slot = slot
            .checked_sub(1)
            .expect("attempted to spend a ticket the player does not hold");
    }

    pub fn add_ticket(&mut self, tt: TicketType) {
        *self.get_mut(tt) += 1;
    }

    pub fn spend_double(&mut self) {
        self.double = self
            .double
            .checked_sub(1)
            .expect("attempted to spend a double ticket the player does not hold");
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PlayerState {
    pub id: PlayerId,
    pub station: StationId,
    pub tickets: TicketInventory
}

impl PlayerState {
    pub fn new(id: PlayerId, station: StationId, tickets: TicketInventory) -> Self {
        Self { id, station, tickets }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Step {
    pub to: StationId,
    pub ticket: TicketType,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Action {
    Single(Step),
    Double(Step, Step),
    // A detective with no legal move forfeits their turn. Position and tickets
    // are unchanged; only turn order advances. MrX never passes — if MrX cannot
    // move the detectives win (see the cornered terminal check in apply_action).
    Pass,
}

#[derive(Clone)]
pub struct GameState {

    // notion of fully observable information

    pub board: Arc<Board>,
    pub players: Vec<PlayerState>,
    // The player moving as an index in the player vector
    pub current_player: usize,
    pub turn_number: usize,

    // We also need termination

    pub is_terminal: bool,
    pub winner: Option<Winner>,
    pub max_turns: usize,

    // Some notion of move history for debugging can be added later

}

impl GameState {
    pub fn new(board: Arc<Board>, players: Vec<PlayerState>) -> Self {
        // A valid roster is exactly one MrX and at least one detective. Checking
        // the counts first also rejects an empty player list with a clear message
        // before any indexing.
        let mrx_count = players.iter().filter(|p| matches!(p.id, PlayerId::MrX)).count();
        let detective_count = players
            .iter()
            .filter(|p| matches!(p.id, PlayerId::Detective(_)))
            .count();

        assert_eq!(
            mrx_count, 1,
            "exactly one MrX required; found {}",
            mrx_count
        );
        assert!(
            detective_count >= 1,
            "at least one detective required; found {}",
            detective_count
        );

        // The rules engine treats index 0 as MrX: turn rotation calls a round
        // complete when play wraps back to player 0, and the turn-limit and
        // cornered terminal checks hang off that. Enforce the invariant here so a
        // mis-ordered player list fails loudly at construction instead of
        // silently corrupting termination logic.
        assert!(
            matches!(players[0].id, PlayerId::MrX),
            "players[0] must be MrX; found {:?}",
            players[0].id
        );

        let mut state = Self {
            board,
            players,
            current_player: 0,
            turn_number: 0,
            is_terminal: false,
            winner: None,
            max_turns: 22,
        };

        // Run the terminal checks at construction so a state built with MrX
        // already caught or cornered is marked terminal immediately. Without
        // this, such a state is non-terminal yet yields no legal action — the
        // zero-action hole that breaks search and the determinizer.
        if let Some(winner) = crate::rules::detect_terminal(&state) {
            state.is_terminal = true;
            state.winner = Some(winner);
        }

        state
    }
}

#[derive(Serialize)]
pub struct StateResponse {
    pub current_player: usize,
    pub players: Vec<PlayerState>,
    pub round: usize,
}

impl From<&GameState> for StateResponse {
    fn from(gs: &GameState) -> Self {
        Self {
            current_player: gs.current_player,
            players: gs.players.clone(),
            round: gs.turn_number,
        }
    }
}

struct GameHistory {
    // We need some notion of the partially observable information, something encoding Mr X's move history.
    // need his known positions and also the ticket he used every turn
    mr_x_revealed_positions: Vec<Option<u8>>,
    mr_x_actions: Vec<Action>,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assert_ticket_get() {
        let inv = TicketInventory::new(1, 2, 3, 4, 5);

        assert_eq!(*inv.get(TicketType::Taxi), 1);
        assert_eq!(*inv.get(TicketType::Bus), 2);
        assert_eq!(*inv.get(TicketType::Underground), 3);
        assert_eq!(*inv.get(TicketType::Black), 4);
        assert_eq!(inv.double, 5);
    }

    #[test]
    fn assert_spend_ticket_subtraction() {
        let mut inv = TicketInventory::new(1, 0, 0, 0, 0);

        inv.spend_ticket(TicketType::Taxi);
        assert_eq!(*inv.get(TicketType::Taxi), 0);
    }

    #[test]
    #[should_panic(expected = "does not hold")]
    fn spend_ticket_underflow_panics_in_all_builds() {
        // Without checked_sub this would wrap to 255 in release; assert it panics
        // instead, so the guard holds regardless of overflow-checks.
        let mut inv = TicketInventory::new(0, 0, 0, 0, 0);
        inv.spend_ticket(TicketType::Taxi);
    }

    #[test]
    #[should_panic(expected = "does not hold")]
    fn spend_double_underflow_panics_in_all_builds() {
        let mut inv = TicketInventory::new(0, 0, 0, 0, 0);
        inv.spend_double();
    }

    #[test]
    fn add_ticket_increments_taxi() {
        let mut inv = TicketInventory::new(0, 0, 0, 0, 0);
        inv.add_ticket(TicketType::Taxi);
        assert_eq!(*inv.get(TicketType::Taxi), 1);
    }

    #[test]
    fn add_ticket_increments_bus() {
        let mut inv = TicketInventory::new(0, 0, 0, 0, 0);
        inv.add_ticket(TicketType::Bus);
        assert_eq!(*inv.get(TicketType::Bus), 1);
    }

    #[test]
    fn add_ticket_increments_underground() {
        let mut inv = TicketInventory::new(0, 0, 0, 0, 0);
        inv.add_ticket(TicketType::Underground);
        assert_eq!(*inv.get(TicketType::Underground), 1);
    }

    #[test]
    fn add_ticket_increments_black() {
        let mut inv = TicketInventory::new(0, 0, 0, 0, 0);
        inv.add_ticket(TicketType::Black);
        assert_eq!(*inv.get(TicketType::Black), 1);
    }

    #[test]
    fn spend_double_decrements_count() {
        let mut inv = TicketInventory::new(0, 0, 0, 0, 2);
        inv.spend_double();
        assert_eq!(inv.double, 1);
    }

    #[test]
    fn add_ticket_does_not_affect_other_tickets() {
        let mut inv = TicketInventory::new(1, 1, 1, 1, 1);
        inv.add_ticket(TicketType::Taxi);
        assert_eq!(*inv.get(TicketType::Bus), 1);
        assert_eq!(*inv.get(TicketType::Underground), 1);
        assert_eq!(*inv.get(TicketType::Black), 1);
        assert_eq!(inv.double, 1);
    }

    fn two_station_board() -> Arc<Board> {
        Arc::new(Board {
            adjacency_map: vec![vec![(StationId { id: 2 }, TicketType::Taxi)], vec![]],
        })
    }

    fn mrx(station: u8) -> PlayerState {
        PlayerState::new(PlayerId::MrX, StationId { id: station }, TicketInventory::new(1, 0, 0, 0, 0))
    }

    fn detective(n: u8, station: u8) -> PlayerState {
        PlayerState::new(PlayerId::Detective(n), StationId { id: station }, TicketInventory::new(1, 0, 0, 0, 0))
    }

    #[test]
    fn new_accepts_one_mrx_and_one_detective() {
        let players = vec![mrx(1), detective(1, 2)];

        let state = GameState::new(two_station_board(), players);
        assert!(matches!(state.players[0].id, PlayerId::MrX));
    }

    #[test]
    #[should_panic(expected = "players[0] must be MrX")]
    fn new_rejects_non_mrx_at_index_zero() {
        // Valid counts (one MrX, one detective) but wrong ordering, so the failure
        // is specifically the index-0 check.
        let players = vec![detective(1, 1), mrx(2)];
        GameState::new(two_station_board(), players);
    }

    #[test]
    #[should_panic(expected = "at least one detective required")]
    fn new_rejects_no_detectives() {
        let players = vec![mrx(1)];
        GameState::new(two_station_board(), players);
    }

    #[test]
    #[should_panic(expected = "exactly one MrX required")]
    fn new_rejects_missing_mrx() {
        let players = vec![detective(1, 1), detective(2, 2)];
        GameState::new(two_station_board(), players);
    }

    #[test]
    #[should_panic(expected = "exactly one MrX required")]
    fn new_rejects_two_mrx() {
        let players = vec![mrx(1), mrx(2), detective(1, 3)];
        GameState::new(two_station_board(), players);
    }

    #[test]
    #[should_panic(expected = "exactly one MrX required")]
    fn new_rejects_empty_player_list() {
        GameState::new(two_station_board(), vec![]);
    }

    #[test]
    fn new_marks_caught_state_terminal() {
        // A detective sharing MrX's station means MrX is already caught; the
        // constructor must flag this rather than yield a live, actionless state.
        let players = vec![mrx(2), detective(1, 2)];
        let state = GameState::new(two_station_board(), players);

        assert!(state.is_terminal);
        assert_eq!(state.winner, Some(Winner::Detectives));
    }

    #[test]
    fn new_marks_cornered_mrx_terminal() {
        // MrX at station 2 (a dead end on two_station_board) with a taxi ticket
        // has no edge to leave by, so he is cornered the moment the game starts.
        let players = vec![
            PlayerState::new(PlayerId::MrX, StationId { id: 2 }, TicketInventory::new(1, 0, 0, 0, 0)),
            detective(1, 1),
        ];
        let state = GameState::new(two_station_board(), players);

        assert!(state.is_terminal);
        assert_eq!(state.winner, Some(Winner::Detectives));
        // The invariant this fix protects: a non-terminal state always has a move,
        // so a terminal one is the only way the action list may be empty.
        assert!(crate::rules::legal_actions(&state).is_empty());
    }

    #[test]
    fn new_leaves_live_state_non_terminal() {
        // MrX at 1 can taxi to the empty station 2; nobody is caught. The state
        // must start live with a move available.
        let players = vec![mrx(1), detective(1, 2)];
        // Place the detective off MrX's only exit so MrX still has a move.
        let board = Arc::new(Board {
            adjacency_map: vec![
                vec![(StationId { id: 2 }, TicketType::Taxi), (StationId { id: 3 }, TicketType::Taxi)],
                vec![(StationId { id: 1 }, TicketType::Taxi)],
                vec![(StationId { id: 1 }, TicketType::Taxi)],
            ],
        });
        let state = GameState::new(board, players);

        assert!(!state.is_terminal);
        assert_eq!(state.winner, None);
        assert!(!crate::rules::legal_actions(&state).is_empty());
    }
}