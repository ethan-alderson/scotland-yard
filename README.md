TO DO

Figure out a plan to fix the - 1 indexing issue (board nodes are idx'd at 1, the adjacency map vector is indexed at 0)

Need to add condition to the legal_actions check that ensures a target position is not occupied. Note that it is illegal to move onto a detective at any time, but moving onto mr X is legal (and terminal).

Determine the correct location of legal_moves (should it be a function of gamestate?)

LEGAL_MOVES IMPROVEMENTS:

Need a notion double moves for mr X ****

Cannot move onto a position occupied by a detective. If a detective moves onto a position with mrx the gamestate is terminal and the winner is a detective.

Add specific moves for MrX, black ticket pruning, and double tickets. Need to design double tickets here. 

Add function to apply action to gamestate

INDEXING PLAN:

Add a NodeId Struct that wraps a u8, u8 is the 1 indexed visual station # on the actual game board.
- If a player is at station 42, the player state will have a NodeId(42)

Add a neighbors function to board that produces all the neighbors of a given nodeId. This function contains 
a -1 that indexes the adjacency map correctly. 

Indexing error:

1) Define Station_Id as a 1 indexed u8 wrapper - DONE
2) Convert adjacency map tuples to station ids from u8s - DONE
3) Write neighbors function in board struct that handles the transition from 1 to 0 indexing for the source station - DONE
4) Repair function to load board from file that uses the Station IDs but fills adjacency map - DONE
5) Rewrite gamestate steps and other u8s to reference Station ID - DONE
6) Rewrite rules to use new station ID - DONE