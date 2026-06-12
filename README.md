TO DO

If a detective moves onto a position with mrx the gamestate is terminal and the winner is a detective.

Double tickets do not have a price, there's no counter for how many MrX gets

Design full testing suite for rules, board, and gamestate - DONE FOR EXISTING FEATURES

Replace current player as usize with current player as playerId

FUTURE TESTS:

Detectives win by capture
Mr X wins if detectives immobilized
Turn rotation
Reveal rounds
Black ticket behavior
Double move ticket consumption
Ticket transfer from detective to Mr X


<!-- UI Plan: -->
<!-- 
yoink the board and station coords from https://github.com/tim-koehler/ScotlandYard

Build front end layer in react on top of rust API back end

1. Add axum and tokio to server/Cargo.toml and get a literal "hello world" GET / route compiling
2. Add Axum State with a dummy struct — just to see how state threading works
3. Wire in your real GameState behind Arc<Mutex<...>>
4. Add routes one at a time, starting with GET /game (read-only, simpler)
5. Add POST /game/move last (write path, needs request body deserialization) -->

1. Double Move Ticket (TicketInventory + legal_actions + apply_action)

TicketInventory has no double field. Currently legal_actions generates Action::Double whenever MrX has enough transport tickets — there's no cost for the double move itself.

Changes:
- Add double: u8 to TicketInventory (alongside taxi, bus, underground, black)
- In legal_actions: gate the entire double-move branch on curr_player.tickets.double > 0
- In apply_action for Action::Double: spend one double ticket from MrX's inventory on top of the two transport tickets already spent by the two apply_step calls

---
2. Detective Ticket Transfer to MrX (apply_step)

When a detective moves, their spent ticket is currently just subtracted and lost. It should be given to MrX.

Changes:
- In apply_step, after deducting the ticket from the moving player, check if curr_player.id is a Detective
- If so, find MrX in new_players and call add_ticket(step.ticket) on his inventory
- Add an add_ticket method to TicketInventory (the inverse of spend_ticket)
- Black tickets used by MrX are discarded (not given to anyone) — this should be the default for MrX moves already, but worth making explicit

---
3. Turn Rotation (apply_action)

current_player and turn_number never change.

Changes:
- At the end of apply_action, after the state is built, advance current_player = (current_player + 1) % players.len()
- When current_player wraps back to 0 (MrX), increment turn_number
- Note: for Action::Double, the intermediate apply_step should not advance the turn — only the outer apply_action call should. This is already structurally true since apply_step is private and apply_action is what the caller uses

---
4. Terminal: Detectives Catch MrX

After each detective's move, if any detective occupies MrX's station, the game ends. The winner field is Option<PlayerId> but there's no way to express "detectives win as a team" — PlayerId::Detective(n) refers to a specific detective.

Changes:
- Add a Detectives variant to PlayerId to represent a team win (used only in winner, not as a player identity)
- In apply_action, after the new state is built (and turn is advanced), check if any detective's station equals MrX's station
- If so: set is_terminal = true, winner = Some(PlayerId::Detectives)

---
5. Terminal: MrX Has No Legal Moves (Detectives Win)

If MrX is cornered — no tickets or all neighbors blocked — the detectives win.

Changes:
- In apply_action, after turn rotation brings current_player back to MrX, compute legal_actions on the new state
- If the result is empty: set is_terminal = true, winner = Some(PlayerId::Detectives)
- This check only runs when the newly active player is MrX

---
6. Terminal: Turn Limit Reached (MrX Wins)

If MrX survives all rounds, he wins. Scotland Yard uses 22 rounds; this should be a configurable constant rather than hardcoded.

Changes:
- Add max_turns: usize to GameState (set at construction time, default 22)
- In apply_action, after turn rotation, if turn_number >= max_turns and it's now MrX's turn again (i.e., the round just completed): set is_terminal = true, winner = Some(PlayerId::MrX)

---
Order of Implementation

┌──────┬────────────────────────────────────────────────────────┬──────────────┐
│ Step │                          What                          │    Where     │
├──────┼────────────────────────────────────────────────────────┼──────────────┤
│ 1    │ Add double to TicketInventory, add add_ticket method   │ gamestate.rs │
├──────┼────────────────────────────────────────────────────────┼──────────────┤
│ 2    │ Add Detectives variant to PlayerId                     │ gamestate.rs │
├──────┼────────────────────────────────────────────────────────┼──────────────┤
│ 3    │ Add max_turns to GameState                             │ gamestate.rs │
├──────┼────────────────────────────────────────────────────────┼──────────────┤
│ 4    │ Gate double moves on double ticket in legal_actions    │ rules.rs     │
├──────┼────────────────────────────────────────────────────────┼──────────────┤
│ 5    │ Spend double ticket in apply_action for Action::Double │ rules.rs     │
├──────┼────────────────────────────────────────────────────────┼──────────────┤
│ 6    │ Transfer detective ticket to MrX in apply_step         │ rules.rs     │
├──────┼────────────────────────────────────────────────────────┼──────────────┤
│ 7    │ Advance turn rotation in apply_action                  │ rules.rs     │
├──────┼────────────────────────────────────────────────────────┼──────────────┤
│ 8    │ Check detective-catches-MrX terminal condition         │ rules.rs     │
├──────┼────────────────────────────────────────────────────────┼──────────────┤
│ 9    │ Check MrX-has-no-moves terminal condition              │ rules.rs     │
├──────┼────────────────────────────────────────────────────────┼──────────────┤
│ 10   │ Check turn-limit terminal condition                    │ rules.rs     │
└──────┴────────────────────────────────────────────────────────┴──────────────┘


