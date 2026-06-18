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

// What the player is currently looking through. `god` is the debug view.
export type Perspective = "god" | "mrx" | { detective: number };

export function perspectiveKey(p: Perspective): string {
  return p === "god" || p === "mrx" ? p : `det-${p.detective}`;
}

export async function fetchView(gameId: string, p: Perspective): Promise<ViewDto> {
  const query =
    p === "god" ? "as=god" : p === "mrx" ? "as=mrx" : `as=detective&n=${p.detective}`;
  const res = await fetch(`/api/games/${gameId}/view?${query}`);
  if (!res.ok) {
    throw new Error(await errorMessage(res));
  }
  return (await res.json()) as ViewDto;
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
