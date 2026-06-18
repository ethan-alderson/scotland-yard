import { useEffect, useState } from "react";
import { fetchBoard, type BoardDto } from "./api/board";
import MapBoard from "./components/MapBoard";

export default function App() {
  const [board, setBoard] = useState<BoardDto | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Calibration view defaults ON so the first thing you see is station ids over
  // the map — the Phase 3 alignment check.
  const [showLabels, setShowLabels] = useState(true);
  const [showEdges, setShowEdges] = useState(false);
  const [lastClicked, setLastClicked] = useState<number | null>(null);

  useEffect(() => {
    fetchBoard()
      .then(setBoard)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
  }, []);

  return (
    <div className="app">
      <div className="toolbar">
        <h1>Scotland Yard — Board</h1>
        <label>
          <input
            type="checkbox"
            checked={showLabels}
            onChange={(e) => setShowLabels(e.target.checked)}
          />
          Show station IDs
        </label>
        <label>
          <input
            type="checkbox"
            checked={showEdges}
            onChange={(e) => setShowEdges(e.target.checked)}
          />
          Show edges
        </label>
        <span className="spacer" />
        <span className="readout">
          {board ? `${board.stations.length} stations · ${board.edges.length} edges` : ""}
          {lastClicked !== null ? `  ·  clicked #${lastClicked}` : ""}
        </span>
      </div>

      {error && <div className="status error">Failed to load board: {error}</div>}
      {!error && !board && <div className="status">Loading board…</div>}
      {board && (
        <MapBoard
          board={board}
          showLabels={showLabels}
          showEdges={showEdges}
          onStationClick={setLastClicked}
        />
      )}
    </div>
  );
}
