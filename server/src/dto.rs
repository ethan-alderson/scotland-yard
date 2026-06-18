//! API data-transfer types and conversions to/from engine types.
//!
//! The server owns its wire format. Engine internals (notably `GameState`, which
//! holds an `Arc<Board>` and is not `Serialize`) never go on the wire directly —
//! everything is projected through these DTOs at the boundary.

use serde::{Deserialize, Serialize};

use engine::board::{StationId, TicketType};
use engine::gamestate::{Action, PlayerId, PlayerState, Step, TicketInventory, Winner};
use engine::history::Game;
use engine::rules::legal_actions;

// ---------------------------------------------------------------------------
// Leaf types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TicketDto {
    Taxi,
    Bus,
    Underground,
    Black,
}

impl From<TicketType> for TicketDto {
    fn from(t: TicketType) -> Self {
        match t {
            TicketType::Taxi => TicketDto::Taxi,
            TicketType::Bus => TicketDto::Bus,
            TicketType::Underground => TicketDto::Underground,
            TicketType::Black => TicketDto::Black,
        }
    }
}

impl From<TicketDto> for TicketType {
    fn from(t: TicketDto) -> Self {
        match t {
            TicketDto::Taxi => TicketType::Taxi,
            TicketDto::Bus => TicketType::Bus,
            TicketDto::Underground => TicketType::Underground,
            TicketDto::Black => TicketType::Black,
        }
    }
}

#[derive(Serialize, Clone, Copy)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum PlayerIdDto {
    Mrx,
    Detective { n: u8 },
}

impl From<PlayerId> for PlayerIdDto {
    fn from(id: PlayerId) -> Self {
        match id {
            PlayerId::MrX => PlayerIdDto::Mrx,
            PlayerId::Detective(n) => PlayerIdDto::Detective { n },
        }
    }
}

#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum WinnerDto {
    MrX,
    Detectives,
}

impl From<Winner> for WinnerDto {
    fn from(w: Winner) -> Self {
        match w {
            Winner::MrX => WinnerDto::MrX,
            Winner::Detectives => WinnerDto::Detectives,
        }
    }
}

#[derive(Serialize)]
pub struct TicketsDto {
    pub taxi: u8,
    pub bus: u8,
    pub underground: u8,
    pub black: u8,
    pub double: u8,
}

impl From<&TicketInventory> for TicketsDto {
    fn from(inv: &TicketInventory) -> Self {
        Self {
            taxi: *inv.get(TicketType::Taxi),
            bus: *inv.get(TicketType::Bus),
            underground: *inv.get(TicketType::Underground),
            black: *inv.get(TicketType::Black),
            double: inv.double,
        }
    }
}

/// A single move leg, used both in legal-move listings and incoming move
/// requests. Station ids are 1-based, matching the board.
#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct StepDto {
    pub to: u16,
    pub ticket: TicketDto,
}

impl StepDto {
    pub fn to_step(self) -> Result<Step, String> {
        if self.to < 1 || self.to > 255 {
            return Err(format!("station id {} out of range", self.to));
        }
        Ok(Step { to: StationId { id: self.to as u8 }, ticket: self.ticket.into() })
    }
}

impl From<&Step> for StepDto {
    fn from(s: &Step) -> Self {
        Self { to: s.to.id as u16, ticket: s.ticket.into() }
    }
}

// ---------------------------------------------------------------------------
// Responses
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct PlayerDto {
    pub id: PlayerIdDto,
    pub station: u16,
    pub tickets: TicketsDto,
}

impl From<&PlayerState> for PlayerDto {
    fn from(p: &PlayerState) -> Self {
        Self {
            id: p.id.into(),
            station: p.station.id as u16,
            tickets: (&p.tickets).into(),
        }
    }
}

#[derive(Serialize)]
pub struct MrXLogEntryDto {
    pub ticket: TicketDto,
    /// Mr X's station, present only on reveal legs.
    pub revealed: Option<u16>,
}

