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


UI Plan:

yoink the board and station coords from https://github.com/tim-koehler/ScotlandYard

Build front end layer in react on top of rust API back end

1. Add axum and tokio to server/Cargo.toml and get a literal "hello world" GET / route compiling
2. Add Axum State with a dummy struct — just to see how state threading works
3. Wire in your real GameState behind Arc<Mutex<...>>
4. Add routes one at a time, starting with GET /game (read-only, simpler)
5. Add POST /game/move last (write path, needs request body deserialization)



