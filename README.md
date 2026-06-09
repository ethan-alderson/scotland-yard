TO DO

Figure out a plan to fix the - 1 indexing issue (board nodes are idx'd at 1, the adjacency map vector is indexed at 0)

Need to add condition to the legal_actions check that ensures a target position is not occupied. Note that it is illegal to move onto a detective at any time, but moving onto mr X is legal (and terminal).

Determine the correct location of legal_moves (should it be a function of gamestate?)



INDEXING PLAN:

Add a NodeId Struct that wraps a u8, u8 is the 1 indexed visual station # on the actual game board.
- If a player is at station 42, the player state will have a NodeId(42)

Add a neighbors function to board that produces all the neighbors of a given nodeId. This function contains 
a -1 that indexes the adjacency map correctly. 


1) Define Station_Id as a 1 indexed u8 wrapper
2) Convert adjacency map to a list of station ids
3) Write neighbors impl in board that handles the transition from 1 to 0 indexing for the source station
4) Repair function to load board from file that uses the raw Station IDs.
5) Rewrite gamestate steps and other u8s to reference Station ID.
6) Rewrite rules to use new station ID. 