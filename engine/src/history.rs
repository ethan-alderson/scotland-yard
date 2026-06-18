use super::board::{StationId, TicketType};
use super::gamestate::{Action, GameState, PlayerId};
use super::rules::apply_action;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct MrXMove {
    pub ticket: TicketType,       
    pub revealed: Option<StationId>, // revealed only on reveal legs
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GameHistory {
    pub mr_x_log: Vec<MrXMove>,        // one entry per leg Mr X has played
}

/// Reveal legs are configuration, not magic constants buried in logic. Leg
/// numbers are 1-based counts of Mr X's legs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevealSchedule {
    legs: Vec<usize>,
}

/// Standard Scotland Yard reveal legs.
pub const STANDARD_REVEALS: [usize; 5] = [3, 8, 13, 18, 24];

impl RevealSchedule {
    pub fn new(legs: Vec<usize>) -> Self {
        Self { legs }
    }

    pub fn standard() -> Self {
        Self { legs: STANDARD_REVEALS.to_vec() }
    }

    pub fn is_reveal_leg(&self, leg: usize) -> bool {
        self.legs.contains(&leg)
    }
}

/// Top-level game wrapper: ground truth plus the public information layer.
///
/// `apply` advances ground truth via the pure engine and, for Mr X's moves only,
/// appends to the public log. The authoritative log only advances on real moves,
/// which all go through here — `apply_action` stays history-free.
pub struct Game {
    pub state: GameState,
    pub history: GameHistory,
    pub reveal: RevealSchedule,
}

impl Game {
    pub fn new(state: GameState, reveal: RevealSchedule) -> Self {
        Self { state, history: GameHistory::default(), reveal }
    }

    /// Apply a move: record Mr X's legs into the public log, then advance ground
    /// truth. The record is taken before `apply_action`, since it reads the
    /// acting player off the current state.
    pub fn apply(&mut self, action: Action) {
        if matches!(self.state.players[self.state.current_player].id, PlayerId::MrX) {
            self.record_mr_x(action);
        }
        self.state = apply_action(&self.state, action);
    }

    fn record_mr_x(&mut self, action: Action) {
        match action {
            Action::Single(step) => self.push_leg(step.ticket, step.to),
            Action::Double(s1, s2) => {
                // A double is two legs; each is logged separately, and a reveal
                // can land on the intermediate leg.
                self.push_leg(s1.ticket, s1.to);
                self.push_leg(s2.ticket, s2.to);
            }
            // Mr X never passes; nothing to record.
            Action::Pass => {}
        }
    }

