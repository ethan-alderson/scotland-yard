import type { BoardDto } from "../api/board";
import EdgeLayer from "./EdgeLayer";
import StationLayer from "./StationLayer";
import TokenLayer, { type TokenSpec } from "./TokenLayer";

interface Props {
  board: BoardDto;
  showLabels: boolean;
  showEdges: boolean;
  tokens?: TokenSpec[];
  highlight?: Set<number>;
  onStationClick?: (id: number) => void;
}

// The whole board is one SVG whose viewBox is the map's native pixel size. The
// map image fills that space and every overlay (edges, stations, tokens) is
// drawn in the same coordinate system, so /api/board coordinates need no
// scaling. CSS scales the SVG responsively.
export default function MapBoard({
  board,
  showLabels,
  showEdges,
  tokens,
  highlight,
  onStationClick,
}: Props) {
  const { image, stations, edges } = board;
  return (
    <div className="board-frame">
      <svg viewBox={`0 0 ${image.w} ${image.h}`}>
        <image href={image.url} x={0} y={0} width={image.w} height={image.h} />
        {showEdges && <EdgeLayer edges={edges} stations={stations} />}
        <StationLayer
          stations={stations}
          showLabels={showLabels}
          highlight={highlight}
          onStationClick={onStationClick}
        />
        {tokens && (
          <TokenLayer tokens={tokens} stations={stations} onStationClick={onStationClick} />
        )}
      </svg>
    </div>
  );
}
