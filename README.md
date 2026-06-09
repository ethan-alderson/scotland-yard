TO DO

Figure out a plan to fix the - 1 indexing issue (board nodes are idx'd at 1, the adjacency map vector is indexed at 0)

Need to add condition to the legal_actions check that ensures a target position is not occupied. Note that it is illegal to move onto a detective at any time, but moving onto mr X is legal.

Determine the correct location of legal_moves (should it be a function of gamestate?)