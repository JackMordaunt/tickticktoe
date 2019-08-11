#![windows_subsystem = "windows"]

use ggez;
use ggez::event::KeyCode;
use ggez::event::{self, MouseButton};
use ggez::graphics::{self, DrawMode, MeshBuilder};
use ggez::input::keyboard::KeyMods;
use ggez::timer;
use ggez::Context;

use clap::{App, Arg};

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Player {
    Naughts,
    Crosses,
}

impl Player {
    fn color(&self) -> graphics::Color {
        match self {
            Player::Naughts => [1.0, 0.647, 0.0, 1.0].into(),
            Player::Crosses => [0.0, 0.35, 1.0, 1.0].into(),
        }
    }
}

// State seen by the client, used to render the game.
#[derive(Clone, Debug)]
struct State {
    winner: Option<(Player, ((usize, usize), (usize, usize)))>,
    turn: Player,
    grid: Vec<Vec<Option<Player>>>,
    size: usize,
    win: usize,
    gravity: bool,
}

enum Command {
    Place(u32, u32),
    Restart,
}

// Simulator decouples the game simulation from input processing and rendering.
// Simulation could occur on the host machine or over a network (ie, on a server).
trait Simulator {
    // Push commands to the simulator. Driven by the client.
    fn push(&mut self, cmd: Command);
    // Receive the updated state from the simulator. Driven by server.
    fn state_changes(&mut self) -> Receiver<State>;
}

// Client transforms hardware events into simulation commands,
// and renders the game state to the screen.
struct Client {
    sim: Box<Simulator>,
    state: State,
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

    fn build_grid(&self, ctx: &ggez::Context, mb: &mut MeshBuilder) -> ggez::GameResult {
        let ((w, h), stroke, color) = (graphics::drawable_size(ctx), 2.0, self.turn.color());
        let column_width = w / self.size as f32;
        for ii in 1..self.size {
            let offset = column_width * ii as f32;
            mb.line(&[[offset, 0.0], [offset, h]], stroke, color)?;
        }
        let row_height = h / self.size as f32;
        for ii in 1..self.size {
            let offset = row_height * ii as f32;
            mb.line(&[[0.0, offset], [w, offset]], stroke, color)?;
        }
        Ok(())
    }

