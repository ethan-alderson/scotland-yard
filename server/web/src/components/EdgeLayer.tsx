import { TICKET_COLORS, type EdgeDto, type StationGeom } from "../api/board";

interface Props {
  edges: EdgeDto[];
  stations: StationGeom[];
}

// Connection lines colored by transport. Useful as an extra alignment check —
// if the station dots are placed right, the lines trace the printed routes.
export default function EdgeLayer({ edges, stations }: Props) {
  const pos = new Map(stations.map((s) => [s.id, s]));

  return (
    <g opacity={0.7}>
      {edges.map((e, i) => {
        const a = pos.get(e.from);
        const b = pos.get(e.to);
        if (!a || !b) return null;
        return (
          <line
            key={i}
            x1={a.x}
            y1={a.y}
            x2={b.x}
            y2={b.y}
            stroke={TICKET_COLORS[e.ticket]}
            strokeWidth={4}
          />
        );
      })}
    </g>
  );
}
