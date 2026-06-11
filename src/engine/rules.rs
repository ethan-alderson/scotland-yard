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

pub fn legal_actions(gamestate: &GameState) -> Vec<Action> {
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

pub fn apply_action<'board>(gamestate: &GameState<'board>, action: Action) -> GameState<'board> {
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
                TicketInventory::new(1, 0, 0, 0),
            ),
        ];

        GameState::new(board, players)
    }
    
    fn branching_board() -> Board {
        Board {
            adjacency_map: vec![
                // Station 1
                vec![
                    (StationId { id: 2 }, TicketType::Taxi),
                    (StationId { id: 3 }, TicketType::Bus),
                ],

                // Station 2
                vec![(StationId { id: 1 }, TicketType::Taxi)],

                // Station 3
                vec![(StationId { id: 1 }, TicketType::Bus)],
            ],
        }
    }

    fn make_branching_state(
        mr_x_tickets: TicketInventory,
        detective_pos: StationId,
    ) -> GameState<'static> {
        let board = Box::leak(Box::new(branching_board()));

        let players = vec![
            PlayerState::new(
                PlayerId::MrX,
                StationId { id: 1 },
                mr_x_tickets,
            ),
            PlayerState::new(
                PlayerId::Detective(1),
                detective_pos,
                TicketInventory::new(1, 0, 0, 0),
            ),
        ];

        GameState::new(board, players)
    }

    fn chain_board() -> Board {
        Board {
            adjacency_map: vec![
                vec![(StationId { id: 2 }, TicketType::Taxi)],

                vec![
                    (StationId { id: 1 }, TicketType::Taxi),
                    (StationId { id: 3 }, TicketType::Taxi),
                ],

                vec![(StationId { id: 2 }, TicketType::Taxi)],
            ],
        }
    }

    fn make_chain_state(tickets: TicketInventory) -> GameState<'static> {
        let board = Box::leak(Box::new(chain_board()));

        let players = vec![
            PlayerState::new(
                PlayerId::MrX,
                StationId { id: 1 },
                tickets,
            ),
            PlayerState::new(
                PlayerId::Detective(1),
                StationId { id: 4 },
                TicketInventory::new(1, 0, 0, 0),
            ),
        ];

        GameState::new(board, players)
    }

    fn dead_end_board() -> Board {
        Board {
            adjacency_map: vec![
                vec![(StationId { id: 2 }, TicketType::Taxi)],
                vec![],
            ],
        }
    }

    fn make_dead_end_state() -> GameState<'static> {
        let board = Box::leak(Box::new(dead_end_board()));

        let players = vec![
            PlayerState::new(
                PlayerId::MrX,
                StationId { id: 1 },
                TicketInventory::new(2, 0, 0, 0),
            ),
            PlayerState::new(
                PlayerId::Detective(1),
                StationId { id: 3 },
                TicketInventory::new(1, 0, 0, 0),
            ),
        ];

        GameState::new(board, players)
    }
    
    fn make_three_player_state() -> GameState<'static> {
        let board = Box::leak(Box::new(tiny_board()));

        let players = vec![
            PlayerState::new(
                PlayerId::MrX,
                StationId { id: 1 },
                TicketInventory::new(1, 0, 0, 0),
            ),
            PlayerState::new(
                PlayerId::Detective(1),
                StationId { id: 2 },
                TicketInventory::new(1, 0, 0, 0),
            ),
            PlayerState::new(
                PlayerId::Detective(2),
                StationId { id: 3 },
                TicketInventory::new(1, 0, 0, 0),
            ),
        ];

        GameState::new(board, players)
    }

    mod is_step_legal_tests {
        use super::*;

        #[test]
        fn is_step_legal_valid_move() {
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
        fn is_step_legal_target_detective_occupied() {
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

        #[test]
        fn is_step_legal_target_mrx_occupied() {
            let mut state = make_state(
                StationId { id: 1 },
                TicketInventory::new(1, 0, 0, 0),
                StationId { id: 2 }, // detective is here
            );

            assert_eq!(state.players[state.current_player].station.id, 1);

            state.current_player = 1;

            let step = Step {
                to: StationId { id: 1 },
                ticket: TicketType::Taxi,
            };
        assert!(is_step_legal(&state, step));
        }
    }
    
    mod legal_steps_tests {

        use super::*;

        #[test]
        fn legal_steps_returns_all_legal_steps() {
            let state = make_branching_state(
                TicketInventory::new(1, 1, 0, 0), // taxi + bus
                StationId { id: 4 },              // not occupying either target
            );

            let steps = legal_steps(&state);

            assert_eq!(steps.len(), 2);

            assert!(steps.contains(&Step {
                to: StationId { id: 2 },
                ticket: TicketType::Taxi,
            }));

            assert!(steps.contains(&Step {
                to: StationId { id: 3 },
                ticket: TicketType::Bus,
            }));
        }

        #[test]
        fn legal_steps_filters_unavailable_tickets() {
            let state = make_branching_state(
                TicketInventory::new(1, 0, 0, 0), // taxi only
                StationId { id: 4 },
            );

            let steps = legal_steps(&state);

            assert_eq!(steps.len(), 1);

            assert_eq!(
                steps[0],
                Step {
                    to: StationId { id: 2 },
                    ticket: TicketType::Taxi,
                }
            );
        }

        #[test]
        fn legal_steps_filters_occupied_detective_locations() {
            let state = make_branching_state(
                TicketInventory::new(1, 1, 0, 0),
                StationId { id: 2 }, // detective occupies taxi destination
            );

            let steps = legal_steps(&state);

            assert_eq!(steps.len(), 1);

            assert_eq!(
                steps[0],
                Step {
                    to: StationId { id: 3 },
                    ticket: TicketType::Bus,
                }
            );
        }
    }
    
    mod legal_actions_tests {
        
        use super::*;
        
        #[test]
        fn legal_actions_detective_only_gets_single_actions() {
            let mut state = make_branching_state(
                TicketInventory::new(1, 1, 0, 0),
                StationId { id: 3 },
            );

            state.current_player = 1;

            let actions = legal_actions(&state);

            for action in actions {
                assert!(matches!(action, Action::Single(_)));
            }
        }

        #[test]
        fn legal_actions_mrx_gets_double_actions() {
            let state = make_chain_state(
                TicketInventory::new(2, 0, 0, 0),
            );

            let actions = legal_actions(&state);

            assert!(actions.contains(&Action::Single(
                Step {
                    to: StationId { id: 2 },
                    ticket: TicketType::Taxi,
                }
            )));

            assert!(actions.contains(&Action::Double(
                Step {
                    to: StationId { id: 2 },
                    ticket: TicketType::Taxi,
                },
                Step {
                    to: StationId { id: 3 },
                    ticket: TicketType::Taxi,
                }
            )));
        }

        #[test]
        fn legal_actions_no_double_when_second_step_impossible() {
            let state = make_dead_end_state();

            let actions = legal_actions(&state);

            assert_eq!(actions.len(), 1);

            assert!(matches!(actions[0], Action::Single(_)));

            assert!(
                !actions.iter().any(|a| matches!(a, Action::Double(_, _)))
            );
        }

        #[test]
        fn legal_actions_mrx_cannot_double_with_only_one_ticket() {
            let state = make_chain_state(
                TicketInventory::new(1, 0, 0, 0),
            );

            let actions = legal_actions(&state);

            assert!(
                !actions.iter().any(|a| matches!(a, Action::Double(_, _)))
            );
        }

    }
    
    mod apply_step_tests {
        
        use super::*;
        
        #[test]
        fn apply_step_updates_player_position() {
            let state = make_state(
                StationId { id: 1 },
                TicketInventory::new(1, 0, 0, 0),
                StationId { id: 3 },
            );

            let step = Step {
                to: StationId { id: 2 },
                ticket: TicketType::Taxi,
            };

            let new_state = apply_step(&state, step);

            assert_eq!(new_state.players[0].station.id, 2);
        }

        #[test]
        fn apply_step_spends_ticket() {
            let state = make_state(
                StationId { id: 1 },
                TicketInventory::new(2, 0, 0, 0),
                StationId { id: 3 },
            );

            let step = Step {
                to: StationId { id: 2 },
                ticket: TicketType::Taxi,
            };

            let new_state = apply_step(&state, step);

            assert_eq!(
                *new_state.players[0].tickets.get(TicketType::Taxi),
                1
            );
        }

        #[test]
        fn apply_step_leaves_other_players_unchanged() {
            let state = make_three_player_state();

            let step = Step {
                to: StationId { id: 2 },
                ticket: TicketType::Taxi,
            };

            let new_state = apply_step(&state, step);

            assert_eq!(state.players[1], new_state.players[1]);
            assert_eq!(state.players[2], new_state.players[2]);
        }

        #[test]
        fn apply_step_does_not_modify_original_state() {
            let state = make_state(
                StationId { id: 1 },
                TicketInventory::new(1, 0, 0, 0),
                StationId { id: 3 },
            );

            let step = Step {
                to: StationId { id: 2 },
                ticket: TicketType::Taxi,
            };

            let new_state = apply_step(&state, step);

            assert_eq!(state.players[0].station.id, 1);
            assert_eq!(new_state.players[0].station.id, 2);
        }
    }

    mod apply_action_tests {
        
        use super::*;

        #[test]
        fn apply_action_single_matches_apply_step() {
            let state = make_state(
                StationId { id: 1 },
                TicketInventory::new(1, 0, 0, 0),
                StationId { id: 3 },
            );

            let step = Step {
                to: StationId { id: 2 },
                ticket: TicketType::Taxi,
            };

            let via_step = apply_step(&state, step);
            let via_action = apply_action(&state, Action::Single(step));

            assert_eq!(via_step.players, via_action.players);
        }

        #[test]
        fn apply_action_double_reaches_final_station() {
            let state = make_chain_state(
                TicketInventory::new(2, 0, 0, 0),
            );

            let action = Action::Double(
                Step {
                    to: StationId { id: 2 },
                    ticket: TicketType::Taxi,
                },
                Step {
                    to: StationId { id: 3 },
                    ticket: TicketType::Taxi,
                },
            );

            let new_state = apply_action(&state, action);

            assert_eq!(new_state.players[0].station.id, 3);
        }

        #[test]
        fn apply_action_double_consumes_two_tickets() {
            let state = make_chain_state(
                TicketInventory::new(2, 0, 0, 0),
            );

            let action = Action::Double(
                Step {
                    to: StationId { id: 2 },
                    ticket: TicketType::Taxi,
                },
                Step {
                    to: StationId { id: 3 },
                    ticket: TicketType::Taxi,
                },
            );

            let new_state = apply_action(&state, action);

            assert_eq!(
                *new_state.players[0].tickets.get(TicketType::Taxi),
                0
            );
        }
    }

    mod is_action_legal_tests {
        
        use super::*;

        #[test]
        fn is_action_legal_terminal_state_rejects_all_actions() {
            let mut state = make_state(
                StationId { id: 1 },
                TicketInventory::new(1, 0, 0, 0),
                StationId { id: 3 },
            );

            state.is_terminal = true;

            let action = Action::Single(Step {
                to: StationId { id: 2 },
                ticket: TicketType::Taxi,
            });

            assert!(!is_action_legal(&state, action));
        }

        #[test]
        fn is_action_legal_detective_single_legal() {
            let mut state = make_state(
                StationId { id: 1 }, // Mr X
                TicketInventory::new(1, 0, 0, 0),
                StationId { id: 2 }, // Detective
            );

            state.current_player = 1;

            let action = Action::Single(Step {
                to: StationId { id: 1 },
                ticket: TicketType::Taxi,
            });

            assert!(is_action_legal(&state, action));
        }

        #[test]
        fn is_action_legal_detective_double_illegal() {
            let mut state = make_chain_state(
                TicketInventory::new(2, 0, 0, 0),
            );

            state.current_player = 1;

            let action = Action::Double(
                Step {
                    to: StationId { id: 2 },
                    ticket: TicketType::Taxi,
                },
                Step {
                    to: StationId { id: 3 },
                    ticket: TicketType::Taxi,
                },
            );

            assert!(!is_action_legal(&state, action));
        }

        #[test]
        fn is_action_legal_mrx_single_legal() {
            let state = make_state(
                StationId { id: 1 },
                TicketInventory::new(1, 0, 0, 0),
                StationId { id: 3 },
            );

            let action = Action::Single(Step {
                to: StationId { id: 2 },
                ticket: TicketType::Taxi,
            });

            assert!(is_action_legal(&state, action));
        }

        #[test]
        fn is_action_legal_mrx_double_legal() {
            let state = make_chain_state(
                TicketInventory::new(2, 0, 0, 0),
            );

            let action = Action::Double(
                Step {
                    to: StationId { id: 2 },
                    ticket: TicketType::Taxi,
                },
                Step {
                    to: StationId { id: 3 },
                    ticket: TicketType::Taxi,
                },
            );

            assert!(is_action_legal(&state, action));
        }

        #[test]
        fn is_action_legal_mrx_double_illegal_first_step() {
            let state = make_chain_state(
                TicketInventory::new(2, 0, 0, 0),
            );

            let action = Action::Double(
                Step {
                    to: StationId { id: 3 }, // not adjacent from 1
                    ticket: TicketType::Taxi,
                },
                Step {
                    to: StationId { id: 2 },
                    ticket: TicketType::Taxi,
                },
            );

            assert!(!is_action_legal(&state, action));
        }

        #[test]
        fn is_action_legal_mrx_double_illegal_second_step() {
            let state = make_chain_state(
                TicketInventory::new(2, 0, 0, 0),
            );

            let action = Action::Double(
                Step {
                    to: StationId { id: 2 },
                    ticket: TicketType::Taxi,
                },
                Step {
                    to: StationId { id: 1 }, // requires moving back from 2
                    ticket: TicketType::Bus, // wrong ticket
                },
            );

            assert!(!is_action_legal(&state, action));
        }

    }

    mod legality_invariant_tests {
        use super::*;

        #[test]
        fn legal_actions_are_all_legal() {
            let state = make_chain_state(
                TicketInventory::new(2, 0, 0, 0),
            );

            for action in legal_actions(&state) {
                assert!(
                    is_action_legal(&state, action),
                    "Generated action was not legal: {:?}",
                    action
                );
            }
        }

        #[test]
        fn legal_actions_are_all_legal_branching_board() {
            let state = make_branching_state(
                TicketInventory::new(1, 1, 0, 0),
                StationId { id: 4 },
            );

            for action in legal_actions(&state) {
                assert!(
                    is_action_legal(&state, action),
                    "Generated action was not legal: {:?}",
                    action
                );
            }
        }

        #[test]
        fn applying_legal_actions_only_changes_current_player() {
            let state = make_three_player_state();

            for action in legal_actions(&state) {
                let next = apply_action(&state, action);

                for i in 0..state.players.len() {
                    if i == state.current_player {
                        continue;
                    }

                    assert_eq!(
                        state.players[i],
                        next.players[i]
                    );
                }
            }
        }
    }
}