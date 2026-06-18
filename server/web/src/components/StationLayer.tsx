import type { StationGeom } from "../api/board";

interface Props {
  stations: StationGeom[];
  showLabels: boolean;
  highlight?: Set<number>;
  onStationClick?: (id: number) => void;
}

// Station markers, drawn in the map's native pixel coordinate space (the parent
// SVG's viewBox is the full image), so coordinates from /api/board are used 1:1.
// Highlighted stations (legal move targets) get a green ring.
export default function StationLayer({ stations, showLabels, highlight, onStationClick }: Props) {
  return (
    <g>
      {stations.map((s) => {
        const hot = highlight?.has(s.id) ?? false;
        return (
          <g
            key={s.id}
            onClick={() => onStationClick?.(s.id)}
            style={{ cursor: onStationClick ? "pointer" : "default" }}
          >
            {hot && (
              <circle
                cx={s.x}
                cy={s.y}
                r={40}
                fill="rgba(57, 211, 83, 0.25)"
                stroke="#39d353"
                strokeWidth={5}
              />
            )}
            <circle
              cx={s.x}
              cy={s.y}
              r={26}
              fill="rgba(64, 156, 255, 0.55)"
              stroke="#0b3a66"
              strokeWidth={2}
            />
            {showLabels && (
              <text
                x={s.x}
                y={s.y}
                textAnchor="middle"
                dominantBaseline="central"
                fontSize={16}
                fontWeight={700}
                fill="#ffffff"
                pointerEvents="none"
              >
                {s.id}
              </text>
            )}
          </g>
        );
      })}
    </g>
  );
}
