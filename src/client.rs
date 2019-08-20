// #![windows_subsystem = "windows"]

use clap::{App, Arg};
use crossbeam_channel::{unbounded, Receiver, Sender};
use ggez;
use ggez::event::KeyCode;
use ggez::event::{self, MouseButton};
use ggez::graphics::{self, DrawMode, MeshBuilder};
use ggez::input::keyboard::KeyMods;
use ggez::timer;
use ggez::Context;
use serde_json;
use ws::{self, Handler, Message, Result};

use ticktacktoe::{State, Player, Command};

// AsColor associates a Color to an arbitrary type.
trait AsColor {
    fn as_color(&self) -> graphics::Color;
}

impl AsColor for Player {
    fn as_color(&self) -> graphics::Color {
        match self {
            Player::Naughts => [1.0, 0.647, 0.0, 1.0].into(),
            Player::Crosses => [0.0, 0.35, 1.0, 1.0].into(),
        }
    }
}

// Client transforms hardware events into simulation commands,
// and renders the game state to the screen.
struct Client {
    sim: Simulator,
    state: Option<State>,
}

// Renderer renders the game state to a MeshBuilder which can be drawn
// by ggez.
struct Renderer {
    pub state: State,
}

impl Renderer {

    pub fn draw(&self, ctx: &ggez::Context, mb: &mut MeshBuilder) -> ggez::GameResult {
        self.build_grid(ctx, mb)?;
        self.build_players(ctx, mb)?;
        if self.state.winner.is_some() {
            self.build_throughline(ctx, mb)?;
        }
        Ok(())
    }

    fn build_grid(&self, ctx: &ggez::Context, mb: &mut MeshBuilder) -> ggez::GameResult {
        let ((w, h), stroke, color) = (graphics::drawable_size(ctx), 2.0, self.state.turn.as_color());
        let column_width = w / self.state.size as f32;
        for ii in 1..self.state.size {
            let offset = column_width * ii as f32;
            mb.line(&[[offset, 0.0], [offset, h]], stroke, color)?;
        }
        let row_height = h / self.state.size as f32;
        for ii in 1..self.state.size {
            let offset = row_height * ii as f32;
            mb.line(&[[0.0, offset], [w, offset]], stroke, color)?;
        }
        Ok(())
    }

    fn build_players(&self, ctx: &ggez::Context, mb: &mut MeshBuilder) -> ggez::GameResult {
        let (w, h) = graphics::drawable_size(ctx);
        let column_width = w / self.state.size as f32;
        let row_height = h / self.state.size as f32;
        let size = (column_width + row_height) / 2.0 / 4.0;
        for (ii, col) in self.state.grid.iter().enumerate() {
            for (jj, cell) in col.iter().enumerate() {
                if let Some(player) = cell {
                    let (x, y) = (
                        (column_width) * ((ii + 1) as f32) - (column_width / 2.0),
                        (row_height) * ((jj + 1) as f32) - (row_height / 2.0),
                    );
                    let color = player.as_color();
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
        if let Some((_, (start, end))) = &self.state.winner {
            let (w, h) = graphics::drawable_size(ctx);
            let stroke = 2.0;
            let column_size = w / self.state.size as f32;
            let row_size = h / self.state.size as f32;
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

struct Simulator {
    states: Receiver<State>,
    cmds: Sender<Command>,
}

struct StateUpdater {
    state: State,
}

impl Handler for StateUpdater {
    fn on_message(&mut self, msg: Message) -> Result<()> {
        if let Message::Text(txt) = msg {
            self.state = serde_json::from_str(&txt).unwrap();
        }
        Ok(())
    }
}

fn cmd_pump(out: ws::Sender, cmds: Receiver<Command>) {
    std::thread::spawn(move || {
        for cmd in cmds {
            out.send(Message::Text(serde_json::to_string(&cmd).unwrap()))
                .unwrap();
        }
    });
}

// This is our "server".
impl Simulator {
    // new creates a facade that interacts with a websocket endpoint.
    fn new(addr: &str, size: u32, win: u32, gravity: bool) -> Self {
        let (states_tx, states_rx) = unbounded();
        let (cmd_tx, cmd_rx) = unbounded();
        let addr = addr.to_owned();
        std::thread::spawn(move || {
            ws::connect(addr, |out: ws::Sender| {
                for cmd in vec![
                    Command::SetGridSize(size),
                    Command::SetWinCondition(win),
                    Command::SetGravity(gravity),
                    Command::StartGame,
                ] {
                    out.send(Message::Text(serde_json::to_string(&cmd).unwrap()))
                        .unwrap();
                }
                cmd_pump(out, cmd_rx.clone());
                |msg| {
                    if let Message::Text(txt) = msg {
                        let state: State = serde_json::from_str(&txt).unwrap();
                        states_tx.send(state).unwrap();
                    }
                    Ok(())
                }
            })
            .unwrap();
        });
        Simulator {
            cmds: cmd_tx,
            states: states_rx,
        }
    }

    fn push(&mut self, cmd: Command) {
        self.cmds.send(cmd).unwrap();
    }

    fn state(&mut self) -> Option<State> {
        // Grab only the latest state update.
        self.states.try_iter().last()
    }
}

impl event::EventHandler for Client {
    fn update(&mut self, _ctx: &mut ggez::Context) -> ggez::GameResult {
        if let Some(state) = self.sim.state() {
            self.state = Some(state);
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
        if let Some(state) = self.state.take() {
            let (w, h) = graphics::drawable_size(ctx);
            let col = (x / w * state.size as f32).min(state.size as f32 - 1.0) as u32;
            let row = (y / h * state.size as f32).min(state.size as f32 - 1.0) as u32;
            self.state = Some(state);
            self.sim.push(Command::Place(col, row));
        } else {
            // FIXME: Hack to provoke server to give us state.
            self.sim.push(Command::Place(0, 0));
        }
    }

    fn draw(&mut self, ctx: &mut ggez::Context) -> ggez::GameResult {
        graphics::clear(ctx, [0.0, 0.0, 0.0, 0.0].into());
        if let Some(state) = self.state.take() {
            let mut mb = MeshBuilder::new();
            let r = Renderer{state};
            r.draw(ctx, &mut mb)?;
            let mesh = mb.build(ctx)?;
            graphics::draw(ctx, &mesh, graphics::DrawParam::default())?;
            self.state = Some(r.state);
        }
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
        .arg(
            Arg::with_name("addr")
                .required(true)
                .takes_value(true)
                .long("address")
                .short("a")
                .help("Address to connect to."),
        )
        .get_matches();
    let size = matches
        .value_of("size")
        .unwrap_or("3")
        .parse::<u32>()
        .expect("parsing size value");
    let win = matches
        .value_of("win")
        .unwrap_or("3")
        .parse::<u32>()
        .expect("parsing win value");
    let address = matches.value_of("addr").unwrap();
    let gravity = matches.is_present("gravity");
    let cb = ggez::ContextBuilder::new("Tick Tack Toe", "Jack Mordaunt")
        .window_setup(ggez::conf::WindowSetup::default().vsync(true));
    // let state = State::new(size, win, gravity);
    // FIXME: If server controls game state then we needn't setup the state
    // here.
    // Need to delay use of state object until connection to server has been
    // established and state has been copied over to this client.
    let sim = Simulator::new(&format!("ws://{}:8080", address), size, win, gravity);
    let client = &mut Client {
        sim: sim,
        state: None,
    };
    let (ctx, event_loop) = &mut cb.build()?;
    event::run(ctx, event_loop, client)
}