/// Full, god-view game state. (Perspective-filtered views arrive in a later
/// phase via the observation layer.)
#[derive(Serialize)]
pub struct GameStateDto {
    pub game_id: String,
    pub current_player: usize,
    pub turn_number: usize,
    pub max_turns: usize,
    pub is_terminal: bool,
    pub winner: Option<WinnerDto>,
    pub players: Vec<PlayerDto>,
    pub mr_x_log: Vec<MrXLogEntryDto>,
}

impl GameStateDto {
    pub fn from_game(game_id: &str, game: &Game) -> Self {
        let s = &game.state;
        Self {
            game_id: game_id.to_string(),
            current_player: s.current_player,
            turn_number: s.turn_number,
            max_turns: s.max_turns,
            is_terminal: s.is_terminal,
            winner: s.winner.map(Into::into),
            players: s.players.iter().map(PlayerDto::from).collect(),
            mr_x_log: game
                .history
                .mr_x_log
                .iter()
                .map(|m| MrXLogEntryDto {
                    ticket: m.ticket.into(),
                    revealed: m.revealed.map(|st| st.id as u16),
                })
                .collect(),
        }
    }
}

/// One destination reachable by the current player, with the ticket(s) that get
/// there (e.g. a square reachable by both taxi and a concealing black ticket).
#[derive(Serialize)]
pub struct MoveOptionDto {
    pub to: u16,
    pub tickets: Vec<TicketDto>,
}

#[derive(Serialize)]
pub struct DoubleOptionDto {
    pub first: StepDto,
    pub second: StepDto,
}

/// The current player's legal moves, shaped for the UI: single steps grouped by
/// destination, every double-move pair listed explicitly, and whether a forced
/// pass is the only option.
#[derive(Serialize)]
pub struct LegalMovesDto {
    pub player: PlayerIdDto,
    pub can_pass: bool,
    pub singles: Vec<MoveOptionDto>,
    pub doubles: Vec<DoubleOptionDto>,
}

impl LegalMovesDto {
    pub fn from_game(game: &Game) -> Self {
        let s = &game.state;
        let actions = legal_actions(s);

        // Group single steps by destination, preserving first-seen order and
        // de-duplicating tickets.
        let mut singles: Vec<MoveOptionDto> = Vec::new();
        let mut doubles: Vec<DoubleOptionDto> = Vec::new();
        let mut can_pass = false;

        for action in &actions {
            match action {
                Action::Single(step) => {
                    let to = step.to.id as u16;
                    let ticket: TicketDto = step.ticket.into();
                    match singles.iter_mut().find(|m| m.to == to) {
                        Some(opt) => {
                            if !opt.tickets.contains(&ticket) {
                                opt.tickets.push(ticket);
                            }
                        }
                        None => singles.push(MoveOptionDto { to, tickets: vec![ticket] }),
                    }
                }
                Action::Double(first, second) => doubles.push(DoubleOptionDto {
                    first: first.into(),
                    second: second.into(),
                }),
                Action::Pass => can_pass = true,
            }
        }

        Self {
            player: s.players[s.current_player].id.into(),
            can_pass,
            singles,
            doubles,
        }
    }
}

// ---------------------------------------------------------------------------
// Requests
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct NewGameRequest {
    pub detectives: usize,
    /// Explicit Mr X start; deal randomly when omitted (with `detective_starts`).
    #[serde(default)]
    pub mr_x_start: Option<u8>,
    /// Explicit detective starts; must match `detectives` in length when given.
    #[serde(default)]
    pub detective_starts: Option<Vec<u8>>,
    /// Seed the random start deal for reproducible games/tests.
    #[serde(default)]
    pub seed: Option<u64>,
}

/// An incoming move. Tagged by `kind`: `single`, `double`, or `pass`.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MoveRequest {
    Single { to: u16, ticket: TicketDto },
    Double { first: StepDto, second: StepDto },
    Pass,
}

impl MoveRequest {
    pub fn to_action(&self) -> Result<Action, String> {
        match self {
            MoveRequest::Single { to, ticket } => {
                Ok(Action::Single(StepDto { to: *to, ticket: *ticket }.to_step()?))
            }
            MoveRequest::Double { first, second } => {
                Ok(Action::Double(first.to_step()?, second.to_step()?))
            }
            MoveRequest::Pass => Ok(Action::Pass),
        }
    }
}
