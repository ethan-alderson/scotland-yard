// Typed client for GET /api/board. Mirrors the server's board_geometry DTOs.

export type Ticket = "taxi" | "bus" | "underground" | "black";

export interface StationGeom {
  id: number;
  x: number;
  y: number;
}

export interface EdgeDto {
  from: number;
  to: number;
  ticket: Ticket;
}

export interface ImageMeta {
  w: number;
  h: number;
  url: string;
}

export interface BoardDto {
  image: ImageMeta;
  stations: StationGeom[];
  edges: EdgeDto[];
}

export async function fetchBoard(): Promise<BoardDto> {
  const res = await fetch("/api/board");
  if (!res.ok) {
    throw new Error(`GET /api/board failed: ${res.status} ${res.statusText}`);
  }
  return (await res.json()) as BoardDto;
}

// Standard Scotland Yard transport colors.
export const TICKET_COLORS: Record<Ticket, string> = {
  taxi: "#f5c518",
  bus: "#1f9d6f",
  underground: "#e3433b",
  black: "#0f0f0f",
};
