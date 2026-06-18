// Typed client for the game-lifecycle endpoints. Mirrors the server's dto.rs.

import type { Ticket } from "./board";

export type PlayerIdDto = { kind: "mrx" } | { kind: "detective"; n: number };

export interface TicketsDto {
  taxi: number;
  bus: number;
  underground: number;
  black: number;
  double: number;
}

export interface PlayerDto {
  id: PlayerIdDto;
  station: number;
  tickets: TicketsDto;
}

export interface MrXLogEntryDto {
  ticket: Ticket;
  revealed: number | null;
}

export type WinnerDto = "mr_x" | "detectives";

export interface GameStateDto {
  game_id: string;
  current_player: number;
  turn_number: number;
  max_turns: number;
  is_terminal: boolean;
  winner: WinnerDto | null;
  players: PlayerDto[];
  mr_x_log: MrXLogEntryDto[];
}

export interface NewGameRequest {
  detectives: number;
  mr_x_start?: number;
  detective_starts?: number[];
  seed?: number;
}

export type ViewerDto =
  | { kind: "god" }
  | { kind: "mrx" }
  | { kind: "detective"; n: number };

export interface MrXViewDto {
  tickets: TicketsDto;
  station: number | null;
  last_revealed_station: number | null;
  log: MrXLogEntryDto[];
}

export interface ViewDto {
  game_id: string;
  viewer: ViewerDto;
  current_player: number;
  turn_number: number;
  max_turns: number;
  is_terminal: boolean;
  winner: WinnerDto | null;
  detectives: PlayerDto[];
  mr_x: MrXViewDto;
}

// In normal play the view follows whoever's turn it is (`current`); debug mode
// reveals everything (`god`).
export type ViewMode = "current" | "god";

export async function fetchView(gameId: string, mode: ViewMode): Promise<ViewDto> {
  const res = await fetch(`/api/games/${gameId}/view?as=${mode}`);
  if (!res.ok) {
    throw new Error(await errorMessage(res));
  }
  return (await res.json()) as ViewDto;
}

// --- Legal moves + applying a move ----------------------------------------

export interface StepDto {
  to: number;
  ticket: Ticket;
}

export interface MoveOptionDto {
  to: number;
  tickets: Ticket[];
}

export interface DoubleOptionDto {
  first: StepDto;
  second: StepDto;
}

export interface LegalMovesDto {
  player: PlayerIdDto;
  can_pass: boolean;
  singles: MoveOptionDto[];
  doubles: DoubleOptionDto[];
}

export type MoveRequest =
  | { kind: "single"; to: number; ticket: Ticket }
  | { kind: "double"; first: StepDto; second: StepDto }
  | { kind: "pass" };

export async function fetchLegalMoves(gameId: string): Promise<LegalMovesDto> {
  const res = await fetch(`/api/games/${gameId}/legal_moves`);
  if (!res.ok) {
    throw new Error(await errorMessage(res));
  }
  return (await res.json()) as LegalMovesDto;
}

// The POST response is god-view state, which we intentionally ignore (a detective
// client must not display Mr X's position); callers re-fetch the perspective
// view instead.
export async function applyMove(gameId: string, req: MoveRequest): Promise<void> {
  const res = await fetch(`/api/games/${gameId}/moves`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(req),
  });
  if (!res.ok) {
    throw new Error(await errorMessage(res));
  }
}

export async function createGame(req: NewGameRequest): Promise<GameStateDto> {
  const res = await fetch("/api/games", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(req),
  });
  if (!res.ok) {
    throw new Error(await errorMessage(res));
  }
  return (await res.json()) as GameStateDto;
}

// Pull the server's `{ "error": "…" }` message out of a failed response.
async function errorMessage(res: Response): Promise<string> {
  try {
    const body = (await res.json()) as { error?: string };
    if (body && body.error) return body.error;
  } catch {
    /* fall through to status text */
  }
  return `${res.status} ${res.statusText}`;
}

export function playerName(id: PlayerIdDto): string {
  return id.kind === "mrx" ? "Mr X" : `Detective ${id.n}`;
}
