/* We define a graph node which is a struct containing an id_number. A board is a set of nodes + a global edge list. 
The edge list is what encodes connections between nodes and their cost (tickets).
*/

use std::collections::HashSet;

struct Node {
    // 8-bit for 199 total nodes
    id: u8
}

enum Ticket_Type {
    taxi,
    bus,
    underground
}

struct Transition {
    destination_id: u8,
    ticket: Ticket_Type
}

// The board struct is a graph container containing all nodes and edges
/* We will keep edges for static construction and debugging but we'll add an adjacency map to 
accelerate MCTS.  */
struct Board {
    // Node ids are in a bounded index space [1, num_nodes], num_nodes is 199. 
    num_nodes: u16,
    // 2D vector, outer is length num_nodes, inner is variable length but its a list 
    // of tuples of neighbors and the ticket required to get there
    adjacency_map: Vec<Vec<Transition>>
}

