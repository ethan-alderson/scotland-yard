use super::board::Board;
use super::board::TicketType;
use super::board::StationId;

use std::sync::Arc;

use serde::{Serialize, Deserialize};

#[derive(PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum PlayerId {
    MrX,
    Detective(u8),
    Detectives,
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct TicketInventory {
    taxi: u8,
    bus: u8,
    underground: u8,
    black: u8,
    pub double: u8,
}

// CONSTRUCTORS
impl TicketInventory {
    pub fn new(taxi: u8, bus: u8, underground: u8, black: u8, double: u8) -> Self {
        Self { taxi, bus, underground, black, double }
    }
}

// GETTERS
impl TicketInventory {
    pub fn get(&self, tt: TicketType) -> &u8 {
        match tt {
            TicketType::Taxi => &self.taxi,
            TicketType::Bus => &self.bus,
            TicketType::Underground => &self.underground,
            TicketType::Black => &self.black
        }
    }

    pub fn get_mut(&mut self, tt: TicketType) -> &mut u8 {
        match tt {
            TicketType::Taxi => &mut self.taxi,
            TicketType::Bus => &mut self.bus,
            TicketType::Underground => &mut self.underground,
            TicketType::Black => &mut self.black
        }
    }
}

// Behavior
impl TicketInventory {
    pub fn spend_ticket(&mut self, tt: TicketType) {
        *self.get_mut(tt) -= 1;
    }

    pub fn add_ticket(&mut self, tt: TicketType) {
        *self.get_mut(tt) += 1;
    }

    pub fn spend_double(&mut self) {
        self.double -= 1;
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PlayerState {
    pub id: PlayerId,
    pub station: StationId,
    pub tickets: TicketInventory
}

impl PlayerState {
    pub fn new(id: PlayerId, station: StationId, tickets: TicketInventory) -> Self {
        Self { id, station, tickets }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Step {
    pub to: StationId,
    pub ticket: TicketType,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Action {
    Single(Step),
    Double(Step, Step),
}

#[derive(Clone)]
pub struct GameState {

    // notion of fully observable information

    pub board: Arc<Board>,
    pub players: Vec<PlayerState>,
    // The player moving as an index in the player vector
    pub current_player: usize,
    pub turn_number: usize,

    // We also need termination

    pub is_terminal: bool,
    pub winner: Option<PlayerId>,
    pub max_turns: usize,

    // Some notion of move history for debugging can be added later

}

impl GameState {
    pub fn new(board: Arc<Board>, players: Vec<PlayerState>) -> Self {
        Self {
            board,
            players,
            current_player: 0,
            turn_number: 0,
            is_terminal: false,
            winner: None,
            max_turns: 22,
        }
    }
}

#[derive(Serialize)]
pub struct StateResponse {
    pub current_player: usize,
    pub players: Vec<PlayerState>,
    pub round: usize,
}

impl From<&GameState> for StateResponse {
    fn from(gs: &GameState) -> Self {
        Self {
            current_player: gs.current_player,
            players: gs.players.clone(),
            round: gs.turn_number,
        }
    }
}

struct GameHistory {
    // We need some notion of the partially observable information, something encoding Mr X's move history.
    // need his known positions and also the ticket he used every turn
    mr_x_revealed_positions: Vec<Option<u8>>,
    mr_x_actions: Vec<Action>,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assert_ticket_get() {
        let inv = TicketInventory::new(1, 2, 3, 4, 5);

        assert_eq!(*inv.get(TicketType::Taxi), 1);
        assert_eq!(*inv.get(TicketType::Bus), 2);
        assert_eq!(*inv.get(TicketType::Underground), 3);
        assert_eq!(*inv.get(TicketType::Black), 4);
        assert_eq!(inv.double, 5);
    }

    #[test]
    fn assert_spend_ticket_subtraction() {
        let mut inv = TicketInventory::new(1, 0, 0, 0, 0);

        inv.spend_ticket(TicketType::Taxi);
        assert_eq!(*inv.get(TicketType::Taxi), 0);
    }

    #[test]
    fn add_ticket_increments_taxi() {
        let mut inv = TicketInventory::new(0, 0, 0, 0, 0);
        inv.add_ticket(TicketType::Taxi);
        assert_eq!(*inv.get(TicketType::Taxi), 1);
    }

    #[test]
    fn add_ticket_increments_bus() {
        let mut inv = TicketInventory::new(0, 0, 0, 0, 0);
        inv.add_ticket(TicketType::Bus);
        assert_eq!(*inv.get(TicketType::Bus), 1);
    }

    #[test]
    fn add_ticket_increments_underground() {
        let mut inv = TicketInventory::new(0, 0, 0, 0, 0);
        inv.add_ticket(TicketType::Underground);
        assert_eq!(*inv.get(TicketType::Underground), 1);
    }

    #[test]
    fn add_ticket_increments_black() {
        let mut inv = TicketInventory::new(0, 0, 0, 0, 0);
        inv.add_ticket(TicketType::Black);
        assert_eq!(*inv.get(TicketType::Black), 1);
    }

    #[test]
    fn spend_double_decrements_count() {
        let mut inv = TicketInventory::new(0, 0, 0, 0, 2);
        inv.spend_double();
        assert_eq!(inv.double, 1);
    }

    #[test]
    fn add_ticket_does_not_affect_other_tickets() {
        let mut inv = TicketInventory::new(1, 1, 1, 1, 1);
        inv.add_ticket(TicketType::Taxi);
        assert_eq!(*inv.get(TicketType::Bus), 1);
        assert_eq!(*inv.get(TicketType::Underground), 1);
        assert_eq!(*inv.get(TicketType::Black), 1);
        assert_eq!(inv.double, 1);
    }
}