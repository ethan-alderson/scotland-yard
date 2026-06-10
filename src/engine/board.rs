/* We define a graph node which is a struct containing an id_number. A board is a set of nodes + a global edge list. 
The edge list is what encodes connections between nodes and their cost (tickets).
*/

use std::fs::File;
use std::str::FromStr;
use std::io::{BufRead, BufReader};

// StationId is the station number on the scotland yard board

#[derive(Copy, Clone, PartialEq)]
pub struct StationId {
    // 8-bit for 199 total nodes
    pub id: u8
}

#[derive(Copy, Clone)]
pub enum TicketType {
    Taxi,
    Bus,
    Underground,
    Black
}

impl FromStr for TicketType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "taxi" => Ok(TicketType::Taxi),
            "bus" => Ok(TicketType::Bus),
            "underground" => Ok(TicketType::Underground),
            "water" => Ok(TicketType::Black),
            _ => Err(format!("Unknown ticket type: {}", s)),
        }
    }
}

// The board struct is a graph container containing all nodes and edges
/* We will keep edges for static construction and debugging but we'll add an adjacency map to 
accelerate MCTS.  */
pub struct Board {
    // 2D vector, outer is length num_nodes, inner is variable length but its a list 
    // of tuples of neighbors and the ticket required to get there
    adjacency_map: Vec<Vec<(StationId, TicketType)>>
}

impl Board {

    // index the board given a 1 indexed value to isolate the indexing difference
    pub fn neighbors(&self, sid: StationId) -> &Vec<(StationId, TicketType)> {
        &self.adjacency_map[sid.id as usize - 1]
    }

    pub fn from_connections_file() -> Self {

        let file = File::open("connections.txt").expect("failed to open file");
        let reader = BufReader::new(file);

        let size: usize = 199;

        let mut adj_map: Vec<Vec<(StationId, TicketType)>> = vec![vec![]; size];

        for line in reader.lines() {
            let line_string = line.expect("failed to parse");
            let record: Vec<&str> = line_string.split(' ').collect();
            
            let idx: usize = record[0].parse::<usize>().unwrap();
            // destination is still station id which is indexed at 1
            let dest: StationId = StationId {id: record[1].parse::<u8>().unwrap()};
            let ticket: TicketType = record[2].parse::<TicketType>().unwrap();

            adj_map[idx].push((dest, ticket));

            // include both edge directions in the adjacency map
            let dest_idx = (dest.id - 1) as usize;
            adj_map[dest_idx].push((StationId {id: (idx + 1) as u8}, ticket));
        }

        Board {
            adjacency_map: adj_map,
        }
    }
}

/*

The adjacency map is a Vec<Vec<(u8, TicketType)>>

A line is source node, destination node, ticket type as a string. 

adjacency map = vector of length 199 with empty vectors at each element

for line:

    s_id, d_id, ticket_str = line.split(' ')

    ticket = str_to_enum(ticket_str)

    adj_map[s_id - 1].push((d_id, ticket))







*/











/*
Test that every node in the range exists in the adjacency map
test that there are no out of bounds nodes
test that there are no missing nodes

Test that for any edge traveling one direction there is an edge in the opposite direction

Ensure every edge has a valid ticket type

No duplicate edges A -> B and A -> B

*/

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn foo () {

//     }
// }
