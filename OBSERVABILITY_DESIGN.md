# Partial-Observability Design

Plan for the information layer that turns the current fully-observable engine into
a substrate for determinized / information-set MCTS (ISMCTS). Addresses review
items #12 (observation type + reveal schedule), #13 (history recording), #14
(determinizer), and resolves #4 (double-move travel-log counting).

---

## 1. Guiding principle: ground truth stays lean, information composes on top

`GameState` remains the **fully-observable ground truth** and the **MCTS rollout
substrate**. It does *not* gain a history `Vec`. This is deliberate:

- Rollouts hammer `apply_action` millions of times. A growing per-node `Vec`
  would defeat the fixed-array / `Copy` optimization the review wants (#8) and
  add heap churn to the hot path.
- In determinized MCTS the sampled world is treated as fully observable for the
  rollout, so rollouts never consult history.

The information layer is therefore a **projection on top of ground truth**, only
maintained at the real-game / decision-root level — exactly the layering the
review calls correct ("keep `GameState` as ground truth and add observation as a
projection").

```
                 observe(viewer)              determinize(rng)
  GameState  ───────────────────▶  Observation  ───────────────────▶  GameState
 (ground truth)                  (what a player knows)              (a sampled world
                                                                     consistent with it)
        ▲                                                                    │
        └──────────────────────  apply_action (pure)  ◀──────────────────────┘
                              rollouts run here, on ground truth
```

---

## 2. The MrX travel-log model (resolves #4)

MrX takes one *round-move* per rotation, but a **double move is two legs**. The
reveal schedule and the move limit count **legs**, not rotations. We store one
log entry per leg, so a double naturally contributes two entries and the
double-move-counting bug (#4) disappears without touching turn rotation.

```rust
/// One leg of MrX's journey, exactly as the detectives observe it.
/// `ticket` is the ticket SPENT — `Black` when concealed, never the underlying
/// edge transport. That concealment is the entire point of the hidden game.
pub struct MrXMove {
    pub ticket: TicketType,           // public: Taxi | Bus | Underground | Black
    pub revealed: Option<StationId>,  // Some only on reveal legs
}

/// The public record of the game. This is the authoritative source for both the
/// observation projection and the determinizer's belief set.
pub struct GameHistory {
    pub mr_x_log: Vec<MrXMove>,        // one entry per leg MrX has played
    pub mr_x_start: Option<StationId>, // hidden in standard SY; Some only if a rule reveals it
}
```

This **replaces** the current stub
(`mr_x_revealed_positions: Vec<Option<u8>>`, `mr_x_actions: Vec<Action>`).
The old `mr_x_actions: Vec<Action>` stored full destinations — that is ground
truth, not an observation, and would leak MrX's position. We keep only the public
ticket plus the conditional reveal. (A separate ground-truth action log can be
added for replay/debugging, but it is not part of the observation layer.)

---

## 3. Reveal schedule

Reveal legs are configuration, not magic constants buried in logic.

```rust
pub struct RevealSchedule { legs: Vec<usize> }   // 1-based MrX leg numbers

pub const STANDARD_REVEALS: [usize; 5] = [3, 8, 13, 18, 24];

impl RevealSchedule {
    pub fn is_reveal_leg(&self, leg: usize) -> bool { self.legs.contains(&leg) }
}
```

Note the mismatch to reconcile during implementation: the engine currently uses
`max_turns = 22` while standard Scotland Yard is a 24-move game with reveals at
{3, 8, 13, 18, 24}. The schedule and the move limit should both be expressed in
**MrX legs** and configured together. **Decision flagged (ties into #4):** switch
the turn-limit check to compare MrX's leg count (`mr_x_log.len()`) against the
move limit, instead of `turn_number`, so a double move correctly consumes two of
MrX's allotted moves.

---

## 4. Observation type (#12)

`observe` projects ground truth to what a given viewer legitimately knows.

```rust
pub enum Observation {
    /// MrX sees the whole board, so his observation is just the ground truth.
    MrX(GameState),
    Detective(DetectiveObservation),
}

pub struct DetectiveObservation {
    pub board: Arc<Board>,
    pub detectives: Vec<PlayerState>,   // public: positions + ticket counts
    pub mr_x_log: Vec<MrXMove>,         // ticket history + reveals
    pub mr_x_tickets: TicketInventory,  // = start loadout − tickets spent (derivable, known)
    pub current_player: usize,
    pub turn_number: usize,
    pub reveal: RevealSchedule,
    pub max_legs: usize,
}

pub fn observe(state: &GameState, history: &GameHistory, viewer: PlayerId) -> Observation;
```

Key facts encoded:
- Detectives see each other's exact positions and ticket counts (public in SY).
- Detectives know MrX's **ticket counts** — they are the fixed start loadout minus
  the tickets recorded in `mr_x_log`, so they are derivable, not secret. Mr X gets the tickets
  used by the detectives, which must also be accounted for. 
- Detectives see MrX's **station only on reveal legs** (the last `Some(..)` in the
  log); between reveals his position is inferred, not observed.

---

## 5. Determinizer (#14)

Given a detective observation, sample one concrete `GameState` consistent with it.

```rust
pub fn determinize(obs: &DetectiveObservation, rng: &mut impl Rng) -> GameState;
```

### 5.1 Belief set (reachable-set propagation)

The heart of the determinizer. Compute the set of stations MrX could occupy now.

```
anchor  = last reveal in mr_x_log, or the (possibly unknown) start set
belief  = { anchor }                 // or all stations if start is unknown

for each leg AFTER the anchor, in order:
    next = {}
    for s in belief:
        for (n, edge_ticket) in board.neighbors(s):
            allowed = leg.ticket == Black          // black rides any edge
                   || edge_ticket == leg.ticket    // else transport must match
            if allowed { next.insert(n) }
    if leg.revealed is Some(r):                    // a reveal collapses belief
        next = next ∩ { r }
    belief = next − detective_occupied_stations    // MrX cannot sit on a detective
```

Properties:
- A **Black** leg fans the belief out across *all* transport types — this is why
  fixing #2 (black-on-any-edge) was a prerequisite; without it the inference
  problem is degenerate.
- A **reveal** leg collapses belief to a singleton, then it re-expands.
- **Double moves** are simply two consecutive legs in the log; the loop handles
  them with no special case.

Refinement (Phase 5): subtracting only *current* detective positions is a sound
over-approximation. Subtracting each leg's contemporaneous detective positions
tightens the set but requires logging detective positions per round (see §6).

### 5.2 Sampling and construction

1. Sample one station from `belief` (uniform first; weighted/particle later).
2. MrX inventory = start loadout − tickets spent (deterministic from the log).
3. Build a `GameState` via `GameState::new` with MrX at the sampled station and
   detectives as observed. The constructor's terminal checks (now run at
   construction — review #2 fix) guarantee the sampled state is never a
   zero-action non-terminal state.

Consistency invariants every determinized state must satisfy (assert in tests):
- sampled station ∈ belief set,
- MrX not co-located with a detective,
- every MrX ticket count ≥ 0.

---

## 6. History recording integration (#13)

Recording happens in a thin top-level wrapper, **not** in `apply_action`.

```rust
pub struct Game {
    pub state: GameState,
    pub history: GameHistory,
    pub reveal: RevealSchedule,
}

impl Game {
    pub fn apply(&mut self, action: Action) {
        if matches!(self.state.players[self.state.current_player].id, PlayerId::MrX) {
            // Push one MrXMove per leg, computing `revealed` from the leg index
            // (mr_x_log.len() after each push) against self.reveal.
            self.record_mr_x(action);
        }
        self.state = apply_action(&self.state, action); // pure, history-free
    }
}
```

Rationale for the wrapper over "record inside `apply_action`" (a deliberate
deviation from the review's wording): `apply_action` is the rollout hot path and
must stay pure and allocation-light. Rollouts never need history, so threading a
growing log through every simulated node would cost throughput for no benefit.
The authoritative public log only needs to advance on *real* moves, which all go
through `Game::apply`.

This is the retrofit the review wants done early (#13) — cheap now, painful later,
because the determinizer and reveal alignment both hang off it.

---

## 7. ISMCTS loop (how it all comes together)

At a detective decision point:

```
obs = observe(real_state, history, Detective(i))
for _ in 0..num_determinizations:
    world = determinize(obs, rng)          // a sampled ground-truth GameState
    run MCTS on `world` using apply_action // fully-observable rollouts, unchanged
aggregate visit counts / values across worlds → choose action
```

The existing pure engine (`legal_actions`, `apply_action`) is reused verbatim
inside each determinized world. The information layer is purely additive.

---

## 8. Build order

1. **`GameHistory` + `MrXMove` + `RevealSchedule` + `Game` wrapper recording**
   (#13). Replace the dead stub. Decide leg-counted move limit (#4) here.
2. **`Observation` + `observe()`** (#12).
3. **Determinizer + belief-set propagation** (#14), uniform sampling.
4. **ISMCTS driver** wiring determinize → existing rollouts.
5. **Refinements**: per-round detective-position logging for tighter belief sets;
   weighted / particle-filter determinization.

---

## 9. Test plan

- **Belief set** after a hand-built reveal + ticket sequence equals a hand-computed
  set.
- **Reveal collapse**: a reveal leg reduces the belief set to the revealed singleton.
- **Black expansion**: a Black leg expands belief across all transport types from
  each frontier station.
- **Double move** pushes two `MrXMove` entries; a reveal on the intermediate leg
  reveals the intermediate station.
- **Determinized-state consistency**: sampled station ∈ belief, MrX not on a
  detective, inventories ≥ 0, state non-terminal-with-moves.
- **`observe`**: MrX observation is full; detective observation hides MrX's station
  between reveals and exposes it on reveal legs.
- **Leg-counted limit** (if adopted): a double move advances MrX's move count by 2.
