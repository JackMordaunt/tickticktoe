
use serde::{Serialize, Deserialize};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Player {
    Naughts,
    Crosses,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Command {
    Place(u32, u32),
    Restart,

    // Lobby commands.
    StartGame,
    SetWinCondition(u32),
    SetGridSize(u32),
    SetGravity(bool),
}

// State seen by the client, used to render the game.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub winner: Option<(Player, ((usize, usize), (usize, usize)))>,
    pub turn: Player,
    pub grid: Vec<Vec<Option<Player>>>,
    pub size: usize,
    pub win: usize,
    pub gravity: bool,
}

impl State {
    pub fn new(size: usize, win: usize, gravity: bool) -> Self {
        State {
            winner: None,
            turn: Player::Naughts,
            grid: vec![vec![None; size]; size],
            size: size,
            win: win,
            gravity: gravity,
        }
    }

    // Checks for consecutive pieces owned by this player in a given direction,
    // returning the count of pieces.
    pub fn check_direction(&self, col: i32, row: i32, x: i32, y: i32, player: Player) -> usize {
        let mut count = 0;
        let mut col = col;
        let mut row = row;
        loop {
            col += x;
            row += y;
            if self.size - 1 < col as usize || col < 0 || self.size - 1 < row as usize || row < 0 {
                return count;
            }
            if self.grid[col as usize][row as usize] == Some(player) {
                count += 1;
            } else {
                return count;
            }
        }
    }
}