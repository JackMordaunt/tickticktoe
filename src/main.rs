#![windows_subsystem = "windows"]

use ggez;
use ggez::event::KeyCode;
use ggez::event::{self, MouseButton};
use ggez::graphics::{self, DrawMode, MeshBuilder};
use ggez::input::keyboard::KeyMods;
use ggez::timer;
use ggez::Context;

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

struct Axis((usize, usize), (usize, usize));

struct MainState {
    winner: Option<(Player, Axis)>,
    turn: Player,
    grid: Vec<Vec<Option<Player>>>,
    size: usize,
    win: usize,
}

impl MainState {
    fn new(size: usize, win: usize) -> ggez::GameResult<MainState> {
        let s = MainState {
            winner: None,
            turn: Player::Naughts,
            grid: vec![vec![None; size]; size],
            size: size,
            win: win,
        };
        Ok(s)
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
        if let Some((_, Axis(start, end))) = &self.winner {
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

impl event::EventHandler for MainState {
    fn update(&mut self, _ctx: &mut ggez::Context) -> ggez::GameResult {
        timer::yield_now();
        Ok(())
    }

    fn key_up_event(&mut self, _ctx: &mut Context, code: KeyCode, _keymods: KeyMods) {
        match code {
            KeyCode::Return => {
                *self = MainState::new(self.size, self.win).unwrap();
            }
            _ => {}
        }
    }

    fn mouse_button_up_event(&mut self, ctx: &mut Context, _btn: MouseButton, x: f32, y: f32) {
        if self.winner.is_some() {
            return;
        }
        let (w, h) = graphics::drawable_size(ctx);
        let col = (x / w * self.size as f32) as usize;
        let row = (y / h * self.size as f32) as usize;
        if self.grid[col][row].is_some() {
            return;
        }
        self.grid[col][row] = Some(self.turn);
        for (forward, backward) in &[
            ((1, 0), (-1, 0)),
            ((0, 1), (0, -1)),
            ((1, 1), (-1, -1)),
            ((-1, 1), (1, -1)),
        ] {
            let forward_count =
                self.check_direction(col as i32, row as i32, forward.0, forward.1, self.turn);
            let backward_count =
                self.check_direction(col as i32, row as i32, backward.0, backward.1, self.turn);
            let count = forward_count + backward_count + 1;
            if count >= self.win {
                self.winner = Some((
                    Player::Crosses,
                    // Calculate the coordinates of the start cell and the end cell.
                    Axis(
                        (
                            (col as i32 + forward.0 * forward_count as i32).max(0) as usize,
                            (row as i32 + forward.1 * forward_count as i32).max(0) as usize,
                        ),
                        (
                            (col as i32 + backward.0 * backward_count as i32).max(0) as usize,
                            (row as i32 + backward.1 * backward_count as i32).max(0) as usize,
                        ),
                    ),
                ));
                break;
            }
        }
        self.turn = match self.turn {
            Player::Naughts => Player::Crosses,
            Player::Crosses => Player::Naughts,
        };
    }

    fn draw(&mut self, ctx: &mut ggez::Context) -> ggez::GameResult {
        graphics::clear(ctx, [0.0, 0.0, 0.0, 0.0].into());
        let mut mb = MeshBuilder::new();
        self.build_grid(ctx, &mut mb)?;
        self.build_players(ctx, &mut mb)?;
        if self.winner.is_some() {
            self.build_throughline(ctx, &mut mb)?;
        }
        let mesh = mb.build(ctx)?;
        graphics::draw(ctx, &mesh, graphics::DrawParam::default())?;
        graphics::present(ctx)?;
        Ok(())
    }
}

use clap::{App, Arg};

pub fn main() -> ggez::GameResult {
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
    let cb = ggez::ContextBuilder::new("Tick Tack Toe", "Jack Mordaunt")
        .window_setup(ggez::conf::WindowSetup::default().vsync(true));
    let (ctx, event_loop) = &mut cb.build()?;
    let state = &mut MainState::new(size, win)?;
    event::run(ctx, event_loop, state)
}
