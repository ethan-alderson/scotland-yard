//! Observation projection (#12): the data structure representing what a given
//! viewer legitimately knows about the world.
//!
//! `observe` projects fully-observable ground truth down to a viewer's
//! information set. Mr X sees everything, so his observation is just the ground
//! truth; a detective sees public state (every player's position and tickets)
//! plus Mr X's ticket history and the stations exposed on reveal legs — but not
//! Mr X's exact station between reveals.

use std::sync::Arc;

use super::board::{Board};
use super::gamestate::{GameState, PlayerId, PlayerState, TicketInventory};
use super::history::{Game, GameHistory, MrXMove, RevealSchedule};

pub enum Observation {
    MrX(GameState),
    Detective(DetectiveObservation),
}

#[derive(Clone)]
pub struct DetectiveObservation {
    pub board: Arc<Board>,
    pub detectives: Vec<PlayerState>,  // public: positions + ticket counts
    pub mr_x_log: Vec<MrXMove>,        // Mr X's ticket history + reveals
    // pub mr_x_start: Option<StationId>, // hidden in standard SY; Some only if revealed
    pub mr_x_tickets: TicketInventory, // start loadout − tickets spent (derivable, known)
    pub current_player: usize,
    pub turn_number: usize,
    pub reveal: RevealSchedule,
    pub max_turns: usize,
}

/// Project ground truth to what `viewer` knows. `reveal` is part of the public
/// configuration (the detective observation carries the schedule, so it is
/// threaded in alongside the state and history).
pub fn observe(
    state: &GameState,
    history: &GameHistory,
    reveal: &RevealSchedule,
    viewer: PlayerId,
) -> Observation {
    match viewer {
        // Mr X's information set is the full ground truth.
        PlayerId::MrX => Observation::MrX(state.clone()),
        PlayerId::Detective(_) => {
            let detectives = state
                .players
                .iter()
                .filter(|p| matches!(p.id, PlayerId::Detective(_)))
                .copied()
                .collect();

            // Mr X's ticket counts are public (start loadout minus what the log
            // records as spent), so they equal his ground-truth inventory — copy
            // it straight across rather than recompute.
            let mr_x = state
                .players
                .iter()
                .find(|p| matches!(p.id, PlayerId::MrX))
                .expect("no Mr X in players");

            Observation::Detective(DetectiveObservation {
                board: Arc::clone(&state.board),
                detectives,
                mr_x_log: history.mr_x_log.clone(),
                // mr_x_start: history.mr_x_start,
                mr_x_tickets: mr_x.tickets,
                current_player: state.current_player,
                turn_number: state.turn_number,
                reveal: reveal.clone(),
                max_turns: state.max_turns,
            })
        }
    }
}

impl Game {
    /// Convenience projection from the top-level game, which already holds the
    /// state, history, and reveal schedule together.
    pub fn observe(&self, viewer: PlayerId) -> Observation {
        observe(&self.state, &self.history, &self.reveal, viewer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{Board, TicketType, StationId};
    use crate::gamestate::{Action, Step, TicketInventory};
    use crate::history::RevealSchedule;

    // 1 <-taxi-> 2 <-taxi-> 3, plus an isolated parking station 4.
    fn board() -> Arc<Board> {
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

    fn game(reveal: RevealSchedule) -> Game {
        let players = vec![
            PlayerState::new(PlayerId::MrX, StationId { id: 1 }, TicketInventory::new(5, 0, 0, 0, 0)),
            PlayerState::new(
                PlayerId::Detective(1),
                StationId { id: 4 },
                TicketInventory::new(3, 0, 0, 0, 0),
            ),
        ];
        Game::new(GameState::new(board(), players), reveal)
    }

    #[test]
    fn mrx_observation_is_full_ground_truth() {
        let g = game(RevealSchedule::standard());
        match g.observe(PlayerId::MrX) {
            Observation::MrX(state) => {
                assert_eq!(state.players[0].station, StationId { id: 1 });
            }
            _ => panic!("Mr X should get a full observation"),
        }
    }

    #[test]
    fn detective_observation_hides_station_between_reveals() {
        // Reveal only at leg 3; after one move Mr X's station is not exposed.
        let mut g = game(RevealSchedule::new(vec![3]));
        g.apply(Action::Single(Step { to: StationId { id: 2 }, ticket: TicketType::Taxi }));

        match g.observe(PlayerId::Detective(1)) {
            Observation::Detective(obs) => {
                assert_eq!(obs.mr_x_log.len(), 1);
                assert_eq!(obs.mr_x_log[0].revealed, None);
                // Mr X spent one taxi ticket; the public inventory reflects that.
                assert_eq!(*obs.mr_x_tickets.get(TicketType::Taxi), 4);
                assert_eq!(obs.detectives.len(), 1);
                assert_eq!(obs.detectives[0].station, StationId { id: 4 });
            }
            _ => panic!("detective should get a detective observation"),
        }
    }

    #[test]
    fn detective_observation_exposes_station_on_reveal_leg() {
        let mut g = game(RevealSchedule::new(vec![1]));
        g.apply(Action::Single(Step { to: StationId { id: 2 }, ticket: TicketType::Taxi }));

        match g.observe(PlayerId::Detective(1)) {
            Observation::Detective(obs) => {
                assert_eq!(obs.mr_x_log[0].revealed, Some(StationId { id: 2 }));
            }
            _ => panic!("detective should get a detective observation"),
        }
    }
}
