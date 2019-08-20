#![allow(dead_code, unused_imports)]
use crossbeam_channel::{unbounded, Receiver as ChanReceiver, Sender as ChanSender};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use uuid::Uuid;
use ws::{self, Factory, Handler, Message, Result, Sender};

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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
enum Command {
    Place(u32, u32),
    Restart,

    // Lobby commands.
    StartGame,
    SetWinCondition(u32),
    SetGridSize(u32),
    SetGravity(bool),
}

#[derive(Clone)]
struct Game {
    state: State,
}

impl Game {
    fn simulate(&mut self, cmd: Command) {
        println!("simulate: {:?}", cmd);
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
                                    (col as i32 + forward.0 * forward_count as i32).max(0) as usize,
                                    (row as i32 + forward.1 * forward_count as i32).max(0) as usize,
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
            _ => {}
        };
    }
}

#[derive(Clone)]
struct Client {
    id: Uuid,
    out: Sender,
    lobby: Arc<Mutex<Lobby>>,
}

#[derive(Clone, Serialize, Deserialize)]
struct ClientMessage {
    id: Uuid,
    cmd: Command,
}

// Lobby contains the state for a pre-game lobby.
struct Lobby {
    players: HashMap<Uuid, Player>,
    spectators: Vec<Client>,
    settings: GameSettings,
    game: Option<Game>,
}

struct SharedLobby {
    state: Arc<Mutex<Lobby>>,
}

// GameSettings contains the params required to start a game.
struct GameSettings {
    grid_size: Option<u32>,
    win_condition: Option<u32>,
    gravity: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
enum LobbyCommand {
    StartGame,
    SetWinCondition(u32),
    SetGridSize(u32),
    SetGravity(bool),
}

impl Factory for SharedLobby {
    type Handler = Client;

    // A new connection comes in.
    // Each new connection will be a client.
    fn connection_made(&mut self, out: Sender) -> Self::Handler {
        let client = Client {
            id: Uuid::new_v4(),
            out: out,
            lobby: self.state.clone(),
        };
        if let Ok(mut lobby) = self.state.lock() {
            let player_count = lobby.players.len();
            if player_count < 1 {
                lobby.players.insert(client.id, Player::Crosses);
            } else if player_count < 2 {
                lobby.players.insert(client.id, Player::Naughts);
            }
            lobby.spectators.push(client.clone());
        }
        client
    }
}

impl Lobby {
    fn apply(&mut self, msg: ClientMessage) -> Result<()> {
        let ClientMessage { id, cmd } = msg;
        if let Some(player) = self.players.get(&id) {
            println!("{:?}.{:?}", player, cmd);
            if let Some(mut game) = self.game.take() {
                if *player == game.state.turn {
                    game.simulate(cmd);
                    let state = game.state.clone();
                    for client in &self.spectators {
                        client
                            .out
                            .send(Message::Text(serde_json::to_string(&state).unwrap()))?;
                    }
                }
                self.game = Some(game);
            } else {
                match cmd {
                    Command::SetWinCondition(win_condition) => {
                        self.settings.win_condition = Some(win_condition);
                    }
                    Command::SetGridSize(grid_size) => {
                        self.settings.grid_size = Some(grid_size);
                    }
                    Command::SetGravity(gravity) => {
                        self.settings.gravity = Some(gravity);
                    }
                    Command::StartGame => {
                        if self.settings.is_valid() && self.game.is_none() {
                            self.game = Some(Game {
                                state: State::new(
                                    self.settings.grid_size.unwrap() as usize,
                                    self.settings.win_condition.unwrap() as usize,
                                    self.settings.gravity.unwrap(),
                                ),
                            });
                        }
                    }
                    _ => {
                        println!("ignoring command: {:?}", cmd);
                    }
                }
            }
        }
        Ok(())
    }
}

impl Handler for Client {
    fn on_message(&mut self, msg: Message) -> Result<()> {
        if let Ok(mut lobby) = self.lobby.lock() {
            if let Message::Text(txt) = msg {
                if let Ok(cmd) = serde_json::from_str::<Command>(&txt) {
                    lobby.apply(ClientMessage {
                        id: self.id,
                        cmd: cmd,
                    })?;
                }
            }
        }
        Ok(())
    }
}

impl GameSettings {
    // Valid if all fields are Some.
    fn is_valid(&self) -> bool {
        if let None = self.grid_size {
            return false;
        }
        if let None = self.win_condition {
            return false;
        }
        if let None = self.gravity {
            return false;
        }
        true
    }
}

impl Default for GameSettings {
    fn default() -> Self {
        GameSettings {
            grid_size: None,
            win_condition: None,
            gravity: None,
        }
    }
}

fn main() {
    let mut lobby = SharedLobby {
        state: Arc::new(Mutex::new(Lobby {
            players: HashMap::new(),
            spectators: vec![],
            settings: GameSettings::default(),
            game: None,
        })),
    };
    ws::listen("25.32.94.215:8080", move |out| lobby.connection_made(out)).unwrap();
}
