import { useEffect, useState } from "react";
import type { BoardDto, Ticket } from "../api/board";
import {
  fetchView,
  perspectiveKey,
  type Perspective,
  type ViewDto,
} from "../api/games";
import MapBoard from "../components/MapBoard";
import type { TokenSpec } from "../components/TokenLayer";

interface Props {
  board: BoardDto;
  gameId: string;
  onNewGame: () => void;
}

const DETECTIVE_COLORS = ["#2f6fec", "#27a35b", "#e0b020", "#9b59b6", "#e0552b"];
const MRX_COLOR = "#101114";
const TICKET_ICON: Record<Ticket, string> = {
  taxi: "🚕",
  bus: "🚌",
  underground: "Ⓤ",
  black: "⬛",
};

export default function PlayPage({ board, gameId, onNewGame }: Props) {
  const [perspective, setPerspective] = useState<Perspective>("god");
  const [view, setView] = useState<ViewDto | null>(null);
  const [error, setError] = useState<string | null>(null);

  const [showLabels, setShowLabels] = useState(false);
  const [showEdges, setShowEdges] = useState(false);

  // Re-fetch whenever the game or the chosen perspective changes. (Phase 6 will
  // also re-fetch after each move.)
  useEffect(() => {
    let alive = true;
    setError(null);
    fetchView(gameId, perspective)
      .then((v) => alive && setView(v))
      .catch((e: unknown) => alive && setError(e instanceof Error ? e.message : String(e)));
    return () => {
      alive = false;
    };
  }, [gameId, perspectiveKey(perspective)]);

  const detectiveNumbers = view
    ? view.detectives.flatMap((d) => (d.id.kind === "detective" ? [d.id.n] : []))
    : [];

  return (
    <div className="play">
      <div className="toolbar">
        <h1>Scotland Yard</h1>
        <button onClick={onNewGame}>New game</button>
        <span className="spacer" />
        <label>
          <input
            type="checkbox"
            checked={showLabels}
            onChange={(e) => setShowLabels(e.target.checked)}
          />
          Station IDs
        </label>
        <label>
          <input
            type="checkbox"
            checked={showEdges}
            onChange={(e) => setShowEdges(e.target.checked)}
          />
          Edges
        </label>
      </div>

      <div className="perspective">
        <span className="perspective-label">View as</span>
        <button
          className={perspective === "god" ? "active debug" : "debug"}
          onClick={() => setPerspective("god")}
        >
          God (debug)
        </button>
        <button
          className={perspective === "mrx" ? "active" : ""}
          onClick={() => setPerspective("mrx")}
        >
          Mr X
        </button>
        {detectiveNumbers.map((n) => (
          <button
            key={n}
            className={typeof perspective === "object" && perspective.detective === n ? "active" : ""}
            onClick={() => setPerspective({ detective: n })}
          >
            Det {n}
          </button>
        ))}
      </div>

      {error && <div className="form-error">{error}</div>}
      {!view && !error && <div className="status">Loading view…</div>}

      {view && (
        <>
          {view.is_terminal ? (
            <div className="banner">
              Game over — {view.winner === "mr_x" ? "Mr X wins" : "Detectives win"}
            </div>
          ) : (
            <div className="banner subtle">
              Round {view.turn_number} · {currentName(view.current_player)} to move
            </div>
          )}

          <div className="play-layout">
            <MapBoard
              board={board}
              showLabels={showLabels}
              showEdges={showEdges}
              tokens={buildTokens(view)}
            />

            <aside className="panel">
              <h2>Mr X</h2>
              <div className={`player-card ${view.current_player === 0 ? "active" : ""}`}>
                <div className="player-head">
                  <span className="dot" data-kind="mrx" />
                  <strong>
                    {view.mr_x.station != null
                      ? `@ ${view.mr_x.station}`
                      : view.mr_x.last_revealed_station != null
                        ? `last seen @ ${view.mr_x.last_revealed_station}`
                        : "position unknown"}
                  </strong>
                </div>
                <div className="tickets">
                  <span>🚕 {view.mr_x.tickets.taxi}</span>
                  <span>🚌 {view.mr_x.tickets.bus}</span>
                  <span>Ⓤ {view.mr_x.tickets.underground}</span>
                  <span>⬛ {view.mr_x.tickets.black}</span>
                  <span>2× {view.mr_x.tickets.double}</span>
                </div>
              </div>

              <h3>Travel log</h3>
              {view.mr_x.log.length === 0 ? (
                <p className="hint">No moves yet.</p>
              ) : (
                <ol className="mrx-log">
                  {view.mr_x.log.map((leg, i) => (
                    <li key={i}>
                      <span className="leg-no">{i + 1}</span>
                      <span className="leg-ticket">{TICKET_ICON[leg.ticket]}</span>
                      {leg.revealed != null ? (
                        <span className="leg-reveal">@ {leg.revealed}</span>
                      ) : (
                        <span className="leg-hidden">hidden</span>
                      )}
                    </li>
                  ))}
                </ol>
              )}

              <h2>Detectives</h2>
              <ul className="player-list">
                {view.detectives.map((d) => {
                  const n = d.id.kind === "detective" ? d.id.n : 0;
                  return (
                    <li key={n} className={view.current_player === n ? "active" : ""}>
                      <div className="player-head">
                        <span
                          className="dot"
                          style={{ background: DETECTIVE_COLORS[(n - 1) % DETECTIVE_COLORS.length] }}
                        />
                        <strong>Detective {n}</strong>
                        <span className="station">@ {d.station}</span>
                      </div>
                      <div className="tickets">
                        <span>🚕 {d.tickets.taxi}</span>
                        <span>🚌 {d.tickets.bus}</span>
                        <span>Ⓤ {d.tickets.underground}</span>
                      </div>
                    </li>
                  );
                })}
              </ul>
              <p className="hint">game #{view.game_id}</p>
            </aside>
          </div>
        </>
      )}
    </div>
  );
}

// player index 0 is Mr X; index k is Detective k.
function currentName(index: number): string {
  return index === 0 ? "Mr X" : `Detective ${index}`;
}

function buildTokens(view: ViewDto): TokenSpec[] {
  const tokens: TokenSpec[] = [];

  for (const d of view.detectives) {
    const n = d.id.kind === "detective" ? d.id.n : 0;
    tokens.push({
      station: d.station,
      label: String(n),
      color: DETECTIVE_COLORS[(n - 1) % DETECTIVE_COLORS.length],
      current: view.current_player === n,
      ghost: false,
    });
  }

  const mrx = view.mr_x;
  if (mrx.station != null) {
    tokens.push({
      station: mrx.station,
      label: "X",
      color: MRX_COLOR,
      current: view.current_player === 0,
      ghost: false,
    });
  } else if (mrx.last_revealed_station != null) {
    // Detective view between reveals: show his last-known spot as a ghost.
    tokens.push({
      station: mrx.last_revealed_station,
      label: "?",
      color: MRX_COLOR,
      current: false,
      ghost: true,
    });
  }

  return tokens;
}
