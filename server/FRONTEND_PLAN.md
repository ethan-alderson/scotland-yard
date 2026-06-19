# Scotland Yard — Web App Plan (server + React frontend)

A plan to build a playable Scotland Yard web app on top of the existing Rust
engine. Designed to be implemented **phase by phase**, where each phase is
independently runnable and testable so you can review progress before moving on.

> **You are overseeing, not implementing.** Each phase below states what gets
> built, how to verify it works, and the "done when" bar. Implementation detail
> (actual Rust/TS code) is intentionally omitted — only contracts (endpoints,
> JSON shapes, layout) are pinned down so the pieces fit together.

---

## 0. Locked decisions

| Decision | Choice | Why |
|---|---|---|
| **Play mode** | **Pass-and-play (hotseat)** in one browser | No AI exists; all sides are human. Simplest correct thing. |
| **Debug view** | A **God-view toggle** available at any time | Reveals every position/ticket for debugging, on top of normal hidden-info play. |
| **Hidden info** | Enforced **server-side** via the existing `observe()` layer | A detective's view endpoint literally never contains Mr X's secret position, so it can't leak through the client. |
| **Server** | `axum` REST + in-memory game store | Already the server's stack (axum 0.8 + tokio). No websockets needed for hotseat. |
| **Frontend** | **Vite + React + TypeScript**, **Tailwind** for styling | Standard, fast dev loop, lives under `server/web/`. |
| **Map rendering** | `map.png` as a background with an **SVG overlay** (`viewBox="0 0 2570 1926"`) | Station coords from `pos.txt` are used 1:1 with no manual scaling math; the SVG scales responsively. |

If any of these should change (e.g. you later want real online multiplayer),
that's a re-plan of the server spine — flag it before Phase 1.

---

## 1. Assets & engine facts (confirmed)

- **`map.png`** — 2570 × 1926 RGBA, ~12 MB. Native pixel space is the coordinate
  system for everything.
- **`pos.txt`** — first line is the count (`199`), then `id x y` per station, in
  `map.png` pixel coordinates. Ranges: x ∈ [67, 2530], y ∈ [37, 1873].
- **`engine/connections.txt`** — edge list `a b ticket`, where ticket ∈
  {`taxi`, `bus`, `underground`, `water`}. `water` maps to the engine's
  `TicketType::Black` (ferry). Counts: 346 taxi, 99 bus, 20 underground, 3 water.
- **Engine API the server will call** (all in the `engine` crate):
  - `Board::from_connections_file(path) -> Board`
  - `Game::new(state, reveal)`, `game.apply(action)`, `game.observe(viewer) -> Observation`
  - `rules::legal_actions(&state) -> Vec<Action>`, `rules::apply_action(...)`
  - Types: `PlayerId`, `PlayerState`, `TicketInventory`, `TicketType`, `StationId`,
    `Step`, `Action::{Single,Double,Pass}`, `Winner`, `Observation::{MrX,Detective}`,
    `DetectiveObservation`.
- **Serialization gotcha:** `GameState` is **not** `Serialize` (it holds
  `Arc<Board>`). The server must **own its own API DTOs** and convert engine
  types → DTOs at the boundary. Do **not** try to derive `Serialize` on
  `GameState`. (Leaf types like `PlayerState`, `TicketType`, `StationId` already
  derive serde and can be reused inside DTOs.)

---

## 2. Directory layout (under `server/`)

```
server/
  Cargo.toml
  FRONTEND_PLAN.md        ← this file
  assets/
    map.png               ← copied/moved here so the server can serve it
    pos.txt               ← station pixel coordinates
  src/
    main.rs               ← axum app wiring
    state.rs              ← GameStore (RwLock<HashMap<GameId, Game>>) + app state
    dto.rs                ← API request/response types (serde)
    board_geometry.rs     ← loads pos.txt + connections into a /api/board payload
    routes/
      games.rs            ← create / get / legal moves / apply move / view
      board.rs            ← board geometry endpoint
  web/                    ← React app (Vite)
    index.html
    package.json
    vite.config.ts        ← dev proxy: /api -> http://127.0.0.1:3000
    src/
      api/                ← typed fetch client mirroring dto.rs
      components/         ← MapBoard, StationLayer, TokenLayer, TicketPanel, MoveControls, ...
      pages/              ← Setup, Play
      state/              ← game/session store (Zustand or React context)
```

