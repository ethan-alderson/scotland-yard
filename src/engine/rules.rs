use super::gamestate::GameState;
use super::gamestate::PlayerState;
use super::gamestate::PlayerId;

use super::gamestate::Step;
use super::gamestate::Action;


fn legal_steps(gamestate: &GameState) -> Vec<Step> {
    let curr_player = &gamestate.players[gamestate.current_player];
    let mut steps = Vec::new();

    for &(dest, tt) in gamestate.board.neighbors(curr_player.station) {
        let step = Step {
            to: dest,
            ticket: tt,
        };

        if is_step_legal(gamestate, step) {
            steps.push(step);
        }
    }

    steps
}

fn legal_actions(gamestate: &GameState) -> Vec<Action> {
    let single_steps = legal_steps(gamestate);
    let curr_player = &gamestate.players[gamestate.current_player];

    let mut actions: Vec<Action> = single_steps
    .iter()
    .copied()
    .map(Action::Single)
    .collect();

    if matches!(curr_player.id, PlayerId::MrX) {
        for first_step in &single_steps {
            let intermediate = apply_step(gamestate, *first_step);

            for second_step in legal_steps(&intermediate) {
                actions.push(Action::Double(*first_step, second_step));
            }
        }
    }
    actions
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

fn apply_action<'board>(gamestate: &GameState<'board>, action: Action) -> GameState<'board> {
    match action {
        Action::Single(s) => apply_step(gamestate, s),
        Action::Double(s1, s2) => {
            let intermediate = apply_step(gamestate, s1);
            apply_step(&intermediate, s2)
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    
    use crate::Board;
    use crate::StationId;
    use crate::engine::board::TicketType;
    use crate::TicketInventory;

    fn tiny_board() -> Board {
        Board {
            adjacency_map: vec![
                // Station 1
                vec![(StationId { id: 2 }, TicketType::Taxi)],

                // Station 2
                vec![
                    (StationId { id: 1 }, TicketType::Taxi),
                    (StationId { id: 3 }, TicketType::Bus),
                ],

                // Station 3
                vec![(StationId { id: 2 }, TicketType::Bus)],
            ],
        }
    }

    fn make_state(
        mr_x_pos: StationId,
        mr_x_tickets: TicketInventory,
        detective_pos: StationId,
    ) -> GameState<'static> {
        let board = Box::leak(Box::new(tiny_board()));

        let players = vec![
            PlayerState::new(PlayerId::MrX, mr_x_pos, mr_x_tickets),
            PlayerState::new(
                PlayerId::Detective(1),
                detective_pos,
                TicketInventory::new(0, 0, 0, 0),
            ),
        ];

        GameState::new(board, players)
    }
    
    #[test]
    fn assert_step_legal_valid_move() {
        let state = make_state(
            StationId { id: 1 },
            TicketInventory::new(1, 0, 0, 0),
            StationId { id: 3 },
        );

        let step = Step {
            to: StationId { id: 2 },
            ticket: TicketType::Taxi,
        };

        assert!(is_step_legal(&state, step));
    }

    #[test]
    fn is_step_legal_missing_ticket() {
        let state = make_state(
            StationId { id: 1 },
            TicketInventory::new(0, 0, 0, 0),
            StationId { id: 3 },
        );

        let step = Step {
            to: StationId { id: 2 },
            ticket: TicketType::Taxi,
        };

        assert!(!is_step_legal(&state, step));
    }

    #[test]
    fn is_step_legal_non_neighbor() {
        let state = make_state(
            StationId { id: 1 },
            TicketInventory::new(1, 0, 0, 0),
            StationId { id: 3 },
        );

        let step = Step {
            to: StationId { id: 3 },
            ticket: TicketType::Taxi,
        };

        assert!(!is_step_legal(&state, step));
    }

    #[test]
    fn is_step_legal_target_occupied() {
        let state = make_state(
            StationId { id: 1 },
            TicketInventory::new(1, 0, 0, 0),
            StationId { id: 2 }, // detective is here
        );

        let step = Step {
            to: StationId { id: 2 },
            ticket: TicketType::Taxi,
        };

        assert!(!is_step_legal(&state, step));
    }
}