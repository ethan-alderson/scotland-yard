// Need a legal actions function that takes in a gamestate and returns a list of legal actions
// Need a transition function that applies an action to a state

use super::gamestate::GameState;
use super::gamestate::PlayerState;
use super::gamestate::PlayerId;

use super::gamestate::Step;
use super::gamestate::Action;


// fn legal_steps(gamestate: &GameState) -> Vec<Step> {

// }

// fn legal_actions(gamestate: &GameState) -> Vec<Action> {
    
// }

fn apply_action<'board>(gamestate: &GameState<'board>, action: Action) -> GameState<'board> {
    match action {
        Action::Single(s) => apply_step(gamestate, s),
        Action::Double(s1, s2) => {
            let intermediate = apply_step(gamestate, s1);
            apply_step(&intermediate, s2)
        }
    }
}

fn apply_step<'board>(gamestate: &GameState<'board>, step: Step) -> GameState<'board> {
    let curr_player = &gamestate.players[gamestate.current_player];

    let mut new_inventory = curr_player.tickets.clone();
    new_inventory.spend_ticket(step.ticket);

    let new_pose = PlayerState {
        id: curr_player.id.clone(),
        station: step.to,
        tickets: new_inventory,
    };

    let mut new_players = gamestate.players.clone();
    new_players[gamestate.current_player] = new_pose;

    GameState {
        players: new_players,
        ..gamestate.clone()
    }
}

fn is_step_legal<'gs, 'board>(gamestate: &'gs GameState<'board>, step: Step) -> bool {
    let curr_player = &gamestate.players[gamestate.current_player];

    *curr_player.tickets.get(step.ticket) > 0 // current player has the required ticket
        && gamestate.board.neighbors(curr_player.station)
            .iter()
            .any(|tuple| tuple.0 == step.to) // step target is a neighbor
        && !gamestate.players.iter().any(|p| {
            matches!(p.id, PlayerId::Detective(_)) && p.station == step.to
        }) // step target is unoccupied by a detective
}

fn is_action_legal(gamestate: &GameState, action: Action) -> bool {

    if gamestate.is_terminal {
        return false;
    }

    let curr_player: &PlayerId = &gamestate.players[gamestate.current_player].id;

    match curr_player {
        PlayerId::Detective(n) => {
            match action {
                Action::Single(s) => is_step_legal(gamestate, s),
                Action::Double(s1,s2 ) => false
            }
        }
        PlayerId::MrX => {
            match action {
                Action::Single(s) => is_step_legal(gamestate, s),
                Action::Double(s1,s2 ) => {
                    // Requires apply_action to test the intermediate step
                    is_step_legal(gamestate, s1) && {
                    let intermediate = apply_step(gamestate, s1);
                    is_step_legal(&intermediate, s2)
}
                }
            }
        }
    }
}