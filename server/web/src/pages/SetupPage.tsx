import { useState } from "react";
import { createGame, type GameStateDto, type NewGameRequest } from "../api/games";

interface Props {
  onCreated: (game: GameStateDto) => void;
}

const MIN_STATION = 1;
const MAX_STATION = 199;

export default function SetupPage({ onCreated }: Props) {
  const [detectives, setDetectives] = useState(3);
  const [mode, setMode] = useState<"random" | "manual">("random");
  const [seed, setSeed] = useState("");
  const [mrXStart, setMrXStart] = useState("");
  const [detStarts, setDetStarts] = useState<string[]>(() => Array(3).fill(""));
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function changeDetectives(n: number) {
    setDetectives(n);
    setDetStarts((prev) => {
      const next = prev.slice(0, n);
      while (next.length < n) next.push("");
      return next;
    });
  }

  function buildRequest(): NewGameRequest {
    if (mode === "random") {
      const req: NewGameRequest = { detectives };
      if (seed.trim() !== "") {
        const s = Number(seed);
        if (!Number.isInteger(s) || s < 0) throw new Error("Seed must be a non-negative integer");
        req.seed = s;
      }
      return req;
    }

    const mx = parseStation(mrXStart, "Mr X start");
    const starts = detStarts.map((v, i) => parseStation(v, `Detective ${i + 1} start`));
    const all = [mx, ...starts];
    if (new Set(all).size !== all.length) {
      throw new Error("Start stations must all be distinct");
    }
    return { detectives, mr_x_start: mx, detective_starts: starts };
  }

  async function submit() {
    setError(null);
    let req: NewGameRequest;
    try {
      req = buildRequest();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      return;
    }

    setSubmitting(true);
    try {
      onCreated(await createGame(req));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="setup">
      <h1>New Game</h1>

      <div className="field">
        <label htmlFor="detectives">Detectives</label>
        <select
          id="detectives"
          value={detectives}
          onChange={(e) => changeDetectives(Number(e.target.value))}
        >
          {[1, 2, 3, 4, 5].map((n) => (
            <option key={n} value={n}>
              {n}
            </option>
          ))}
        </select>
      </div>

      <div className="field">
        <span className="field-label">Start positions</span>
        <div className="radio-row">
          <label>
            <input
              type="radio"
              name="mode"
              checked={mode === "random"}
              onChange={() => setMode("random")}
            />
            Random
          </label>
          <label>
            <input
              type="radio"
              name="mode"
              checked={mode === "manual"}
              onChange={() => setMode("manual")}
            />
            Manual
          </label>
        </div>
      </div>

      {mode === "random" && (
        <div className="field">
          <label htmlFor="seed">Seed (optional)</label>
          <input
            id="seed"
            type="number"
            min={0}
            placeholder="random each time"
            value={seed}
            onChange={(e) => setSeed(e.target.value)}
          />
        </div>
      )}

      {mode === "manual" && (
        <div className="manual-starts">
          <div className="field">
            <label htmlFor="mrx">Mr X start</label>
            <input
              id="mrx"
              type="number"
              min={MIN_STATION}
              max={MAX_STATION}
              value={mrXStart}
              onChange={(e) => setMrXStart(e.target.value)}
            />
          </div>
          {detStarts.map((v, i) => (
            <div className="field" key={i}>
              <label htmlFor={`det-${i}`}>Detective {i + 1} start</label>
              <input
                id={`det-${i}`}
                type="number"
                min={MIN_STATION}
                max={MAX_STATION}
                value={v}
                onChange={(e) =>
                  setDetStarts((prev) => prev.map((x, j) => (j === i ? e.target.value : x)))
                }
              />
            </div>
          ))}
        </div>
      )}

      <p className="hint">
        Standard tickets (Mr X 4/3/3 + 5 black + 2 double; detectives 10/8/4) and
        standard reveals (legs 3, 8, 13, 18, 24).
      </p>

      {error && <div className="form-error">{error}</div>}

      <button className="primary" onClick={submit} disabled={submitting}>
        {submitting ? "Starting…" : "Start game"}
      </button>
    </div>
  );
}

function parseStation(value: string, label: string): number {
  const n = Number(value);
  if (value.trim() === "" || !Number.isInteger(n) || n < MIN_STATION || n > MAX_STATION) {
    throw new Error(`${label} must be a station between ${MIN_STATION} and ${MAX_STATION}`);
  }
  return n;
}
