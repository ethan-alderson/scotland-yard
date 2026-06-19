//! Static board geometry for the frontend: station pixel coordinates (from
//! `pos.txt`), connection edges (from `connections.txt`), and the map image
//! metadata. Loaded once at startup and served verbatim from `GET /api/board`.

use std::fs;
use std::path::Path;

use engine::board::TicketType;
use serde::Serialize;

use crate::dto::TicketDto;

#[derive(Serialize, Clone)]
pub struct ImageMeta {
    pub w: u32,
    pub h: u32,
    pub url: String,
}

/// A station's pixel position within `map.png` (native image space).
#[derive(Serialize, Clone)]
pub struct StationGeom {
    pub id: u16,
    pub x: u16,
    pub y: u16,
}

/// One connection between two stations, by transport. Parallel edges (e.g. taxi
/// *and* bus between the same pair) are kept distinct so the UI can draw each.
#[derive(Serialize, Clone)]
pub struct EdgeDto {
    pub from: u16,
    pub to: u16,
    pub ticket: TicketDto,
}

#[derive(Serialize, Clone)]
pub struct BoardDto {
    pub image: ImageMeta,
    pub stations: Vec<StationGeom>,
    pub edges: Vec<EdgeDto>,
}

impl BoardDto {
    /// Build the payload from the asset files. Panics loudly on malformed input
    /// — this runs at startup, so a bad asset should stop the server, not serve
    /// a broken board.
    pub fn load(connections_path: &Path, assets_dir: &Path) -> Self {
        let pos_path = assets_dir.join("pos.txt");
        let map_path = assets_dir.join("map.png");

        let stations = parse_positions(&pos_path);
        let edges = parse_edges(connections_path);
        let (w, h) = png_dimensions(&map_path);

        Self {
            image: ImageMeta { w, h, url: "/assets/map.png".to_string() },
            stations,
            edges,
        }
    }
}

/// `pos.txt`: first line is the station count, then `id x y` per station.
fn parse_positions(path: &Path) -> Vec<StationGeom> {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    let mut lines = text.lines();

    let count: usize = lines
        .next()
        .expect("pos.txt is empty")
        .trim()
        .parse()
        .expect("pos.txt first line must be the station count");

    let stations: Vec<StationGeom> = lines
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let mut f = line.split_whitespace();
            let id = f.next().expect("missing station id").parse().expect("bad id");
            let x = f.next().expect("missing x").parse().expect("bad x");
            let y = f.next().expect("missing y").parse().expect("bad y");
            StationGeom { id, x, y }
        })
        .collect();

    assert_eq!(
        stations.len(),
        count,
        "pos.txt declared {count} stations but listed {}",
        stations.len()
    );
    stations
}

/// `connections.txt`: `from to ticket`, ticket ∈ {taxi,bus,underground,water}.
/// `water` is the ferry edge, which the engine models as a Black ticket.
fn parse_edges(path: &Path) -> Vec<EdgeDto> {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let mut f = line.split_whitespace();
            let from = f.next().expect("missing edge `from`").parse().expect("bad from");
            let to = f.next().expect("missing edge `to`").parse().expect("bad to");
            // Reuse the engine's canonical string→ticket mapping (water→Black).
            let ticket: TicketType = f
                .next()
                .expect("missing edge ticket")
                .parse()
                .expect("unknown ticket type");
            EdgeDto { from, to, ticket: ticket.into() }
        })
        .collect()
}

/// Read width/height straight from the PNG IHDR chunk (big-endian u32s at byte
/// offsets 16 and 20), avoiding an image-decoding dependency.
fn png_dimensions(path: &Path) -> (u32, u32) {
    let header = fs::read(path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    assert!(header.len() >= 24, "{} is too short to be a PNG", path.display());
    assert_eq!(&header[1..4], b"PNG", "{} is not a PNG", path.display());

    let w = u32::from_be_bytes([header[16], header[17], header[18], header[19]]);
    let h = u32::from_be_bytes([header[20], header[21], header[22], header[23]]);
    (w, h)
}