> Asset note: `map.png` is 12 MB. For dev it's fine. Before any "production"
> phase, consider committing a downscaled copy (e.g. ~1400px wide) for faster
> loads; the SVG overlay is resolution-independent so a smaller image won't
> affect coordinate math.

---

## 3. API contract (target — built across Phases 1–2)

All JSON. `:id` is a game id (uuid string). Ticket strings are
`"taxi" | "bus" | "underground" | "black"`.

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/api/games` | Create a new game from a setup config; returns `game_id` + initial public state. |
| `GET`  | `/api/games/:id` | God-view full state (used by debug toggle). |
| `GET`  | `/api/games/:id/view?as=mrx` \| `?as=detective&n=1` | Perspective-filtered state via `observe()`. Detective views omit Mr X's live position. |
| `GET`  | `/api/games/:id/legal_moves` | Legal actions for the **current** player, shaped for the UI (grouped by destination + ticket). |
| `POST` | `/api/games/:id/moves` | Apply one action (single/double/pass). Returns the new state from the mover's perspective. |
| `GET`  | `/api/board` | Static board geometry: stations `[{id,x,y}]`, edges `[{from,to,ticket}]`, `{image:{w,h,url}}`. |
| `GET`  | `/assets/map.png` | The map image (served statically). |

**Representative shapes** (final field names settled in Phase 1):

```jsonc
// POST /api/games  (request)
{
  "detectives": 4,
  "mr_x_start": null,            // null = deal randomly from standard start cards
  "detective_starts": null,     // null = deal randomly
  "tickets": "standard",        // preset; or an explicit loadout object
  "reveal": "standard"          // STANDARD_REVEALS = [3,8,13,18,24]
}

// GET /api/games/:id  (god view response)
{
  "game_id": "…",
  "current_player": 0,
  "turn_number": 2,
  "max_turns": 22,
  "is_terminal": false,
  "winner": null,
  "players": [
    {"id": {"kind":"mrx"},        "station": 45, "tickets": {"taxi":4,"bus":3,"underground":3,"black":5,"double":2}},
    {"id": {"kind":"detective","n":1}, "station": 13, "tickets": {"taxi":10,"bus":8,"underground":4,"black":0,"double":0}}
  ],
  "mr_x_log": [
    {"ticket":"taxi","revealed":null},
    {"ticket":"bus","revealed":45}    // station shown only on reveal legs
  ]
}

// GET /api/games/:id/view?as=detective&n=1  (detective view: Mr X position hidden)
{
  "viewer": {"kind":"detective","n":1},
  "current_player": 0,
  "turn_number": 2,
  "detectives": [ /* full PlayerState for each detective */ ],
  "mr_x": { "tickets": {…}, "last_revealed_station": 45, "log": [ … ] },  // no live station
  "is_terminal": false, "winner": null
}

// GET /api/games/:id/legal_moves  (UI-friendly)
{
  "player": {"kind":"mrx"},
  "can_pass": false,
  "moves": [
    {"to": 46, "tickets": ["taxi","black"]},     // pick-a-ticket when ambiguous
    {"to": 58, "tickets": ["bus"]}
  ],
  "double": {                                     // present only for Mr X w/ a double ticket
    "available": true,
    "first_steps": [ {"to":46,"tickets":["taxi"]}, … ]
  }
}

