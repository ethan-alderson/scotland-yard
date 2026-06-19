# Scotland Yard — Web Frontend

React + TypeScript + Vite. Phase 3 renders the board: the map image with a
station overlay (and an optional edge overlay) for the coordinate-calibration
check.

## Prerequisites

- The Rust API server (in `../`, the `server` crate).
- **Node.js 18+ installed inside the same environment as this repo.**

> ⚠️ **WSL users:** this repo lives on the Linux filesystem (`/home/...`). You
> must use a **Linux** Node, not a Windows one. Windows `npm` cannot install
> here — it chokes on the `\\wsl.localhost\...` UNC path (`esbuild` install
> fails). Install Node inside WSL, e.g.:
> ```bash
> curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
> # restart the shell, then:
> nvm install --lts
> ```
> Verify you're on Linux Node: `which node` should be under your home dir
> (e.g. `~/.nvm/...`), **not** `/mnt/c/...`.

## Running (two terminals)

**Terminal 1 — API server** (from the repo root):
```bash
cargo run -p server
# -> Scotland Yard API listening on http://127.0.0.1:3000
```

**Terminal 2 — frontend dev server** (from `server/web`):
```bash
npm install        # first time only
npm run dev
# -> Vite dev server on http://localhost:5173
```

Vite proxies `/api` and `/assets` to the API server on `:3000`, so the browser
only talks to one origin (no CORS).

## Testing it in the browser

1. Open **http://localhost:5173**.
2. The map should fill the page, with blue station dots overlaid and their
   **station IDs** shown (calibration view is on by default).
3. **Alignment check (the Phase 3 goal):** every numbered dot should sit on its
   printed circle on the map. Pan/zoom your browser; the SVG scales with the
   window and dots stay aligned.
4. Toggle **Show edges** to draw colored connection lines (yellow = taxi,
   green = bus, red = underground, black = ferry). They should trace the routes
   printed on the board.
5. Click any station — its id appears in the top-right readout (wiring for the
   move UI in a later phase).
6. Toggle **Show station IDs** off for a clean map.

If dots are systematically offset, that points to a coordinate issue in
`pos.txt` / `/api/board`, not the rendering — flag it before later phases build
on station positions.

## Useful commands

- `npm run dev` — dev server with hot reload
- `npm run typecheck` — TypeScript check, no emit
- `npm run build` — typecheck + production build to `dist/`
- `npm run preview` — serve the production build locally


UI CHANGES:

Make the ticket usage option a menu that appears once you click a station. The menu should always appear even if the station only has one transit option, as a way to force the player to confirm their move. It should show all ticket types but with the unusable choices grayed out. Mr X should also use this menu to confirm the double move. Overlaying the move menu on the board will let us make the board bigger and centralize the players strategization to the board itself, removing the need to constantly scroll around.

We need to organize the screen better for such a large board, the player profiles and their inventories are taking up a large portion of the righthand side, making the board larger by putting them in a bar above it would be helpful for interactivity.

We need to know the function of a station when a player is on it, right now the player icon covers the station on the board so we can't see what the actual station that they're on is. 



