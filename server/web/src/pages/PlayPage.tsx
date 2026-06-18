import { useCallback, useEffect, useState } from "react";
import type { BoardDto, Ticket } from "../api/board";
import {
  applyMove,
  fetchLegalMoves,
  fetchView,
  type LegalMovesDto,
  type MoveRequest,
  type StepDto,
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

type Mode = "single" | "double";

export default function PlayPage({ board, gameId, onNewGame }: Props) {
  // Debug reveals everything; otherwise the view follows whoever's turn it is.
  const [debug, setDebug] = useState(false);
  const [view, setView] = useState<ViewDto | null>(null);
  const [legalMoves, setLegalMoves] = useState<LegalMovesDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const [showLabels, setShowLabels] = useState(false);
  const [showEdges, setShowEdges] = useState(false);

  // Move-builder state.
  const [mode, setMode] = useState<Mode>("single");
  const [pendingFirst, setPendingFirst] = useState<StepDto | null>(null);
  const [pendingDest, setPendingDest] = useState<{ to: number; tickets: Ticket[] } | null>(null);

  const clearPending = useCallback(() => {
    setPendingFirst(null);
    setPendingDest(null);
  }, []);

  // Fetch the current view and, while the game is live, the current player's
  // legal moves. In normal play the view IS the current player's, so they can
  // always act; in debug we see all and can drive any player.
  const reload = useCallback(async () => {
    setError(null);
    const v = await fetchView(gameId, debug ? "god" : "current");
    setView(v);
    setMode("single");
    clearPending();
    setLegalMoves(v.is_terminal ? null : await fetchLegalMoves(gameId));
  }, [gameId, debug, clearPending]);

  useEffect(() => {
    reload().catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
  }, [reload]);

  async function submit(req: MoveRequest) {
    setSubmitting(true);
    setError(null);
    try {
      await applyMove(gameId, req);
      await reload();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSubmitting(false);
    }
  }

  const isMrXTurn = view?.current_player === 0;
  const doublesAvailable = !!legalMoves && legalMoves.doubles.length > 0 && isMrXTurn;
  const effectiveMode: Mode = doublesAvailable ? mode : "single";

  // Tickets available to reach `to` at the current stage of the builder.
  function ticketsFor(to: number): Ticket[] {
    if (!legalMoves) return [];
    if (effectiveMode === "single") {
      return legalMoves.singles.find((s) => s.to === to)?.tickets ?? [];
    }
    if (!pendingFirst) {
      return unique(legalMoves.doubles.filter((d) => d.first.to === to).map((d) => d.first.ticket));
    }
    return unique(
      legalMoves.doubles
        .filter(
          (d) =>
            d.first.to === pendingFirst.to &&
            d.first.ticket === pendingFirst.ticket &&
            d.second.to === to,
        )
        .map((d) => d.second.ticket),
    );
  }

  // The set of legal target stations to highlight at the current stage.
  function legalTargets(): Set<number> {
    if (!legalMoves) return new Set();
    if (effectiveMode === "single") return new Set(legalMoves.singles.map((s) => s.to));
    if (!pendingFirst) return new Set(legalMoves.doubles.map((d) => d.first.to));
    return new Set(
      legalMoves.doubles
        .filter((d) => d.first.to === pendingFirst.to && d.first.ticket === pendingFirst.ticket)
        .map((d) => d.second.to),
    );
  }

  // Commit a chosen (destination, ticket) for the current stage.
  function act(to: number, ticket: Ticket) {
    setPendingDest(null);
    if (effectiveMode === "single") {
      void submit({ kind: "single", to, ticket });
    } else if (!pendingFirst) {
      setPendingFirst({ to, ticket });
    } else {
      void submit({ kind: "double", first: pendingFirst, second: { to, ticket } });
    }
  }

  function handleStationClick(id: number) {
    if (!legalMoves || submitting) return;
    const tickets = ticketsFor(id);
    if (tickets.length === 0) return; // not a legal target right now — ignore
    if (tickets.length === 1) act(id, tickets[0]);
    else setPendingDest({ to: id, tickets });
  }

  function changeMode(next: Mode) {
    setMode(next);
    clearPending();
  }

  return (
    <div className="play">
      <div className="toolbar">
        <h1>Scotland Yard</h1>
        <button onClick={onNewGame}>New game</button>
        <span className="spacer" />
        <label className="debug-toggle">
          <input type="checkbox" checked={debug} onChange={(e) => setDebug(e.target.checked)} />
          Debug (reveal all)
        </label>
        <label>
          <input type="checkbox" checked={showLabels} onChange={(e) => setShowLabels(e.target.checked)} />
          Station IDs
        </label>
        <label>
          <input type="checkbox" checked={showEdges} onChange={(e) => setShowEdges(e.target.checked)} />
          Edges
        </label>
      </div>

      {error && <div className="form-error">{error}</div>}
      {!view && !error && <div className="status">Loading…</div>}

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

          {!view.is_terminal &&
            (legalMoves ? (
              <div className="move-bar">
                <span className="move-who">{currentName(view.current_player)} — your move</span>

                {isMrXTurn && doublesAvailable && (
                  <span className="mode-toggle">
                    <button className={effectiveMode === "single" ? "active" : ""} onClick={() => changeMode("single")}>
                      Single
                    </button>
                    <button className={effectiveMode === "double" ? "active" : ""} onClick={() => changeMode("double")}>
                      Double
                    </button>
                  </span>
                )}

                {effectiveMode === "double" && (
                  <span className="double-status">
                    {pendingFirst
                      ? `1st leg → ${pendingFirst.to} (${pendingFirst.ticket}); pick the 2nd stop`
                      : "pick the 1st stop"}
                  </span>
                )}

                {pendingDest ? (
                  <span className="ticket-picker">
                    Ticket to {pendingDest.to}:
                    {pendingDest.tickets.map((t) => (
                      <button key={t} onClick={() => act(pendingDest.to, t)}>
                        {TICKET_ICON[t]} {t}
                      </button>
                    ))}
                  </span>
                ) : (
                  <span className="hint">Click a highlighted station.</span>
                )}

                {(pendingFirst || pendingDest) && <button onClick={clearPending}>Reset</button>}

                {legalMoves.can_pass && (
                  <button className="primary" onClick={() => submit({ kind: "pass" })}>
                    Pass (no moves)
                  </button>
                )}

                {submitting && <span className="hint">submitting…</span>}
              </div>
            ) : (
              <div className="move-bar hint-bar">Loading moves…</div>
            ))}

          <div className="play-layout">
            <MapBoard
              board={board}
              showLabels={showLabels}
              showEdges={showEdges}
              tokens={buildTokens(view)}
              highlight={legalMoves ? legalTargets() : undefined}
              onStationClick={legalMoves ? handleStationClick : undefined}
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

function unique<T>(xs: T[]): T[] {
  return [...new Set(xs)];
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
