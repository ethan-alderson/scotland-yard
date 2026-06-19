import { useState } from "react";
import type { StationGeom } from "../api/board";

export interface TokenSpec {
  station: number;
  label: string;
  color: string;
  current: boolean;
  /** Ghost = Mr X's last-known position in a detective view (translucent/dashed). */
  ghost: boolean;
  /** Faded = the origin Mr X is occupying while a double move is mid-flight. */
  faded?: boolean;
  /** Pending = projected Mr X at the chosen intermediate stop of a double move. */
  pending?: boolean;
}

interface Props {
  tokens: TokenSpec[];
  stations: StationGeom[];
  onStationClick?: (id: number) => void;
}

// Player tokens drawn on top of the station overlay, in native pixel space.
export default function TokenLayer({ tokens, stations, onStationClick }: Props) {
  const pos = new Map(stations.map((s) => [s.id, s]));
  // Hovering a token peeks at the station beneath it (the token shrinks to a
  // little circle off to the top-right).
  const [hovered, setHovered] = useState<number | null>(null);

  return (
    <g>
      {tokens.map((t, i) => {
        const at = pos.get(t.station);
        if (!at) return null;
        const peek = hovered === i;
        return (
          <g
            key={i}
            opacity={t.faded ? 0.3 : 1}
            style={{ cursor: onStationClick ? "pointer" : "default" }}
            onMouseEnter={() => setHovered(i)}
            onMouseLeave={() => setHovered((h) => (h === i ? null : h))}
            onClick={() => onStationClick?.(t.station)}
          >
            {/* Stable, invisible hit area so the hover/click target doesn't move
                when the token shrinks away — keeps the peek from flickering. */}
            <circle cx={at.x} cy={at.y} r={34} fill="transparent" pointerEvents="all" />

            {peek ? (
              // Shrunk peek marker, offset clear of the station center.
              <circle
                cx={at.x + 26}
                cy={at.y - 26}
                r={12}
                fill={t.ghost ? "rgba(16, 17, 20, 0.6)" : t.color}
                stroke="#ffffff"
                strokeWidth={2}
                pointerEvents="none"
              />
            ) : (
              <g pointerEvents="none">
                {t.current && (
                  <circle cx={at.x} cy={at.y} r={42} fill="none" stroke="#ffffff" strokeWidth={5} />
                )}
                {/* Tentative intermediate stop: gold dashed halo around Mr X. */}
                {t.pending && (
                  <circle
                    cx={at.x}
                    cy={at.y}
                    r={46}
                    fill="none"
                    stroke="#e0b020"
                    strokeWidth={5}
                    strokeDasharray="9 6"
                  />
                )}
                <circle
                  cx={at.x}
                  cy={at.y}
                  r={34}
                  fill={t.ghost ? "rgba(16, 17, 20, 0.35)" : t.color}
                  stroke={t.ghost ? "#9aa3ad" : t.pending ? "#e0b020" : "#ffffff"}
                  strokeWidth={3}
                  strokeDasharray={t.ghost ? "7 5" : undefined}
                />
                <text
                  x={at.x}
                  y={at.y}
                  textAnchor="middle"
                  dominantBaseline="central"
                  fontSize={34}
                  fontWeight={800}
                  fill="#ffffff"
                  pointerEvents="none"
                >
                  {t.label}
                </text>
              </g>
            )}
          </g>
        );
      })}
    </g>
  );
}
