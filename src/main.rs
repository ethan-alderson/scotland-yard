mod engine;

use engine::board::Board;
use engine::gamestate::GameState;
use engine::gamestate::PlayerId;

use engine::gamestate::PlayerState;
use engine::gamestate::TicketSet;

fn main() {
    let board = Board::from_connections_file();
    // need to construct players

    let xtix = TicketSet::new( 4,3, 3, 5);
    let mrx = PlayerState::new(PlayerId::MrX, 0, xtix);

    // initialize gamestate
    let _gamestate= GameState::new(&board, vec![mrx]);
    
    println!("ready!");
}
