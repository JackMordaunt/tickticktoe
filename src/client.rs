#![windows_subsystem = "windows"]

use ggez;
use ggez::event::KeyCode;
use ggez::event::{self, MouseButton};
use ggez::graphics::{self, DrawMode, MeshBuilder};
use ggez::input::keyboard::KeyMods;
use ggez::timer;
use ggez::Context;

use clap::{App, Arg};
use serde::{Serialize, Deserialize};
use serde_json;

impl Player {
    fn color(&self) -> graphics::Color {
        match self {
            Player::Naughts => [1.0, 0.647, 0.0, 1.0].into(),
            Player::Crosses => [0.0, 0.35, 1.0, 1.0].into(),
        }
    }
}

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

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
enum Player {
    Naughts,
    Crosses,
}

#[derive(Serialize, Deserialize)]
enum Command {
    Place(u32, u32),
    Restart,
}

// Client transforms hardware events into simulation commands,
// and renders the game state to the screen.
struct Client {
    sim: Simulator,
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

use ws::{self, Handler, Result, Message, CloseCode, Sender, Handshake};

use std::sync::Arc;
use std::sync::Mutex;

struct Simulator {
    state: Option<State>,
    out: Option<Sender>,
}

impl Handler for Simulator {
    fn on_message(&mut self, msg: Message) -> Result<()> {
        if let Message::Text(txt) = msg {
            let state: State = serde_json::from_str(&txt).unwrap();
            self.state = Some(state);
        }
        Ok(())
    }
}

// This is our "server".
impl Simulator {
    // FIXME: Simulator needs to mutate it's state based on async messages
    // from websocket connection, while also being accessed from Client. 
    // Should Client own an Arc<Mutex<Simulator>>? 
    fn new() -> Arc<Mutex<Self>> {
        let mut sim = Arc::new(Mutex::new(Simulator {
            out: None,
            state: None,
        }));
        std::thread::spawn(move || {
            ws::connect("ws://127.0.0.1:1234", |out| {
                sim.clone().get_mut().unwrap().out = Some(out);
                sim
            })
            .unwrap();
        });
        sim
    }
    // Simulate state changes based on commands.
    // This api allows additional commands without breaking the api.
    fn push(&mut self, cmd: Command) {
        if let Some(out) = self.out {
            out.send(Message::Text(serde_json::to_string(&cmd).unwrap())).unwrap();
        }
    }

    fn state(&mut self) -> Option<State> {
        self.state
    }
}

impl event::EventHandler for Client {
    fn update(&mut self, _ctx: &mut ggez::Context) -> ggez::GameResult {
        if let Some(state) = self.sim.state() {
            self.state = state;
        }
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
    let state = State::new(size, win, gravity);
    let client = &mut Client {
        sim: Simulator::new(),
        state: state, // State to render from.
    };
    let (ctx, event_loop) = &mut cb.build()?;
    event::run(ctx, event_loop, client)
}
