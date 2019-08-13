use std::thread;
use ws::{self, CloseCode, Handler, Message, Result, Sender};

use serde::{Deserialize, Serialize};
use serde_json;

// State seen by the client, used to render the game.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct State {
    winner: Option<(Player, ((usize, usize), (usize, usize)))>,
    turn: Player,
    grid: Vec<Vec<Option<Player>>>,
    size: usize,
    win: usize,
    gravity: bool,
}

impl State {
    fn new(size: usize, win: usize, gravity: bool) -> Self {
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
    fn check_direction(&self, col: i32, row: i32, x: i32, y: i32, player: Player) -> usize {
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

struct Simulator {
    state: State,
    dirty: bool,
}

// This is our "server".
impl Simulator {
    // Simulate state changes based on commands.
    // This api allows additional commands without breaking the api.
    fn push(&mut self, cmd: Command) {
        match cmd {
            Command::Restart => {
                self.state = State::new(self.state.size, self.state.win, self.state.gravity);
            }
            Command::Place(col, row) => {}
        }
        self.dirty = true;
    }

    fn state(&mut self) -> Option<State> {
        if self.dirty {
            self.dirty = false;
            Some(self.state.clone())
        } else {
            None
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
enum Player {
    Naughts,
    Crosses,
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
enum Command {
    Place(u32, u32),
    Restart,
}

struct Server {
    out: Sender,
    state: State,
}

impl Handler for Server {
    fn on_message(&mut self, msg: Message) -> Result<()> {
        if let Message::Text(txt) = msg {
            let cmd: Command = serde_json::from_str(&txt).unwrap();
            match cmd {
                Command::Place(col, row) => {
                    if self.state.winner.is_some() {
                        return Ok(());
                    }
                    let col = col as usize;
                    let mut row = row as usize;
                    if self.state.gravity {
                        // If gravity is on, we place in the first open cell starting from
                        // the last row.
                        // If the column is completely full, then the click is a non-move.
                        if self.state.grid[col][0].is_some() {
                            return Ok(());
                        }
                        for ii in (0..self.state.grid[col].len()).rev() {
                            if self.state.grid[col][ii].is_none() {
                                self.state.grid[col][ii] = Some(self.state.turn);
                                row = ii; // Capture the real row value.
                                break;
                            }
                        }
                    } else {
                        if self.state.grid[col][row].is_some() {
                            return Ok(());
                        }
                        self.state.grid[col][row] = Some(self.state.turn);
                    }
                    for (forward, backward) in &[
                        ((1, 0), (-1, 0)),
                        ((0, 1), (0, -1)),
                        ((1, 1), (-1, -1)),
                        ((-1, 1), (1, -1)),
                    ] {
                        let forward_count = self.state.check_direction(
                            col as i32,
                            row as i32,
                            forward.0,
                            forward.1,
                            self.state.turn,
                        );
                        let backward_count = self.state.check_direction(
                            col as i32,
                            row as i32,
                            backward.0,
                            backward.1,
                            self.state.turn,
                        );
                        let count = forward_count + backward_count + 1;
                        if count >= self.state.win {
                            self.state.winner = Some((
                                Player::Crosses,
                                // Calculate the coordinates of the start cell and the end cell.
                                (
                                    (
                                        (col as i32 + forward.0 * forward_count as i32).max(0)
                                            as usize,
                                        (row as i32 + forward.1 * forward_count as i32).max(0)
                                            as usize,
                                    ),
                                    (
                                        (col as i32 + backward.0 * backward_count as i32).max(0)
                                            as usize,
                                        (row as i32 + backward.1 * backward_count as i32).max(0)
                                            as usize,
                                    ),
                                ),
                            ));
                            break;
                        }
                    }
                    self.state.turn = match self.state.turn {
                        Player::Naughts => Player::Crosses,
                        Player::Crosses => Player::Naughts,
                    };
                }
                Command::Restart => {
                    self.state = State::new(self.state.size, self.state.win, self.state.gravity);
                }
            };
        }
        self.out.send(Message::Text(
            serde_json::to_string_pretty(&self.state).unwrap(),
        ))
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {}
}

fn main() {
    ws::listen("127.0.0.1:1234", |out| Server {
        out: out,
        state: State::new(8, 4, true),
    })
    .unwrap();
}