// POST /api/games/:id/moves  (request — one of)
{"single": {"to": 46, "ticket": "taxi"}}
{"double": [{"to":46,"ticket":"taxi"}, {"to":58,"ticket":"bus"}]}
{"pass": true}
```

---

## 4. The pass-and-play + debug model (the important UX rule)

- The store holds **one `Game`** (ground truth). The client never holds secrets.
- **Normal play:** the active screen shows the **current player's legitimate
  view**, fetched from `/view?as=…`:
  - Mr X's turn → full board (his own perspective).
  - A detective's turn → `DetectiveObservation`: detective tokens shown exactly;
    Mr X shown only at his **last revealed** station (a "?" / ghost marker
    otherwise).
- **Handoff interstitial:** between turns, a "Pass the device to **Detective 2**"
  screen gates the next view so positions aren't glimpsed by the wrong player.
- **Debug / God view:** a clearly-labeled toggle (distinct color/banner) that
  swaps the data source to `GET /api/games/:id` (full truth) **without** changing
  whose turn it is. It's for inspection only; turning it off returns to the
  legitimate view. This is the "best of both worlds" switch.

---

## 5. Phases

Each phase: **Goal → Deliverables → How to test → Done when.** Phases 1–2 are
backend, 3+ are frontend; the frontend can start against a couple of stub
endpoints if you'd rather interleave.

### Phase 1 — Server core: game store + REST lifecycle
- **Goal:** Drive a full game over HTTP with no UI.
- **Deliverables:** `GameStore` (in-memory, `RwLock<HashMap>`), `dto.rs`,
  endpoints `POST /api/games`, `GET /api/games/:id`, `GET …/legal_moves`,
  `POST …/moves`. Standard board loaded from `engine/connections.txt` at startup.
  Standard ticket/reveal presets.
- **How to test:** a `curl` script (committed as `server/scripts/smoke.sh`):
  create a game → read state → read legal moves → POST a legal move → confirm
  `turn_number`/positions advanced → drive to a terminal state and see a `winner`.
- **Done when:** the script plays a game start-to-finish and the JSON is
  internally consistent (legal_moves ⊆ what the engine accepts; illegal move →
  4xx with a clear error).

### Phase 2 — Server: board geometry + map asset
- **Goal:** Everything the frontend needs to draw the board.
- **Deliverables:** `GET /api/board` (stations from `pos.txt`, edges from
  `connections.txt`, image meta), `map.png` + `pos.txt` moved to `server/assets/`,
  static serving at `/assets/*`.
- **How to test:** `curl /api/board | jq '.stations | length'` → `199`; spot-check
  a few station coords against `pos.txt`; open `/assets/map.png` in a browser.
- **Done when:** board payload has 199 stations and all edges, and the image
  loads.

NOTE - I MOVED A COPY OF CONNECTIONS.TXT INTO SERVER, WE CAN ADD THAT TO WHEREVER ASSETS END UP BEING STORED

### Phase 3 — Frontend scaffold + map + station calibration
- **Goal:** The map renders with a correctly-aligned, clickable station overlay.
- **Deliverables:** Vite/React/TS app, dev proxy to axum, `MapBoard` (img +
  SVG `viewBox="0 0 2570 1926"`), `StationLayer` drawing a dot per station from
  `/api/board`, a **calibration toggle** that labels each dot with its id.
- **How to test:** **visual** — dots sit on the map's printed circles; toggling
  ids on shows them at the right stations. (This is the make-or-break alignment
  check; do it before building anything that depends on station positions.)
- **Done when:** every dot visually lands on its station across the whole board,
  responsive to window resize.

### Phase 4 — Frontend: new-game setup
- **Goal:** Start a game from the UI.
- **Deliverables:** a Setup page (# detectives, ticket preset = standard,
  reveal = standard, random vs manual start positions) that `POST`s `/api/games`
  and routes into the Play page.
- **How to test:** start a game; pieces appear at dealt start stations matching
  the `POST` response.
- **Done when:** a game can be created and you land on the board with tokens
  placed.

### Phase 5 — Frontend: render game state + perspective/debug toggle
- **Goal:** See the live game truthfully, with hidden info handled.
- **Deliverables:** `TokenLayer` (detective + Mr X tokens on stations),
  `TicketPanel` per player, turn indicator, **Mr X travel log** (ticket icons +
  reveal markers), winner banner. **Perspective selector** wired to `/view?as=…`
  and the **God-view debug toggle** wired to `/api/games/:id` (§4).
- **How to test:** in detective perspective Mr X is hidden except at reveal
  stations; flip debug on → his true station appears; flip off → hidden again.
  Cross-check debug view against the engine `curl` state from Phase 1.
- **Done when:** all three view modes render correctly and the debug toggle never
  alters whose turn it is.

### Phase 6 — Frontend: move interaction
- **Goal:** Make legal moves by clicking.
- **Deliverables:** click current player → highlight legal destinations from
  `/legal_moves` → click a destination → if multiple tickets, a small picker
  (incl. **Black** for Mr X); **double-move** flow for Mr X (pick first leg, then
  second); **Pass** button when a detective is stuck. Submit → refetch view.
- **How to test:** play a complete **hotseat** game to a terminal state (capture,
  cornered, and turn-limit endings all reachable); illegal clicks are simply not
  offered.
- **Done when:** a full game is playable end-to-end purely through the UI.

### Phase 7 — Pass-and-play handoff + reveal polish + endgame
- **Goal:** Make hotseat actually pleasant and leak-free.
- **Deliverables:** handoff interstitial between turns (§4), reveal-leg emphasis
  (flash Mr X when a reveal happens), "last known position" ghost marker for Mr X
  between reveals, end-game screen with winner + "New game", basic ticket-count
  warnings.
- **How to test:** play a 2–3 person hotseat game; confirm a detective never sees
  Mr X's live position during their turn, and reveal legs surface him as expected.
- **Done when:** a non-technical person can sit down and play a clean hidden-info
  game.

### Phase 8 (optional) — Single-binary production build
- **Goal:** `cargo run` serves the built frontend too.
- **Deliverables:** `npm run build` → axum serves `web/dist` as static files with
  an SPA fallback; documented run steps; downscaled `map.png` for load speed.
- **How to test:** fresh build, run the binary, play a game with no Vite dev
  server running.
- **Done when:** one command serves API + UI.

---

## 6. Reference: standard config defaults (confirm before Phase 1)

These are the conventional Scotland Yard values; the engine takes them as plain
parameters, so they're trivial to change. **Please confirm or correct:**

- **Detective tickets (each):** 10 taxi, 8 bus, 4 underground, 0 black, 0 double.
- **Mr X tickets:** 4 taxi, 3 bus, 3 underground, 5 black, 2 double.
- **Reveal legs:** `STANDARD_REVEALS = [3, 8, 13, 18, 24]` (already in engine).
- **Move/turn limit:** engine currently uses `max_turns = 22`. ⚠️ This is
  **counted in rounds, not Mr X legs**, and the observability design flagged that
  a double move should consume two of Mr X's moves. We deliberately did **not**
  change that rule yet. For the UI we'll display the limit as-is; if you want the
  24-leg standard, that's a small engine change to schedule separately.
- **Start positions:** deal random distinct stations from the standard SY start
  cards (detective set and Mr X set). For v1, random-distinct-from-all-stations is
  acceptable if you'd rather not encode the start cards yet.

---

## 7. Risks / watch-items for oversight

1. **Coordinate alignment (Phase 3)** is the single highest-risk item. Insist on
   the visual calibration pass before anything depends on station positions.
2. **DTO boundary discipline** — keep engine types out of the wire format except
   the serde leaf types; the server owns the JSON shape. Prevents `Arc<Board>`
   serialization headaches and decouples UI from engine internals.
3. **Hidden info must be server-enforced**, not hidden in CSS. The detective view
   endpoint should not even contain Mr X's live station. Verify by inspecting the
   raw `/view?as=detective` JSON.
4. **One mutable game, many reads** — guard the store with a lock; a move is a
   read-modify-write. Fine for hotseat; revisit if multiplayer ever returns.
5. **Map size** — 12 MB image will feel heavy; downscale before Phase 8.

---

## 8. How you'll review each phase

For every phase Opus should hand you: (a) the exact command(s) to run it, (b) the
test from the phase's "How to test," and (c) for frontend phases, a screenshot or
a short description of what to click. You approve → next phase. Backend phases are
verifiable by `curl`/`jq` without touching the UI, so they can be signed off fast.
```
