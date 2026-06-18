import type { BoardDto } from "../api/board";
import EdgeLayer from "./EdgeLayer";
import StationLayer from "./StationLayer";

interface Props {
  board: BoardDto;
  showLabels: boolean;
  showEdges: boolean;
  onStationClick?: (id: number) => void;
}

// The whole board is one SVG whose viewBox is the map's native pixel size. The
// map image fills that space and every overlay (edges, stations) is drawn in the
// same coordinate system, so /api/board coordinates need no scaling. CSS scales
// the SVG responsively.
export default function MapBoard({ board, showLabels, showEdges, onStationClick }: Props) {
  const { image, stations, edges } = board;
  return (
    <div className="board-frame">
      <svg viewBox={`0 0 ${image.w} ${image.h}`}>
        <image href={image.url} x={0} y={0} width={image.w} height={image.h} />
        {showEdges && <EdgeLayer edges={edges} stations={stations} />}
        <StationLayer
          stations={stations}
          showLabels={showLabels}
          onStationClick={onStationClick}
        />
      </svg>
    </div>
  );
}
