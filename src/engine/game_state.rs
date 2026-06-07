mod board;

use crate::board::Board;

enum PlayerId {
    MrX,
    Detective(u8)
}

#[derive(Copy, Clone)]
struct TicketSet {
    taxi: u8,
    bus: u8,
    underground: u8,
}

struct PlayerState {
    id: PlayerId,
    node: u8,
    tickets: TicketSet
}

/*

What else does game state need?

*/
struct GameState<'a> {
    board: &'a Board,
    players: Vec<PlayerState>,
    // Not the turn number in game history, this is which player is moving.
    current_turn: usize

}