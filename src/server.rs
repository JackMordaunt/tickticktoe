use ws::{self, Factory, Handler, Message, Result, Sender};
use crossbeam_channel::{unbounded, Sender as ChanSender};
use serde::{Deserialize, Serialize};
use serde_json;
use uuid::Uuid;
use std::thread;

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

struct Connection {
    // id: Uuid,
    // out: Sender,
    // state: State,
    cmds: ChanSender<Command>,
}

impl Handler for Connection {
    fn on_message(&mut self, msg: Message) -> Result<()> {
        if let Message::Text(txt) = msg {
            let cmd: Command = serde_json::from_str(&txt).unwrap();
            self.cmds.send(cmd).unwrap();
        }
        Ok(())
        // self.out.send(Message::Text(
        //     serde_json::to_string_pretty(&self.state).unwrap(),
        // ))
    }
}

struct Server {
    state: State,
    players: Vec<Sender>,
    spectators: Vec<Sender>,
}

impl Server {
    fn cmd(&mut self, cmd: Command) {
        match cmd {
            Command::Place(col, row) => {
                if self.state.winner.is_some() {
                    return;
                }
                let col = col as usize;
                let mut row = row as usize;
                if self.state.gravity {
                    // If gravity is on, we place in the first open cell starting from
                    // the last row.
                    // If the column is completely full, then the click is a non-move.
                    if self.state.grid[col][0].is_some() {
                        return;
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
                        return;
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
}

impl Factory for Server {
    type Handler = Connection;

    fn connection_made(&mut self, out: Sender) -> Self::Handler {
        if self.players.len() < 2 {
            self.players[self.players.len()] = out;
        } else {
            self.spectators.push(out);
        }
        let (tx, rx) = unbounded();
        // FIXME: Not sure how to get back commands from the connections.
        // Need some abstraction that handles multiple connections, like a "room".
        thread::spawn(move || {
            for cmd in rx {
                self.cmd(cmd);
            }
        });
        Connection {
            // id: Uuid::new_v4(),
            // out: out,
            // state: self.state.clone(),
            cmds: tx,
        }
    }
}

fn main() {
    let svr = Server {
        state: State::new(8, 4, true),
        players: vec![],
        spectators: vec![],
    };
    ws::WebSocket::new(svr)
        .unwrap()
        .bind("127.0.0.1:1234")
        .unwrap()
        .run()
        .unwrap();
    // ws::listen("127.0.0.1:1234", svr)
    // .unwrap();
}