    /// Append one leg. `station` is Mr X's ground-truth position *after* this leg,
    /// exposed only when this leg is a reveal leg. `ticket` is the public spent
    /// ticket (already `Black` when concealed).
    fn push_leg(&mut self, ticket: TicketType, station: StationId) {
        // Leg numbers are 1-based; this leg is the (len + 1)-th Mr X has played.
        let leg_number = self.history.mr_x_log.len() + 1;
        let revealed = if self.reveal.is_reveal_leg(leg_number) {
            Some(station)
        } else {
            None
        };
        self.history.mr_x_log.push(MrXMove { ticket, revealed });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{Board, StationId, TicketType};
    use crate::gamestate::{GameState, PlayerId, PlayerState, Step, TicketInventory};
    use std::sync::Arc;

    // chain: 1 <-taxi-> 2 <-taxi-> 3, with an isolated detective parking spot (4).
    fn chain_board() -> Arc<Board> {
        Arc::new(Board {
            adjacency_map: vec![
                vec![(StationId { id: 2 }, TicketType::Taxi)],
                vec![
                    (StationId { id: 1 }, TicketType::Taxi),
                    (StationId { id: 3 }, TicketType::Taxi),
                ],
                vec![(StationId { id: 2 }, TicketType::Taxi)],
                vec![],
            ],
        })
    }

    fn game_with(reveal: RevealSchedule, mr_x_tickets: TicketInventory) -> Game {
        let players = vec![
            PlayerState::new(PlayerId::MrX, StationId { id: 1 }, mr_x_tickets),
            // Detective parked at the isolated station 4 so it never interferes.
            PlayerState::new(
                PlayerId::Detective(1),
                StationId { id: 4 },
                TicketInventory::new(0, 0, 0, 0, 0),
            ),
        ];
        Game::new(GameState::new(chain_board(), players), reveal)
    }

    #[test]
    fn reveal_schedule_is_reveal_leg() {
        let schedule = RevealSchedule::standard();
        assert!(schedule.is_reveal_leg(3));
        assert!(schedule.is_reveal_leg(24));
        assert!(!schedule.is_reveal_leg(1));
        assert!(!schedule.is_reveal_leg(4));
    }

    #[test]
    fn single_move_pushes_one_leg_concealed_off_reveal() {
        // No reveal until leg 3, so Mr X's first leg records the ticket but no
        // station.
        let mut game = game_with(RevealSchedule::standard(), TicketInventory::new(2, 0, 0, 0, 0));

        game.apply(Action::Single(Step { to: StationId { id: 2 }, ticket: TicketType::Taxi }));

        assert_eq!(game.history.mr_x_log.len(), 1);
        assert_eq!(game.history.mr_x_log[0].ticket, TicketType::Taxi);
        assert_eq!(game.history.mr_x_log[0].revealed, None);
    }

    #[test]
    fn reveal_leg_exposes_station() {
        // A schedule that reveals on the first leg exposes Mr X's post-leg station.
        let mut game = game_with(RevealSchedule::new(vec![1]), TicketInventory::new(2, 0, 0, 0, 0));

        game.apply(Action::Single(Step { to: StationId { id: 2 }, ticket: TicketType::Taxi }));

        assert_eq!(game.history.mr_x_log[0].revealed, Some(StationId { id: 2 }));
    }

    #[test]
    fn double_move_pushes_two_legs_and_can_reveal_intermediate() {
        // Reveal on leg 1 (the intermediate of the double) but not leg 2: the
        // double records two entries, exposing only the intermediate station.
        let mut game = game_with(RevealSchedule::new(vec![1]), TicketInventory::new(2, 0, 0, 0, 1));

        game.apply(Action::Double(
            Step { to: StationId { id: 2 }, ticket: TicketType::Taxi },
            Step { to: StationId { id: 3 }, ticket: TicketType::Taxi },
        ));

        assert_eq!(game.history.mr_x_log.len(), 2);
        assert_eq!(game.history.mr_x_log[0].revealed, Some(StationId { id: 2 }));
        assert_eq!(game.history.mr_x_log[1].revealed, None);
    }

    #[test]
    fn detective_moves_are_not_recorded() {
        // Only Mr X's moves enter the public log. Board gives the detective its
        // own edge (4 <-taxi-> 5) far from Mr X's area so it can move without
        // catching him.
        let board = Arc::new(Board {
            adjacency_map: vec![
                vec![(StationId { id: 2 }, TicketType::Taxi)], // 1
                vec![
                    (StationId { id: 1 }, TicketType::Taxi),
                    (StationId { id: 3 }, TicketType::Taxi),
                ], // 2
                vec![(StationId { id: 2 }, TicketType::Taxi)], // 3
                vec![(StationId { id: 5 }, TicketType::Taxi)], // 4
                vec![(StationId { id: 4 }, TicketType::Taxi)], // 5
            ],
        });
        let players = vec![
            PlayerState::new(PlayerId::MrX, StationId { id: 1 }, TicketInventory::new(2, 0, 0, 0, 0)),
            PlayerState::new(
                PlayerId::Detective(1),
                StationId { id: 4 },
                TicketInventory::new(2, 0, 0, 0, 0),
            ),
        ];
        let mut game = Game::new(GameState::new(board, players), RevealSchedule::standard());

        // Mr X moves (recorded), then the detective moves (not recorded).
        game.apply(Action::Single(Step { to: StationId { id: 2 }, ticket: TicketType::Taxi }));
        game.apply(Action::Single(Step { to: StationId { id: 5 }, ticket: TicketType::Taxi }));

        assert_eq!(game.history.mr_x_log.len(), 1);
    }
}