    fn build_players(&self, ctx: &ggez::Context, mb: &mut MeshBuilder) -> ggez::GameResult {
        let (w, h) = graphics::drawable_size(ctx);
        let column_width = w / self.size as f32;
        let row_height = h / self.size as f32;
        let size = (column_width + row_height) / 2.0 / 4.0;
        for (ii, col) in self.grid.iter().enumerate() {
            for (jj, cell) in col.iter().enumerate() {
                if let Some(player) = cell {
                    let (x, y) = (
                        (column_width) * ((ii + 1) as f32) - (column_width / 2.0),
                        (row_height) * ((jj + 1) as f32) - (row_height / 2.0),
                    );
                    let color = player.color();
                    match player {
                        Player::Naughts => {
                            mb.circle(DrawMode::stroke(2.0), [x, y], size, 0.1, color);
                        }
                        Player::Crosses => {
                            mb.line(&[[x - size, y - size], [x + size, y + size]], 2.0, color)?;
                            mb.line(&[[x + size, y - size], [x - size, y + size]], 2.0, color)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn build_throughline(&self, ctx: &ggez::Context, mb: &mut MeshBuilder) -> ggez::GameResult {
        if let Some((_, (start, end))) = &self.winner {
            let (w, h) = graphics::drawable_size(ctx);
            let stroke = 2.0;
            let column_size = w / self.size as f32;
            let row_size = h / self.size as f32;
            let coords = [
                [
                    start.0 as f32 * column_size + column_size / 2.0 - stroke / 2.0,
                    start.1 as f32 * row_size + row_size / 2.0 - stroke / 2.0,
                ],
                [
                    end.0 as f32 * column_size + column_size / 2.0 - stroke / 2.0,
                    end.1 as f32 * row_size + row_size / 2.0 - stroke / 2.0,
                ],
            ];
            mb.line(&coords, stroke, [1.0, 1.0, 1.0, 1.0].into())?;
        }
        Ok(())
    }
}

struct LocalSimulator {
    state: State,
    state_changes: Option<Sender<State>>,
}

// This is our "server".
impl Simulator for LocalSimulator {
    // Simulate state changes based on commands.
    // This api allows additional commands without breaking the api.
    fn push(&mut self, cmd: Command) {
        match cmd {
            Command::Restart => {
                self.state = State::new(self.state.size, self.state.win, self.state.gravity);
            }
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
        }
        // self.state_changed(self.state.clone());
        if let Some(sender) = self.state_changes.clone() {
            sender.send(self.state.clone()).unwrap();
        }
    }

    fn state_changes(&mut self) -> Receiver<State> {
        let (sx, rx) = channel();
        self.state_changes = Some(sx);
        rx
    }
}

impl event::EventHandler for Client {
    fn update(&mut self, _ctx: &mut ggez::Context) -> ggez::GameResult {
        timer::yield_now();
        Ok(())
    }

    fn key_up_event(&mut self, _ctx: &mut Context, code: KeyCode, _keymods: KeyMods) {
        match code {
            KeyCode::Return => self.sim.push(Command::Restart),
            _ => {}
        }
    }

    fn mouse_button_up_event(&mut self, ctx: &mut Context, _btn: MouseButton, x: f32, y: f32) {
        let (w, h) = graphics::drawable_size(ctx);
        let col = (x / w * self.state.size as f32).min(self.state.size as f32 - 1.0) as u32;
        let row = (y / h * self.state.size as f32).min(self.state.size as f32 - 1.0) as u32;
        self.sim.push(Command::Place(col, row));
    }

    fn draw(&mut self, ctx: &mut ggez::Context) -> ggez::GameResult {
        graphics::clear(ctx, [0.0, 0.0, 0.0, 0.0].into());
        let mut mb = MeshBuilder::new();
        self.state.build_grid(ctx, &mut mb)?;
        self.state.build_players(ctx, &mut mb)?;
        if self.state.winner.is_some() {
            self.state.build_throughline(ctx, &mut mb)?;
        }
        let mesh = mb.build(ctx)?;
        graphics::draw(ctx, &mesh, graphics::DrawParam::default())?;
        graphics::present(ctx)?;
        Ok(())
    }
}

fn main() -> ggez::GameResult {
    let matches = App::new("Tick Tack Toe")
        .arg(
            Arg::with_name("size")
                .takes_value(true)
                .long("size")
                .short("s")
                .help("Size of grid."),
        )
        .arg(
            Arg::with_name("win")
                .takes_value(true)
                .long("win")
                .short("w")
                .help("Number of aligned pieces required to win the game."),
        )
        .arg(
            Arg::with_name("gravity")
                .takes_value(false)
                .long("gravity")
                .short("g")
                .help("Simulate gravity when placing a piece."),
        )
        .get_matches();
    let size = matches
        .value_of("size")
        .unwrap_or("3")
        .parse::<usize>()
        .expect("parsing size value");
    let win = matches
        .value_of("win")
        .unwrap_or("3")
        .parse::<usize>()
        .expect("parsing win value");
    let gravity = matches.is_present("gravity");
    let cb = ggez::ContextBuilder::new("Tick Tack Toe", "Jack Mordaunt")
        .window_setup(ggez::conf::WindowSetup::default().vsync(true));
    let (ctx, event_loop) = &mut cb.build()?;
    // let state = State::new(size, win, gravity);
    // let mut sim = LocalSimulator {
    //     state: state.clone(),
    //     state_changes: None,
    // };
    // let client = &mut Client {
    //     sim: Box::new(sim),
    //     state: state,
    // };
    // let client = Arc::new(client);
    // let (tx, rx) = channel();
    // sim.state_changes = Some(tx);
    // std::thread::spawn(move || {
    //     for state in rx {
    //         client.state = state;
    //     }
    // });
    event::run(ctx, event_loop, client)
}
