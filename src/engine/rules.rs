// Need a legal actions function that takes in a gamestate and returns a list of legal actions
// Need a transition function that applies an action to a state

use super::gamestate::GameState;
use super::gamestate::PlayerState;
use super::gamestate::PlayerId;
use super::gamestate::TicketInventory;

use super::gamestate::Step;
use super::gamestate::Action;

fn legal_actions(gamestate: &GameState) -> Vec<Action> {
    println!("pass");

    let curr_player: &PlayerState = &gamestate.players[gamestate.current_player];
    let mut legal_moves: Vec<Action> = vec![];

    //     // index the board given a 1 indexed value to isolate the indexing difference
    // pub fn neighbors(&self, sid: StationId) -> &Vec<(StationId, TicketType)> {
    //     &self.adjacency_map[sid.id as usize - 1]
    // }

    // core logic to search for single steps across all players
    for &(dest, tt) in gamestate.board.neighbors(curr_player.station) {
        if *curr_player.tickets.get(tt) > 0{
            legal_moves.push(Action::Single(
                Step {to: dest, ticket: tt}
            ));
        }
    }
    legal_moves
}
    // could implement branching here for players / mrx, could also prune if mrx is curr player and has a black ticket
    // since that implies all neighbors are valid. These seem like future optimizations once correctness is confirmed.
    /* if matches!(curr_player.id, PlayerId::Detective(_)) {

    } else {
        // executes if the current player is mr x, add black
         ticket prune here?
    } */
    
