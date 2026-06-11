mod engine;

use engine::board::Board;
use engine::board::StationId;

use engine::gamestate::GameState;
use engine::gamestate::PlayerId;

use engine::gamestate::PlayerState;
use engine::gamestate::TicketInventory;

fn main() {
    let board = Board::from_connections_file(String::from("connections.txt"));
    // need to construct players

    let xtix = TicketInventory::new( 4,3, 3, 5);
    let dtix = TicketInventory::new( 10,8, 4, 0);
    
    let mrx = PlayerState::new(PlayerId::MrX, StationId {id: 0}, xtix);
    let detective = PlayerState::new(PlayerId::Detective(0), StationId{ id: 100 }, dtix);
    
    // initialize gamestate
    let _gamestate= GameState::new(&board, vec![mrx, detective]);
    
    println!("ready!");
}
