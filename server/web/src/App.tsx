import { useEffect, useState } from "react";
import { fetchBoard, type BoardDto } from "./api/board";
import SetupPage from "./pages/SetupPage";
import PlayPage from "./pages/PlayPage";

export default function App() {
  const [board, setBoard] = useState<BoardDto | null>(null);
  const [boardError, setBoardError] = useState<string | null>(null);
  const [gameId, setGameId] = useState<string | null>(null);

  // Board geometry is static; fetch it once for the whole session.
  useEffect(() => {
    fetchBoard()
      .then(setBoard)
      .catch((e: unknown) => setBoardError(e instanceof Error ? e.message : String(e)));
  }, []);

  if (boardError) {
    return (
      <div className="app">
        <div className="status error">Failed to load board: {boardError}</div>
      </div>
    );
  }
  if (!board) {
    return (
      <div className="app">
        <div className="status">Loading board…</div>
      </div>
    );
  }

  return (
    <div className="app">
      {gameId ? (
        <PlayPage board={board} gameId={gameId} onNewGame={() => setGameId(null)} />
      ) : (
        <SetupPage onCreated={(game) => setGameId(game.game_id)} />
      )}
    </div>
  );
}
