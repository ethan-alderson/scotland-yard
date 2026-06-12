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

fn apply_step(gamestate: &GameState, step: Step) -> GameState {
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

    if matches!(curr_player.id, PlayerId::Detective(_)) {
        let mrx_idx = new_players
            .iter()
            .position(|p| matches!(p.id, PlayerId::MrX))
            .expect("no MrX in players");
        new_players[mrx_idx].tickets.add_ticket(step.ticket);
    }

    GameState {
        players: new_players,
        ..gamestate.clone()
    }
}

pub fn apply_action(gamestate: &GameState, action: Action) -> GameState {
    let mut new_state = match action {
        Action::Single(s) => apply_step(gamestate, s),
        Action::Double(s1, s2) => {
            let intermediate = apply_step(gamestate, s1);
            apply_step(&intermediate, s2)
        }
    };

    let next_player = (new_state.current_player + 1) % new_state.players.len();
    new_state.current_player = next_player;
    if next_player == 0 {
        new_state.turn_number += 1;
    }

    // Terminal: a detective landed on MrX's station.
    let mrx_station = new_state.players.iter()
        .find(|p| matches!(p.id, PlayerId::MrX))
        .map(|p| p.station);

    if let Some(mrx_station) = mrx_station {
        if new_state.players.iter().any(|p| {
            matches!(p.id, PlayerId::Detective(_)) && p.station == mrx_station
        }) {
            new_state.is_terminal = true;
            new_state.winner = Some(PlayerId::Detectives);
            return new_state;
        }
    }

    // Remaining checks only apply when a full round just completed (MrX's turn next).
    if next_player == 0 {
        // Terminal: turn limit reached, MrX wins.
        if new_state.turn_number >= new_state.max_turns {
            new_state.is_terminal = true;
            new_state.winner = Some(PlayerId::MrX);
            return new_state;
        }

        // Terminal: MrX has no legal moves, detectives win.
        if legal_actions(&new_state).is_empty() {
            new_state.is_terminal = true;
            new_state.winner = Some(PlayerId::Detectives);
            return new_state;
        }
    }

    new_state
}

