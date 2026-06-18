import type { StationGeom } from "../api/board";

export interface TokenSpec {
  station: number;
  label: string;
  color: string;
  current: boolean;
  /** Ghost = Mr X's last-known position in a detective view (translucent/dashed). */
  ghost: boolean;
}

interface Props {
  tokens: TokenSpec[];
  stations: StationGeom[];
}

// Player tokens drawn on top of the station overlay, in native pixel space.
export default function TokenLayer({ tokens, stations }: Props) {
  const pos = new Map(stations.map((s) => [s.id, s]));

  return (
    <g>
      {tokens.map((t, i) => {
        const at = pos.get(t.station);
        if (!at) return null;
        return (
          <g key={i}>
            {t.current && (
              <circle cx={at.x} cy={at.y} r={42} fill="none" stroke="#ffffff" strokeWidth={5} />
            )}
            <circle
              cx={at.x}
              cy={at.y}
              r={34}
              fill={t.ghost ? "rgba(16, 17, 20, 0.35)" : t.color}
              stroke={t.ghost ? "#9aa3ad" : "#ffffff"}
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
        );
      })}
    </g>
  );
}
