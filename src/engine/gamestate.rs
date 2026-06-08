
use super::board::Board;
use super::board::TicketType;

pub enum PlayerId {
    MrX,
    Detective(u8)
}

#[derive(Copy, Clone)]
pub struct TicketSet {
    taxi: u8,
    bus: u8,
    underground: u8,
    water: u8
}

impl TicketSet {
    pub fn new(taxi: u8, bus: u8, underground: u8, water: u8) -> Self {
        Self { taxi, bus, underground, water }
    }
}

pub struct PlayerState {
    id: PlayerId,
    node: u8,
    tickets: TicketSet
}

impl PlayerState {
    pub fn new(id: PlayerId, node: u8, tickets: TicketSet) -> Self {
        Self { id, node, tickets }
    }
}

struct Step {
    to: u8,
    ticket: TicketType,
}

enum Action {
    Single(Step),
    Double(Step, Step),
}

pub struct GameState<'a> {
    
    // notion of fully observable information

    board: &'a Board,
    players: Vec<PlayerState>,
    // The player moving as an index in the player vector
    current_player: usize,
    turn_number: usize,

    // We also need termination

    is_terminal: bool,
    winner: Option<PlayerId>,

    // Some notion of move history for debugging can be added later

}

impl<'a> GameState<'a> {
    pub fn new(board: &'a Board, players: Vec<PlayerState>) -> Self {
        Self {
            board,
            players,
            current_player: 0,
            turn_number: 0,
            is_terminal: false,
            winner: None,
        }
    }
}

struct GameHistory {
    // We need some notion of the partially observable information, something encoding Mr X's move history.
    // need his known positions and also the ticket he used every turn
    mr_x_revealed_positions: Vec<Option<u8>>,
    mr_x_actions: Vec<Action>,
}