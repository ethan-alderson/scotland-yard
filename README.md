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



