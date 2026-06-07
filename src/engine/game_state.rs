mod board;

use crate::board::Board;
use crate::board::TicketType;

enum PlayerId {
    MrX,
    Detective(u8)
}

#[derive(Copy, Clone)]
struct TicketSet {
    taxi: u8,
    bus: u8,
    underground: u8
}

struct PlayerState {
    id: PlayerId,
    node: u8,
    tickets: TicketSet
}

struct Step {
    to: u8,
    ticket: TicketType,
}

enum Action {
    Single(Step),
    Double(Step, Step),
}

/*

What else does game state need?

*/
struct GameState<'a> {
    
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

struct GameHistory {
    // We need some notion of the partially observable information, something encoding Mr X's move history.
    // need his known positions and also the ticket he used every turn
    mr_x_revealed_positions: Vec<Option<u8>>,
    mr_x_actions: Vec<Action>,
}