use super::gamestate::GameState;
use super::gamestate::PlayerState;
use super::gamestate::PlayerId;

use super::gamestate::Step;
use super::gamestate::Action;
use super::gamestate::Winner;

use super::board::TicketType;
use super::board::StationId;

fn legal_steps(gamestate: &GameState) -> Vec<Step> {
    let curr_player = &gamestate.players[gamestate.current_player];
    let mut steps = Vec::new();

    // Only MrX ever holds black tickets, so possession alone gates concealment.
    let has_black = *curr_player.tickets.get(TicketType::Black) > 0;

    // A black step conceals the destination, not the route, so multiple parallel
    // edges to the same station (e.g. taxi + bus) must collapse to a single black
    // step. Track destinations that already have one to avoid duplicates — both
    // from native ferry edges and from concealment alternatives.
    let mut black_dests: Vec<StationId> = Vec::new();

    for &(dest, tt) in gamestate.board.neighbors(curr_player.station) {
        let is_black_edge = tt == TicketType::Black;

        // Move paid with the edge's native transport (for ferry edges this is
        // itself a black ticket). Skip a native black step if another parallel
        // edge already produced one to this destination.
        let native = Step { to: dest, ticket: tt };
        if !(is_black_edge && black_dests.contains(&dest)) && is_step_legal(gamestate, native) {
            steps.push(native);
            if is_black_edge {
                black_dests.push(dest);
            }
        }

        // Concealment alternative: a black ticket may be spent on any edge to
        // hide which transport was used. Emit at most one per destination.
        if has_black && !is_black_edge && !black_dests.contains(&dest) {
            let concealed = Step { to: dest, ticket: TicketType::Black };
            if is_step_legal(gamestate, concealed) {
                steps.push(concealed);
                black_dests.push(dest);
            }
        }
    }

    steps
}

