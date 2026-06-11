/* We define a graph node which is a struct containing an id_number. A board is a set of nodes + a global edge list. 
The edge list is what encodes connections between nodes and their cost (tickets).
*/

use std::fs::File;
use std::str::FromStr;
use std::io::{BufRead, BufReader};

use::serde::{Serialize, Deserialize};

// StationId is the station number on the scotland yard board

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub struct StationId {
    // 8-bit for 199 total nodes
    pub id: u8
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
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
#[derive(Serialize, Deserialize)]
pub struct Board {
    // 2D vector, outer is length num_nodes, inner is variable length but its a list 
    // of tuples of neighbors and the ticket required to get there
    pub adjacency_map: Vec<Vec<(StationId, TicketType)>>
}

impl Board {

    // index the board given a 1 indexed value to isolate the indexing difference
    pub fn neighbors(&self, sid: StationId) -> &Vec<(StationId, TicketType)> {
        &self.adjacency_map[sid.id as usize - 1]
    }

    pub fn from_connections_file(filepath: String) -> Self {

        let file = File::open(filepath).expect("failed to open file");
        let reader = BufReader::new(file);

        let size: usize = 199;

        let mut adj_map: Vec<Vec<(StationId, TicketType)>> = vec![vec![]; size];

        for line in reader.lines() {
            let line_string = line.expect("failed to parse");
            let record: Vec<&str> = line_string.split(' ').collect();
            
            let idx: usize = record[0].parse::<usize>().unwrap() - 1;
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


#[cfg(test)]
mod tests {
    use super::*;
    
    fn setup_board() -> Board {
        Board::from_connections_file(String::from("../connections.txt"))
    }

    #[test]
    fn assert_valid_stations () {
        let test_board = setup_board();

        for neighbors in &test_board.adjacency_map {
            for (station_id, _) in neighbors {
                // assert!(station_id.id >= 0); // technically useless but in case I move away from u8s I'll keep it
                assert!(station_id.id <= 199);
            }
        }
    }

    #[test]
    fn assert_no_skipped_stations () {

        let test_board = setup_board();

        assert_eq!(
            test_board.adjacency_map.len(),
            199,
            "Board should contain 199 stations"
        );
    }

    #[test]
    fn assert_no_isolated_stations() {
        let test_board = setup_board();

        for (i, neighbors) in test_board.adjacency_map.iter().enumerate() {
            assert!(
                !neighbors.is_empty(),
                "Station {} has no neighbors",
                i + 1
            );
        }
    }

    #[test]
    fn assert_all_edges_bidirectional() {
        use std::collections::HashSet;
        
        let board = setup_board();

        // Collect all directed edges into a set
        let mut edges = HashSet::new();

        for (from_idx, neighbors) in board.adjacency_map.iter().enumerate() {
            let from = StationId { id: from_idx as u8 + 1 };

            for (to, ticket) in neighbors {
                edges.insert((from, *to, *ticket));
            }
        }

        // Check that every edge has a reverse edge
        for (from, to, ticket) in &edges {
            assert!(
                edges.contains(&(*to, *from, *ticket)),
                "Missing reverse edge: {:?} -> {:?} (ticket {:?})",
                from,
                to,
                ticket
            );
        }
    }

    #[test]
    fn assert_no_duplicate_edges() {
        use std::collections::HashSet;

        let board = setup_board();

        // track all directed edges and ensure we never see the same one twice
        let mut seen = HashSet::new();

        for (from_idx, neighbors) in board.adjacency_map.iter().enumerate() {
            let from = StationId { id: from_idx as u8 + 1 };

            for (to, ticket) in neighbors {
                let edge = (from, *to, *ticket);

                assert!(
                    seen.insert(edge),
                    "Duplicate edge found: {:?} -> {:?} ({:?})",
                    from,
                    to,
                    ticket
                );
            }
        }
    }

    #[test]
    fn assert_no_self_loops() {
        let board = setup_board();

        for (from_idx, neighbors) in board.adjacency_map.iter().enumerate() {
            let from = StationId { id: from_idx as u8 + 1 };

            for (to, ticket) in neighbors {
                assert!(
                    from != *to,
                    "Self-loop detected at station {:?} with ticket {:?}",
                    from,
                    ticket
                );
            }
        }
    }
}