fn is_step_legal(gamestate: &GameState, step: Step) -> bool {
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
        PlayerId::Detective(_) => {
            match action {
                Action::Single(s) => is_step_legal(gamestate, s),
                Action::Double(_, _) => false
            }
        }
        PlayerId::Detectives => false,
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
    
    use crate::board::Board;
    use crate::board::StationId;
    use crate::board::TicketType;
    use crate::gamestate::TicketInventory;

    use std::sync::Arc;

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
    ) -> GameState {
        let board = Arc::new(tiny_board());

        let players = vec![
            PlayerState::new(PlayerId::MrX, mr_x_pos, mr_x_tickets),
            PlayerState::new(
                PlayerId::Detective(1),
                detective_pos,
                TicketInventory::new(1, 0, 0, 0, 0),
            ),
        ];

        GameState::new(board.clone(), players)
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
    ) -> GameState {
        let board = Arc::new(branching_board());

        let players = vec![
            PlayerState::new(
                PlayerId::MrX,
                StationId { id: 1 },
                mr_x_tickets,
            ),
            PlayerState::new(
                PlayerId::Detective(1),
                detective_pos,
                TicketInventory::new(1, 0, 0, 0, 0),
            ),
        ];

        GameState::new(board.clone(), players)
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

    fn make_chain_state(tickets: TicketInventory) -> GameState {
        let board = Arc::new(chain_board());

        let players = vec![
            PlayerState::new(
                PlayerId::MrX,
                StationId { id: 1 },
                tickets,
            ),
            PlayerState::new(
                PlayerId::Detective(1),
                StationId { id: 4 },
                TicketInventory::new(1, 0, 0, 0, 0),
            ),
        ];

        GameState::new(board.clone(), players)
    }

    fn dead_end_board() -> Board {
        Board {
            adjacency_map: vec![
                vec![(StationId { id: 2 }, TicketType::Taxi)],
                vec![],
            ],
        }
    }

    fn make_dead_end_state() -> GameState {
        let board = Arc::new(dead_end_board());

        let players = vec![
            PlayerState::new(
                PlayerId::MrX,
                StationId { id: 1 },
                TicketInventory::new(2, 0, 0, 0, 0),
            ),
            PlayerState::new(
                PlayerId::Detective(1),
                StationId { id: 3 },
                TicketInventory::new(1, 0, 0, 0, 0),
            ),
        ];

        GameState::new(board.clone(), players)
    }
    
    fn make_three_player_state() -> GameState {
        let board = Arc::new(tiny_board());

        let players = vec![
            PlayerState::new(
                PlayerId::MrX,
                StationId { id: 1 },
                TicketInventory::new(1, 0, 0, 0, 0),
            ),
            PlayerState::new(
                PlayerId::Detective(1),
                StationId { id: 2 },
                TicketInventory::new(1, 0, 0, 0, 0),
            ),
            PlayerState::new(
                PlayerId::Detective(2),
                StationId { id: 3 },
                TicketInventory::new(1, 0, 0, 0, 0),
            ),
        ];

        GameState::new(board.clone(), players)
    }

    mod is_step_legal_tests {
        use super::*;

        #[test]
        fn is_step_legal_valid_move() {
            let state = make_state(
                StationId { id: 1 },
                TicketInventory::new(1, 0, 0, 0, 0),
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
                TicketInventory::new(0, 0, 0, 0, 0),
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
                TicketInventory::new(1, 0, 0, 0, 0),
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
                TicketInventory::new(1, 0, 0, 0, 0),
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
                TicketInventory::new(1, 0, 0, 0, 0),
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
                TicketInventory::new(1, 1, 0, 0, 0), // taxi + bus
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
                TicketInventory::new(1, 0, 0, 0, 0), // taxi only
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
                TicketInventory::new(1, 1, 0, 0, 0),
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
                TicketInventory::new(1, 1, 0, 0, 0),
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
                TicketInventory::new(2, 0, 0, 0, 0),
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
                TicketInventory::new(1, 0, 0, 0, 0),
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
                TicketInventory::new(1, 0, 0, 0, 0),
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
                TicketInventory::new(2, 0, 0, 0, 0),
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
                TicketInventory::new(1, 0, 0, 0, 0),
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

        #[test]
        fn apply_step_detective_ticket_transferred_to_mrx() {
            // Detective at station 3 moves to station 2 (bus); MrX at station 1.
            // tiny_board: 3 <-bus-> 2 <-taxi-> 1
            let board = Arc::new(tiny_board());
            let players = vec![
                PlayerState::new(
                    PlayerId::MrX,
                    StationId { id: 1 },
                    TicketInventory::new(0, 0, 0, 0, 0),
                ),
                PlayerState::new(
                    PlayerId::Detective(1),
                    StationId { id: 3 },
                    TicketInventory::new(0, 1, 0, 0, 0),
                ),
            ];
            let mut state = GameState::new(board, players);
            state.current_player = 1;

            let step = Step { to: StationId { id: 2 }, ticket: TicketType::Bus };
            let new_state = apply_step(&state, step);

            assert_eq!(*new_state.players[1].tickets.get(TicketType::Bus), 0);
            assert_eq!(*new_state.players[0].tickets.get(TicketType::Bus), 1);
        }

        #[test]
        fn apply_step_mrx_ticket_not_transferred() {
            // MrX moves; his spent ticket should not be added back to anyone.
            let state = make_state(
                StationId { id: 1 },
                TicketInventory::new(2, 0, 0, 0, 0),
                StationId { id: 3 },
            );

            let step = Step { to: StationId { id: 2 }, ticket: TicketType::Taxi };
            let new_state = apply_step(&state, step);

            assert_eq!(*new_state.players[0].tickets.get(TicketType::Taxi), 1);
            assert_eq!(*new_state.players[1].tickets.get(TicketType::Taxi), 1);
        }
    }

    mod apply_action_tests {
        
        use super::*;

        #[test]
        fn apply_action_single_matches_apply_step() {
            let state = make_state(
                StationId { id: 1 },
                TicketInventory::new(1, 0, 0, 0, 0),
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
                TicketInventory::new(2, 0, 0, 0, 0),
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
                TicketInventory::new(2, 0, 0, 0, 0),
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

        #[test]
        fn apply_action_advances_current_player() {
            // MrX is player 0; after his action current_player should be 1.
            let state = make_state(
                StationId { id: 1 },
                TicketInventory::new(1, 0, 0, 0, 0),
                StationId { id: 3 },
            );
            assert_eq!(state.current_player, 0);

            let new_state = apply_action(&state, Action::Single(Step {
                to: StationId { id: 2 },
                ticket: TicketType::Taxi,
            }));

            assert_eq!(new_state.current_player, 1);
        }

        #[test]
        fn apply_action_wraps_current_player_to_zero() {
            // Detective is the last player; after his action current_player wraps to 0.
            let mut state = make_state(
                StationId { id: 1 },
                TicketInventory::new(1, 0, 0, 0, 0),
                StationId { id: 3 },
            );
            state.current_player = 1;

            let new_state = apply_action(&state, Action::Single(Step {
                to: StationId { id: 2 },
                ticket: TicketType::Taxi,
            }));

            assert_eq!(new_state.current_player, 0);
        }

        #[test]
        fn apply_action_increments_turn_number_on_wrap() {
            // Wrapping back to player 0 should increment turn_number.
            let mut state = make_state(
                StationId { id: 1 },
                TicketInventory::new(1, 0, 0, 0, 0),
                StationId { id: 3 },
            );
            state.current_player = 1;
            assert_eq!(state.turn_number, 0);

            let new_state = apply_action(&state, Action::Single(Step {
                to: StationId { id: 2 },
                ticket: TicketType::Taxi,
            }));

            assert_eq!(new_state.turn_number, 1);
        }

        #[test]
        fn apply_action_does_not_increment_turn_number_mid_round() {
            // Advancing from player 0 to player 1 should not increment turn_number.
            let state = make_state(
                StationId { id: 1 },
                TicketInventory::new(1, 0, 0, 0, 0),
                StationId { id: 3 },
            );
            assert_eq!(state.turn_number, 0);

            let new_state = apply_action(&state, Action::Single(Step {
                to: StationId { id: 2 },
                ticket: TicketType::Taxi,
            }));

            assert_eq!(new_state.turn_number, 0);
        }

        #[test]
        fn apply_action_double_advances_current_player_once() {
            // A double move involves two apply_step calls internally but must only
            // advance current_player once.
            let state = make_chain_state(TicketInventory::new(2, 0, 0, 0, 0));
            assert_eq!(state.current_player, 0);

            let new_state = apply_action(&state, Action::Double(
                Step { to: StationId { id: 2 }, ticket: TicketType::Taxi },
                Step { to: StationId { id: 3 }, ticket: TicketType::Taxi },
            ));

            assert_eq!(new_state.current_player, 1);
        }

        // Step 4: detective catches MrX
        #[test]
        fn apply_action_detective_catches_mrx_sets_terminal() {
            // tiny_board: 1 <-taxi-> 2 <-bus-> 3
            // MrX at 2, detective at 1; detective taxis to 2 and catches MrX.
            let board = Arc::new(tiny_board());
            let players = vec![
                PlayerState::new(PlayerId::MrX, StationId { id: 2 }, TicketInventory::new(0, 0, 0, 0, 0)),
                PlayerState::new(PlayerId::Detective(1), StationId { id: 1 }, TicketInventory::new(1, 0, 0, 0, 0)),
            ];
            let mut state = GameState::new(board, players);
            state.current_player = 1;

            let new_state = apply_action(&state, Action::Single(
                Step { to: StationId { id: 2 }, ticket: TicketType::Taxi },
            ));

            assert!(new_state.is_terminal);
            assert_eq!(new_state.winner, Some(PlayerId::Detectives));
        }

        #[test]
        fn apply_action_non_catching_move_not_terminal() {
            // MrX moves away from the detective; nobody shares a station so the
            // game must not be marked terminal.
            // tiny_board: 1 <-taxi-> 2 <-bus-> 3. Detective is at 3 (far away).
            let state = make_state(
                StationId { id: 1 },
                TicketInventory::new(1, 0, 0, 0, 0),
                StationId { id: 3 },
            );

            let new_state = apply_action(&state, Action::Single(
                Step { to: StationId { id: 2 }, ticket: TicketType::Taxi },
            ));

            assert!(!new_state.is_terminal);
            assert_eq!(new_state.winner, None);
        }

        // Step 5: MrX cornered
        #[test]
        fn apply_action_mrx_cornered_detectives_win() {
            // MrX at station 1 with no tickets; after the detective completes
            // the round, MrX has no legal moves and detectives win.
            // Station 1 only connects via taxi, which MrX does not have.
            // The detective bus ticket transferred to MrX on move is unusable
            // from station 1 (no bus edges there).
            let board = Arc::new(tiny_board());
            let players = vec![
                PlayerState::new(PlayerId::MrX, StationId { id: 1 }, TicketInventory::new(0, 0, 0, 0, 0)),
                PlayerState::new(PlayerId::Detective(1), StationId { id: 3 }, TicketInventory::new(0, 1, 0, 0, 0)),
            ];
            let mut state = GameState::new(board, players);
            state.current_player = 1;

            let new_state = apply_action(&state, Action::Single(
                Step { to: StationId { id: 2 }, ticket: TicketType::Bus },
            ));

            assert!(new_state.is_terminal);
            assert_eq!(new_state.winner, Some(PlayerId::Detectives));
        }

        // Step 6: turn limit
        #[test]
        fn apply_action_turn_limit_mrx_wins() {
            // With max_turns = 1, completing the first round makes MrX the winner.
            // Detective needs a bus ticket to move from 3 to 2 on tiny_board.
            let board = Arc::new(tiny_board());
            let players = vec![
                PlayerState::new(PlayerId::MrX, StationId { id: 1 }, TicketInventory::new(1, 0, 0, 0, 0)),
                PlayerState::new(PlayerId::Detective(1), StationId { id: 3 }, TicketInventory::new(0, 1, 0, 0, 0)),
            ];
            let mut state = GameState::new(board, players);
            state.max_turns = 1;
            state.current_player = 1; // jump to the detective's move to complete turn 0

            let new_state = apply_action(&state, Action::Single(
                Step { to: StationId { id: 2 }, ticket: TicketType::Bus },
            ));

            // turn_number is now 1 >= max_turns 1, MrX wins
            assert!(new_state.is_terminal);
            assert_eq!(new_state.winner, Some(PlayerId::MrX));
        }

        #[test]
        fn apply_action_mid_round_does_not_trigger_turn_limit() {
            // Even if turn_number would equal max_turns after a mid-round advance,
            // the check only fires when the round fully completes (next_player == 0).
            let mut state = make_state(
                StationId { id: 1 },
                TicketInventory::new(1, 0, 0, 0, 0),
                StationId { id: 3 },
            );
            state.max_turns = 1;
            // current_player is 0 (MrX); his move advances to player 1, not 0.

            let new_state = apply_action(&state, Action::Single(
                Step { to: StationId { id: 2 }, ticket: TicketType::Taxi },
            ));

            assert!(!new_state.is_terminal);
            assert_eq!(new_state.winner, None);
        }
    }

    mod is_action_legal_tests {
        
        use super::*;

        #[test]
        fn is_action_legal_terminal_state_rejects_all_actions() {
            let mut state = make_state(
                StationId { id: 1 },
                TicketInventory::new(1, 0, 0, 0, 0),
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
                TicketInventory::new(1, 0, 0, 0, 0),
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
                TicketInventory::new(2, 0, 0, 0, 0),
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
                TicketInventory::new(1, 0, 0, 0, 0),
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
                TicketInventory::new(2, 0, 0, 0, 0),
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
                TicketInventory::new(2, 0, 0, 0, 0),
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
                TicketInventory::new(2, 0, 0, 0, 0),
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

        #[test]
        fn is_action_legal_detectives_player_id_always_false() {
            let board = Arc::new(tiny_board());
            let mut state = GameState::new(
                board,
                vec![
                    PlayerState::new(
                        PlayerId::Detectives,
                        StationId { id: 1 },
                        TicketInventory::new(1, 0, 0, 0, 0),
                    ),
                ],
            );
            state.current_player = 0;

            let single = Action::Single(Step {
                to: StationId { id: 2 },
                ticket: TicketType::Taxi,
            });
            let double = Action::Double(
                Step { to: StationId { id: 2 }, ticket: TicketType::Taxi },
                Step { to: StationId { id: 1 }, ticket: TicketType::Taxi },
            );

            assert!(!is_action_legal(&state, single));
            assert!(!is_action_legal(&state, double));
        }

    }

    mod legality_invariant_tests {
        use super::*;

        #[test]
        fn legal_actions_are_all_legal() {
            let state = make_chain_state(
                TicketInventory::new(2, 0, 0, 0, 0),
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
                TicketInventory::new(1, 1, 0, 0, 0),
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