pub fn legal_actions(gamestate: &GameState) -> Vec<Action> {
    // A finished game has no moves. Without this, a terminal state whose current
    // detective is stuck would still emit a Pass — a generated action the
    // validator rejects, breaking the generator ⊆ validator invariant.
    if gamestate.is_terminal {
        return Vec::new();
    }

    let single_steps = legal_steps(gamestate);
    let curr_player = &gamestate.players[gamestate.current_player];

    let mut actions: Vec<Action> = single_steps
    .iter()
    .copied()
    .map(Action::Single)
    .collect();

    // Double moves are MrX-only and consume one of his finite double tickets, so
    // they are only generated when he actually holds one.
    if matches!(curr_player.id, PlayerId::MrX) && curr_player.tickets.double > 0 {
        for first_step in &single_steps {
            let intermediate = apply_step(gamestate, *first_step);

            for second_step in legal_steps(&intermediate) {
                actions.push(Action::Double(*first_step, second_step));
            }
        }
    }

    // Pass rule: a detective with no move available forfeits their turn so the
    // game can continue. This guarantees every non-terminal state has at least
    // one legal action. MrX is excluded — a cornered MrX is a loss, not a pass.
    if actions.is_empty() && matches!(curr_player.id, PlayerId::Detective(_)) {
        actions.push(Action::Pass);
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

/// Determines a state's terminal status from its current position, independent
/// of how the state was reached. Used both after a transition in apply_action
/// and at construction in GameState::new, so the invariant "every non-terminal
/// state has >= 1 legal action" holds for freshly built states too — important
/// for the determinizer, which constructs states wholesale.
///
/// Priority matches apply_action's original ordering: a capture outranks the
/// turn limit (a detective landing on MrX on the final move still wins), which
/// outranks MrX being cornered.
pub fn detect_terminal(gamestate: &GameState) -> Option<Winner> {
    let mrx_station = gamestate.players.iter()
        .find(|p| matches!(p.id, PlayerId::MrX))
        .map(|p| p.station)
        .expect("no MrX in players");

    // Caught: a detective stands on MrX's station.
    if gamestate.players.iter().any(|p| {
        matches!(p.id, PlayerId::Detective(_)) && p.station == mrx_station
    }) {
        return Some(Winner::Detectives);
    }

    // Turn limit reached: MrX evaded capture for the full game.
    if gamestate.turn_number >= gamestate.max_turns {
        return Some(Winner::MrX);
    }

    // Cornered: it is MrX's turn and he has no legal move. A stuck detective
    // passes instead, so only MrX can be actionless in a live game — which is
    // exactly the zero-action hole this check closes at construction.
    let curr_player = &gamestate.players[gamestate.current_player];
    if matches!(curr_player.id, PlayerId::MrX) && legal_actions(gamestate).is_empty() {
        return Some(Winner::Detectives);
    }

    None
}

pub fn apply_action(gamestate: &GameState, action: Action) -> GameState {
    // Applying an action past the end of the game is a no-op. Without this guard,
    // turn rotation and the terminal checks keep running and can rewrite a settled
    // winner (e.g. a post-terminal detective move onto MrX flipping an MrX
    // turn-limit win into a Detectives win).
    if gamestate.is_terminal {
        return gamestate.clone();
    }

    let mut new_state = match action {
        Action::Single(s) => apply_step(gamestate, s),
        Action::Double(s1, s2) => {
            let intermediate = apply_step(gamestate, s1);
            let mut after = apply_step(&intermediate, s2);
            // The two apply_step calls spend the transport tickets; the double
            // move itself costs one double ticket, charged once per action.
            after.players[gamestate.current_player].tickets.spend_double();
            after
        }
        // A pass leaves position and tickets untouched; the turn rotation below
        // is what makes play continue past a stuck detective.
        Action::Pass => gamestate.clone(),
    };

    let next_player = (new_state.current_player + 1) % new_state.players.len();
    new_state.current_player = next_player;
    if next_player == 0 {
        new_state.turn_number += 1;
    }

    // Capture is checked after every move; the turn-limit and cornered rules in
    // detect_terminal only fire when MrX is up next (turn_number advances only on
    // the round-completing wrap, and the cornered branch is MrX-gated), so this
    // single call preserves the original round-boundary semantics.
    if let Some(winner) = detect_terminal(&new_state) {
        new_state.is_terminal = true;
        new_state.winner = Some(winner);
    }

    new_state
}

fn is_step_legal(gamestate: &GameState, step: Step) -> bool {
    let curr_player = &gamestate.players[gamestate.current_player];

    *curr_player.tickets.get(step.ticket) > 0 // current player has the required ticket
        && gamestate.board.neighbors(curr_player.station)
            .iter()
            .any(|&(dest, edge_ticket)| {
                // The edge must reach the target, and the ticket must be able to
                // traverse it: either it matches the edge's transport, or it is a
                // black ticket, which may be spent on any edge.
                dest == step.to
                    && (step.ticket == TicketType::Black || edge_ticket == step.ticket)
            })
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
                Action::Double(_, _) => false,
                // A pass is legal only when the detective is genuinely stuck.
                Action::Pass => legal_steps(gamestate).is_empty(),
            }
        }
        PlayerId::MrX => {
            match action {
                Action::Single(s) => is_step_legal(gamestate, s),
                Action::Double(s1,s2 ) => {
                    // A double move requires a spare double ticket and both steps
                    // legal, the second from the intermediate position.
                    gamestate.players[gamestate.current_player].tickets.double > 0
                    && is_step_legal(gamestate, s1) && {
                    let intermediate = apply_step(gamestate, s1);
                    is_step_legal(&intermediate, s2)
}
                }
                Action::Pass => false,
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
                // Holds a double ticket so the "no double" result is genuinely due
                // to the dead end, not a missing double ticket.
                TicketInventory::new(2, 0, 0, 0, 1),
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
            // Detective at station 3 can only leave by bus; grant the ticket so a
            // real move exists and this assertion is not satisfied vacuously.
            state.players[1].tickets.add_ticket(TicketType::Bus);

            let actions = legal_actions(&state);

            assert!(!actions.is_empty());
            for action in actions {
                // Detectives never receive a Double; with a move available they
                // never receive a Pass either.
                assert!(matches!(action, Action::Single(_)));
            }
        }

        #[test]
        fn legal_actions_mrx_gets_double_actions() {
            let state = make_chain_state(
                TicketInventory::new(2, 0, 0, 0, 1),
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
            // Has a double ticket but only one transport ticket, so the absence of
            // a double action is due to the transport shortage, not the gate.
            let state = make_chain_state(
                TicketInventory::new(1, 0, 0, 0, 1),
            );

            let actions = legal_actions(&state);

            assert!(
                !actions.iter().any(|a| matches!(a, Action::Double(_, _)))
            );
        }

        #[test]
        fn legal_actions_no_double_without_double_ticket() {
            // Two transport tickets but zero double tickets: a double is reachable
            // by geometry yet must not be generated, since it can't be paid for.
            let state = make_chain_state(
                TicketInventory::new(2, 0, 0, 0, 0),
            );

            let actions = legal_actions(&state);

            assert!(actions.iter().any(|a| matches!(a, Action::Single(_))));
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
                TicketInventory::new(2, 0, 0, 0, 1),
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
                TicketInventory::new(2, 0, 0, 0, 1),
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
        fn apply_action_double_spends_a_double_ticket() {
            // Two double tickets in; one spent by the double move, one remaining.
            let state = make_chain_state(
                TicketInventory::new(2, 0, 0, 0, 2),
            );

            let action = Action::Double(
                Step { to: StationId { id: 2 }, ticket: TicketType::Taxi },
                Step { to: StationId { id: 3 }, ticket: TicketType::Taxi },
            );

            let new_state = apply_action(&state, action);

            assert_eq!(new_state.players[0].tickets.double, 1);
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
            let state = make_chain_state(TicketInventory::new(2, 0, 0, 0, 1));
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
            assert_eq!(new_state.winner, Some(Winner::Detectives));
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
            assert_eq!(new_state.winner, Some(Winner::Detectives));
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
            assert_eq!(new_state.winner, Some(Winner::MrX));
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
            // MrX at 1 can taxi to the empty station 2, so the constructed state
            // is not terminal (MrX is neither caught nor cornered). The detective
            // at 3 then buses to the empty station 2.
            let mut state = make_state(
                StationId { id: 1 }, // Mr X
                TicketInventory::new(1, 0, 0, 0, 0),
                StationId { id: 3 }, // Detective
            );

            state.current_player = 1;
            state.players[1].tickets.add_ticket(TicketType::Bus);

            let action = Action::Single(Step {
                to: StationId { id: 2 },
                ticket: TicketType::Bus,
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
                TicketInventory::new(2, 0, 0, 0, 1),
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
        fn is_action_legal_mrx_double_rejected_without_double_ticket() {
            // A geometrically valid double that the validator must still reject
            // because MrX holds no double ticket. The existing generator/validator
            // invariant would NOT catch a too-permissive validator here, so this
            // boundary is asserted directly.
            let state = make_chain_state(
                TicketInventory::new(2, 0, 0, 0, 0),
            );

            let action = Action::Double(
                Step { to: StationId { id: 2 }, ticket: TicketType::Taxi },
                Step { to: StationId { id: 3 }, ticket: TicketType::Taxi },
            );

            assert!(!is_action_legal(&state, action));
        }

        #[test]
        fn is_action_legal_mrx_double_illegal_first_step() {
            // Double ticket present so the rejection is caused by the illegal first
            // step alone, not the gate.
            let state = make_chain_state(
                TicketInventory::new(2, 0, 0, 0, 1),
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
            // Double ticket present so the rejection is caused by the illegal second
            // step alone, not the gate.
            let state = make_chain_state(
                TicketInventory::new(2, 0, 0, 0, 1),
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

    mod black_ticket_tests {
        use super::*;

        // Station 1: taxi edge to 2, ferry (black) edge to 3.
        // Station 2: taxi edge back to 1.
        // Station 3: ferry (black) edge back to 1.
        // Station 4: taxi edge to 2 — an isolated parking spot for the detective
        //            so it never blocks the moves under test.
        fn black_board() -> Board {
            Board {
                adjacency_map: vec![
                    vec![
                        (StationId { id: 2 }, TicketType::Taxi),
                        (StationId { id: 3 }, TicketType::Black),
                    ],
                    vec![(StationId { id: 1 }, TicketType::Taxi)],
                    vec![(StationId { id: 1 }, TicketType::Black)],
                    vec![(StationId { id: 2 }, TicketType::Taxi)],
                ],
            }
        }

        fn mrx_state(mrx_station: StationId, mrx_tickets: TicketInventory) -> GameState {
            let board = Arc::new(black_board());
            let players = vec![
                PlayerState::new(PlayerId::MrX, mrx_station, mrx_tickets),
                PlayerState::new(
                    PlayerId::Detective(1),
                    StationId { id: 4 },
                    TicketInventory::new(1, 0, 0, 0, 0),
                ),
            ];
            GameState::new(board, players)
        }

        #[test]
        fn mrx_with_black_gets_concealment_alternative() {
            // MrX at station 1 holding a taxi and a black ticket should be able to
            // reach station 2 with EITHER ticket, and reach the ferry station 3.
            let state = mrx_state(
                StationId { id: 1 },
                TicketInventory::new(1, 0, 0, 1, 0),
            );

            let steps = legal_steps(&state);

            assert!(steps.contains(&Step { to: StationId { id: 2 }, ticket: TicketType::Taxi }));
            assert!(steps.contains(&Step { to: StationId { id: 2 }, ticket: TicketType::Black }));
            assert!(steps.contains(&Step { to: StationId { id: 3 }, ticket: TicketType::Black }));
            assert_eq!(steps.len(), 3);
        }

        #[test]
        fn ferry_edge_does_not_produce_a_duplicate_black_step() {
            // The ferry edge to 3 is already a black edge; it must yield exactly one
            // black step, not a native + concealment pair.
            let state = mrx_state(
                StationId { id: 1 },
                TicketInventory::new(1, 0, 0, 1, 0),
            );

            let steps = legal_steps(&state);
            let black_to_3 = steps
                .iter()
                .filter(|s| **s == Step { to: StationId { id: 3 }, ticket: TicketType::Black })
                .count();

            assert_eq!(black_to_3, 1);
        }

        #[test]
        fn mrx_without_black_has_no_concealment_and_cannot_ferry() {
            // Without a black ticket: no concealment alternative on the taxi edge,
            // and the ferry to station 3 is unreachable.
            let state = mrx_state(
                StationId { id: 1 },
                TicketInventory::new(1, 0, 0, 0, 0),
            );

            let steps = legal_steps(&state);

            assert_eq!(steps, vec![Step { to: StationId { id: 2 }, ticket: TicketType::Taxi }]);
        }

        #[test]
        fn detective_cannot_use_ferry() {
            // A detective stranded on a ferry-only station cannot cross the river,
            // even flush with taxi/bus/underground tickets, because it has no black.
            let board = Arc::new(black_board());
            let players = vec![
                PlayerState::new(
                    PlayerId::MrX,
                    StationId { id: 1 },
                    TicketInventory::new(1, 0, 0, 1, 0),
                ),
                PlayerState::new(
                    PlayerId::Detective(1),
                    StationId { id: 3 },
                    TicketInventory::new(5, 5, 5, 0, 0),
                ),
            ];
            let mut state = GameState::new(board, players);
            state.current_player = 1;

            assert!(legal_steps(&state).is_empty());
        }

        #[test]
        fn is_step_legal_rejects_ticket_not_supported_by_edge() {
            // Station 1 reaches 2 only by taxi. A bus step to 2 must be rejected even
            // though the player holds a bus ticket. (Guards the edge-matching fix.)
            let state = mrx_state(
                StationId { id: 1 },
                TicketInventory::new(0, 1, 0, 0, 0),
            );

            let step = Step { to: StationId { id: 2 }, ticket: TicketType::Bus };
            assert!(!is_step_legal(&state, step));
        }

        #[test]
        fn is_step_legal_accepts_black_on_a_non_ferry_edge() {
            let state = mrx_state(
                StationId { id: 1 },
                TicketInventory::new(0, 0, 0, 1, 0),
            );

            let step = Step { to: StationId { id: 2 }, ticket: TicketType::Black };
            assert!(is_step_legal(&state, step));
        }

        #[test]
        fn apply_step_black_ticket_spent_and_not_transferred() {
            // MrX spending a black ticket loses it; it is discarded, not handed off.
            let state = mrx_state(
                StationId { id: 1 },
                TicketInventory::new(0, 0, 0, 1, 0),
            );

            let next = apply_step(&state, Step { to: StationId { id: 2 }, ticket: TicketType::Black });

            assert_eq!(next.players[0].station.id, 2);
            assert_eq!(*next.players[0].tickets.get(TicketType::Black), 0);
            assert_eq!(*next.players[1].tickets.get(TicketType::Black), 0);
        }

        #[test]
        fn parallel_edges_yield_one_black_step_per_destination() {
            // Station 1 reaches station 2 by BOTH taxi and bus (parallel edges).
            // MrX holding a black ticket must get each native step once and a
            // single black concealment step to 2 — not one per parallel edge.
            let board = Arc::new(Board {
                adjacency_map: vec![
                    vec![
                        (StationId { id: 2 }, TicketType::Taxi),
                        (StationId { id: 2 }, TicketType::Bus),
                    ],
                    vec![(StationId { id: 1 }, TicketType::Taxi)],
                ],
            });
            let players = vec![
                PlayerState::new(
                    PlayerId::MrX,
                    StationId { id: 1 },
                    TicketInventory::new(1, 1, 0, 1, 0),
                ),
                PlayerState::new(
                    PlayerId::Detective(1),
                    StationId { id: 1 }, // off-board parking irrelevant; placed to avoid blocking
                    TicketInventory::new(1, 0, 0, 0, 0),
                ),
            ];
            // Park the detective somewhere that does not occupy station 2.
            let mut state = GameState::new(board, players);
            state.players[1].station = StationId { id: 1 };

            let steps = legal_steps(&state);
            let black_to_2 = steps
                .iter()
                .filter(|s| **s == Step { to: StationId { id: 2 }, ticket: TicketType::Black })
                .count();

            assert_eq!(black_to_2, 1, "parallel edges must not duplicate the black step");
            // The two distinct native steps and one black step.
            assert!(steps.contains(&Step { to: StationId { id: 2 }, ticket: TicketType::Taxi }));
            assert!(steps.contains(&Step { to: StationId { id: 2 }, ticket: TicketType::Bus }));
            assert_eq!(steps.len(), 3);
        }

        #[test]
        fn black_generated_actions_are_all_legal() {
            // The generator/validator agreement must hold on the black action space
            // too, including the concealment alternatives.
            let state = mrx_state(
                StationId { id: 1 },
                TicketInventory::new(1, 0, 0, 1, 0),
            );

            for action in legal_actions(&state) {
                assert!(
                    is_action_legal(&state, action),
                    "generated action was not legal: {:?}",
                    action
                );
            }
        }
    }

    mod pass_rule_tests {
        use super::*;

        // tiny_board: 1 <-taxi-> 2 <-bus-> 3.
        // A detective stranded at station 1 with no taxi ticket has no move.
        // MrX sits at 3 with a bus ticket so he is neither caught nor cornered,
        // keeping the game alive for the assertions below.
        fn stuck_detective_state() -> GameState {
            let board = Arc::new(tiny_board());
            let players = vec![
                PlayerState::new(
                    PlayerId::MrX,
                    StationId { id: 3 },
                    TicketInventory::new(0, 1, 0, 0, 0),
                ),
                PlayerState::new(
                    PlayerId::Detective(1),
                    StationId { id: 1 },
                    TicketInventory::new(0, 0, 0, 0, 0),
                ),
            ];
            let mut state = GameState::new(board, players);
            state.current_player = 1; // detective to act
            state
        }

        #[test]
        fn stuck_detective_has_exactly_one_action_a_pass() {
            let state = stuck_detective_state();
            let actions = legal_actions(&state);
            assert_eq!(actions, vec![Action::Pass]);
        }

        #[test]
        fn non_terminal_state_always_has_an_action() {
            // The core soundness invariant the pass rule guarantees: a stuck
            // detective in a live game still yields a non-empty action list.
            let state = stuck_detective_state();
            assert!(!state.is_terminal);
            assert!(!legal_actions(&state).is_empty());
        }

        #[test]
        fn applying_pass_advances_turn_without_changing_player() {
            let state = stuck_detective_state();
            let before = state.players[1];

            let next = apply_action(&state, Action::Pass);

            // detective's position and tickets are untouched
            assert_eq!(next.players[1], before);
            // turn handed to MrX, round counter advanced
            assert_eq!(next.current_player, 0);
            assert_eq!(next.turn_number, 1);
            // game continues
            assert!(!next.is_terminal);
            assert_eq!(next.winner, None);
        }

        #[test]
        fn detective_with_a_move_cannot_pass() {
            // Give the detective a taxi ticket so a real move exists; Pass must
            // then be both absent from legal_actions and rejected by the validator.
            let mut state = stuck_detective_state();
            state.players[1].tickets.add_ticket(TicketType::Taxi);

            let actions = legal_actions(&state);
            assert!(!actions.contains(&Action::Pass));
            assert!(actions.iter().any(|a| matches!(a, Action::Single(_))));
            assert!(!is_action_legal(&state, Action::Pass));
        }

        #[test]
        fn stuck_mrx_does_not_pass() {
            // MrX at station 1 with no taxi ticket is cornered, not passing:
            // legal_actions is empty and Pass is never legal for MrX. This is
            // what lets the cornered terminal check fire instead of stalling.
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
                    TicketInventory::new(0, 0, 0, 0, 0),
                ),
            ];
            let state = GameState::new(board, players); // current_player = 0 (MrX)

            assert!(legal_actions(&state).is_empty());
            assert!(!is_action_legal(&state, Action::Pass));
        }
    }

    mod terminal_guard_tests {
        use super::*;

        #[test]
        fn legal_actions_empty_on_terminal_state() {
            // A stuck detective in a live game gets [Pass]; once the state is
            // terminal the generator must return nothing instead, so it never
            // emits an action the validator would reject.
            let board = Arc::new(tiny_board());
            let players = vec![
                PlayerState::new(PlayerId::MrX, StationId { id: 3 }, TicketInventory::new(0, 1, 0, 0, 0)),
                PlayerState::new(PlayerId::Detective(1), StationId { id: 1 }, TicketInventory::new(0, 0, 0, 0, 0)),
            ];
            let mut state = GameState::new(board, players);
            state.current_player = 1; // stuck detective to act

            // Sanity: live, the stuck detective passes.
            assert_eq!(legal_actions(&state), vec![Action::Pass]);

            state.is_terminal = true;
            assert!(legal_actions(&state).is_empty());
        }

        #[test]
        fn generator_subset_validator_on_terminal_state() {
            // The generator ⊆ validator invariant must hold on terminal states:
            // both sides agree there are no legal actions.
            let board = Arc::new(tiny_board());
            let players = vec![
                PlayerState::new(PlayerId::MrX, StationId { id: 3 }, TicketInventory::new(0, 1, 0, 0, 0)),
                PlayerState::new(PlayerId::Detective(1), StationId { id: 1 }, TicketInventory::new(0, 0, 0, 0, 0)),
            ];
            let mut state = GameState::new(board, players);
            state.current_player = 1;
            state.is_terminal = true;

            for action in legal_actions(&state) {
                assert!(is_action_legal(&state, action));
            }
            // And Pass — which the live state would have generated — is rejected.
            assert!(!is_action_legal(&state, Action::Pass));
        }

        #[test]
        fn apply_action_on_terminal_state_is_a_noop() {
            // Applying past the end must not rotate turns, advance the round, or
            // change the winner.
            let board = Arc::new(tiny_board());
            let players = vec![
                PlayerState::new(PlayerId::MrX, StationId { id: 1 }, TicketInventory::new(1, 0, 0, 0, 0)),
                PlayerState::new(PlayerId::Detective(1), StationId { id: 3 }, TicketInventory::new(1, 0, 0, 0, 0)),
            ];
            let mut state = GameState::new(board, players);
            state.is_terminal = true;
            state.winner = Some(Winner::MrX);
            state.current_player = 1;
            state.turn_number = 5;

            let next = apply_action(&state, Action::Single(
                Step { to: StationId { id: 2 }, ticket: TicketType::Taxi },
            ));

            assert_eq!(next.current_player, 1); // turn not rotated
            assert_eq!(next.turn_number, 5);    // round not advanced
            assert_eq!(next.winner, Some(Winner::MrX));
            assert_eq!(next.players, state.players);
        }

        #[test]
        fn post_terminal_capture_cannot_flip_winner() {
            // MrX has won by turn limit. A detective then "moves" onto MrX's
            // station; the guard must keep the MrX win rather than flipping it to
            // a Detectives capture win.
            // tiny_board: 1 <-taxi-> 2. MrX at 2, detective at 1 with a taxi.
            let board = Arc::new(tiny_board());
            let players = vec![
                PlayerState::new(PlayerId::MrX, StationId { id: 2 }, TicketInventory::new(0, 0, 0, 0, 0)),
                PlayerState::new(PlayerId::Detective(1), StationId { id: 1 }, TicketInventory::new(1, 0, 0, 0, 0)),
            ];
            let mut state = GameState::new(board, players);
            state.is_terminal = true;
            state.winner = Some(Winner::MrX);
            state.current_player = 1;

            let next = apply_action(&state, Action::Single(
                Step { to: StationId { id: 2 }, ticket: TicketType::Taxi },
            ));

            assert_eq!(next.winner, Some(Winner::MrX));
        }
    }

    mod legality_invariant_tests {
        use super::*;

        #[test]
        fn legal_actions_are_all_legal() {
            // Includes a double ticket so the invariant also covers the generated
            // double actions, not just singles.
            let state = make_chain_state(
                TicketInventory::new(2, 0, 0, 0, 1),
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