import type { StationGeom } from "../api/board";

interface Props {
  stations: StationGeom[];
  showLabels: boolean;
  onStationClick?: (id: number) => void;
}

// Station markers, drawn in the map's native pixel coordinate space (the parent
// SVG's viewBox is the full image), so coordinates from /api/board are used 1:1.
export default function StationLayer({ stations, showLabels, onStationClick }: Props) {
  return (
    <g>
      {stations.map((s) => (
        <g
          key={s.id}
          onClick={() => onStationClick?.(s.id)}
          style={{ cursor: onStationClick ? "pointer" : "default" }}
        >
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
      ))}
    </g>
  );
}
