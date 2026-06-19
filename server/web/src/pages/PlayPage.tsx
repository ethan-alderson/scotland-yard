import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
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
// Ticket types offered in the move menu, in display order. Detectives never
// hold black tickets, so they only see the first three.
const MRX_TICKET_TYPES: Ticket[] = ["taxi", "bus", "underground", "black"];
const DETECTIVE_TICKET_TYPES: Ticket[] = ["taxi", "bus", "underground"];

// The face of a wheel ticket button: a per-ticket color class plus its glyph.
function ticketFace(t: Ticket): { cls: string; node: ReactNode } {
  switch (t) {
    case "taxi":
      return { cls: "ticket-taxi", node: <span className="t-label">TAXI</span> };
    case "bus":
      return { cls: "ticket-bus", node: <span className="t-label">BUS</span> };
    case "underground":
      return { cls: "ticket-underground", node: <span className="t-roundel">U</span> };
    case "black":
      return { cls: "ticket-black", node: <span className="t-label">BLACK</span> };
  }
}

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
  // The station whose move menu is currently open (null = no menu). Usable
  // tickets are derived live from `legalMoves`, so toggling single/double
  // updates the menu in place.
  const [pendingDest, setPendingDest] = useState<{ to: number } | null>(null);

  // Station id -> pixel coordinates, for anchoring the move menu on the board.
  const stationPos = useMemo(() => {
    const m = new Map<number, { x: number; y: number }>();
    for (const s of board.stations) m.set(s.id, { x: s.x, y: s.y });
    return m;
  }, [board]);

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
    if (!legalTargets().has(id)) return; // not a legal target right now — ignore
    // Always open the menu (even for a single ticket option) so the move must
    // be confirmed deliberately.
    setPendingDest({ to: id });
  }

  // Toggle single/double from inside the menu. We're always at the first leg
  // here (the toggle is hidden once a first leg is committed), so drop any
  // stale pending leg but keep the menu open.
  function setMenuMode(next: Mode) {
    setMode(next);
    setPendingFirst(null);
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

          <div className="play-layout">
            <div className="players-bar">
              <div className={`player-card mrx ${view.current_player === 0 ? "active" : ""}`}>
                <div className="player-head">
                  <span className="dot" data-kind="mrx" />
                  <strong>Mr X</strong>
                  <span className="station">
                    {view.mr_x.station != null
                      ? `@ ${view.mr_x.station}`
                      : view.mr_x.last_revealed_station != null
                        ? `last seen @ ${view.mr_x.last_revealed_station}`
                        : "unknown"}
                  </span>
                </div>
                <div className="tickets">
                  <span>🚕 {view.mr_x.tickets.taxi}</span>
                  <span>🚌 {view.mr_x.tickets.bus}</span>
                  <span>Ⓤ {view.mr_x.tickets.underground}</span>
                  <span>⬛ {view.mr_x.tickets.black}</span>
                  <span>2× {view.mr_x.tickets.double}</span>
                </div>
                {view.mr_x.log.length === 0 ? (
                  <p className="hint log-hint">No moves yet.</p>
                ) : (
                  <ol className="mrx-log horizontal">
                    {view.mr_x.log.map((leg, i) => (
                      <li key={i}>
                        <span className="leg-no">{i + 1}</span>
                        <span className="leg-ticket">{TICKET_ICON[leg.ticket]}</span>
                        {leg.revealed != null ? (
                          <span className="leg-reveal">@ {leg.revealed}</span>
                        ) : (
                          <span className="leg-hidden">·</span>
                        )}
                      </li>
                    ))}
                  </ol>
                )}
              </div>

              {view.detectives.map((d) => {
                const n = d.id.kind === "detective" ? d.id.n : 0;
                return (
                  <div key={n} className={`player-card ${view.current_player === n ? "active" : ""}`}>
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
                  </div>
                );
              })}
            </div>

            <div className="board-stage">
              <MapBoard
                board={board}
                showLabels={showLabels}
                showEdges={showEdges}
                tokens={buildTokens(view, pendingFirst)}
                highlight={legalMoves ? legalTargets() : undefined}
                onStationClick={legalMoves ? handleStationClick : undefined}
              />

              {/* Move wheel — ticket options ringed around the target station. */}
              {pendingDest &&
                (() => {
                  const pos = stationPos.get(pendingDest.to);
                  if (!pos) return null;
                  const usable = new Set(ticketsFor(pendingDest.to));
                  const types = isMrXTurn ? MRX_TICKET_TYPES : DETECTIVE_TICKET_TYPES;
                  // The ring is the ticket icons plus, for Mr X, a double-move
                  // toggle. On the second leg it stays (lit) so toggling it off
                  // cancels the committed intermediate move.
                  const showDouble = isMrXTurn && doublesAvailable;
                  const slots = showDouble ? types.length + 1 : types.length;
                  const ring = (i: number) =>
                    `translate(-50%, -50%) rotate(${(360 / slots) * i}deg) translateY(-44px) rotate(${-(360 / slots) * i}deg)`;
                  return (
                    <div
                      className="move-wheel"
                      style={{
                        left: `${(pos.x / board.image.w) * 100}%`,
                        top: `${(pos.y / board.image.h) * 100}%`,
                      }}
                    >
                      {types.map((t, i) => {
                        const face = ticketFace(t);
                        return (
                          <button
                            key={t}
                            className={`wheel-ticket ${face.cls}`}
                            style={{ transform: ring(i) }}
                            disabled={!usable.has(t) || submitting}
                            onClick={() => act(pendingDest.to, t)}
                            title={t}
                          >
                            {face.node}
                          </button>
                        );
                      })}

                      {showDouble && (
                        <button
                          className={`wheel-ticket wheel-double ${effectiveMode === "double" ? "active" : ""}`}
                          style={{ transform: ring(types.length) }}
                          onClick={() => {
                            if (pendingFirst) {
                              // Toggling off on the 2nd leg cancels the intermediate stop.
                              clearPending();
                              setMode("single");
                            } else {
                              setMenuMode(effectiveMode === "double" ? "single" : "double");
                            }
                          }}
                          title={pendingFirst ? "Cancel intermediate move" : "Double move"}
                        >
                          <span className="t-icon">2×</span>
                        </button>
                      )}

                      <button className="wheel-hub" onClick={() => setPendingDest(null)} title="Cancel">
                        {pendingDest.to}
                      </button>

                      {effectiveMode === "double" && (
                        <span className="wheel-note">{pendingFirst ? "2nd stop" : "double — pick 1st leg"}</span>
                      )}
                    </div>
                  );
                })()}

              {/* No legal moves available. */}
              {!view.is_terminal && legalMoves?.can_pass && !pendingDest && (
                <div className="board-toast">
                  No legal moves for {currentName(view.current_player)}.
                  <button className="primary" onClick={() => submit({ kind: "pass" })}>
                    Pass
                  </button>
                </div>
              )}

              {!view.is_terminal && !legalMoves && <div className="board-toast">Loading moves…</div>}
              {submitting && <div className="board-toast">Submitting…</div>}
            </div>
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

function buildTokens(view: ViewDto, pendingFirst: StepDto | null): TokenSpec[] {
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
    // Mid double move: leave a faded Mr X at the origin and project a tentative
    // one at the committed first leg (the "current" highlight follows him there).
    const midDouble = pendingFirst != null;
    tokens.push({
      station: mrx.station,
      label: "X",
      color: MRX_COLOR,
      current: view.current_player === 0 && !midDouble,
      ghost: false,
      faded: midDouble,
    });
    if (pendingFirst) {
      tokens.push({
        station: pendingFirst.to,
        label: "X",
        color: MRX_COLOR,
        current: true,
        ghost: false,
        pending: true,
      });
    }
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
