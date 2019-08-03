#![windows_subsystem = "windows"]

use ggez;
use ggez::event::KeyCode;
use ggez::event::{self, MouseButton};
use ggez::graphics::{self, DrawMode};
use ggez::input::keyboard::KeyMods;
use ggez::timer;
use ggez::Context;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Player {
    Naughts,
    Crosses,
}

impl Into<graphics::Color> for Player {
    fn into(self) -> graphics::Color {
        match self {
            Player::Naughts => [1.0, 0.647, 0.0, 1.0].into(),
            Player::Crosses => [0.0, 0.35, 1.0, 1.0].into(),
        }
    }
}

struct MainState {
    winner: Option<Player>,
    turn: Player,
    grid: [[Option<Player>; 3]; 3],
}

impl MainState {
    fn new() -> ggez::GameResult<MainState> {
        let s = MainState {
            winner: None,
            turn: Player::Naughts,
            grid: [[None; 3]; 3],
        };
        Ok(s)
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
                self.winner = None;
                self.grid = [[None; 3]; 3];
            }
            _ => {}
        }
    }

    // FIXME: Win detection and cell detection are hardcoded to a 3x3 grid.
    fn mouse_button_up_event(&mut self, ctx: &mut Context, _btn: MouseButton, x: f32, y: f32) {
        if self.winner.is_some() {
            return;
        }
        let (w, h) = graphics::size(ctx);
        let col = if x < w / 3.0 {
            0
        } else if x < 2.0 * (w / 3.0) {
            1
        } else {
            2
        };
        let row = if y < h / 3.0 {
            0
        } else if y < 2.0 * (h / 3.0) {
            1
        } else {
            2
        };
        if self.grid[col][row].is_some() {
            return;
        }
        self.grid[col][row] = Some(self.turn);
        self.turn = match self.turn {
            Player::Naughts => Player::Crosses,
            Player::Crosses => Player::Naughts,
        };
        // Check win condition.
        let grid = self.grid;
        // column 0
        if grid[0][0].is_some() && grid[0][0] == grid[0][1] && grid[0][1] == grid[0][2] {
            self.winner = grid[0][0];
        }
        // column 1
        if grid[1][0].is_some() && grid[1][0] == grid[1][1] && grid[1][1] == grid[1][2] {
            self.winner = grid[1][0];
        }
        // column 2
        if grid[2][0].is_some() && grid[2][0] == grid[2][1] && grid[2][1] == grid[2][2] {
            self.winner = grid[2][0];
        }
        // row 0
        if grid[0][0].is_some() && grid[0][0] == grid[1][0] && grid[1][0] == grid[2][0] {
            self.winner = grid[0][0];
        }
        // row 1
        if grid[0][1].is_some() && grid[0][1] == grid[1][1] && grid[1][1] == grid[2][1] {
            self.winner = grid[0][1];
        }
        // row 2
        if grid[0][2].is_some() && grid[0][2] == grid[1][2] && grid[1][2] == grid[2][2] {
            self.winner = grid[0][2];
        }
        // diag 0
        if grid[0][0].is_some() && grid[0][0] == grid[1][1] && grid[1][1] == grid[2][2] {
            self.winner = grid[0][0];
        }
        // diag 1
        if grid[0][2].is_some() && grid[0][2] == grid[1][1] && grid[1][1] == grid[2][0] {
            self.winner = grid[0][2];
        }
    }

    fn draw(&mut self, ctx: &mut ggez::Context) -> ggez::GameResult {
        graphics::clear(ctx, [0.0, 0.0, 0.0, 0.0].into());
        // draw grid
        let (w, h) = graphics::size(ctx);
        let mut mb = graphics::MeshBuilder::new();
        mb.line(&[[w / 3.0, 000.0], [w / 3.0, h]], 2.0, self.turn.into())?
            .line(
                &[[(w / 3.0) * 2.0, 000.0], [(w / 3.0) * 2.0, h]],
                2.0,
                self.turn.into(),
            )?
            .line(&[[0.0, h / 3.0], [w, h / 3.0]], 2.0, self.turn.into())?
            .line(
                &[[0.0, (h / 3.0) * 2.0], [w, (h / 3.0) * 2.0]],
                2.0,
                self.turn.into(),
            )?;
        // draw players
        for (ii, col) in self.grid.iter().enumerate() {
            for (jj, cell) in col.iter().enumerate() {
                if let Some(player) = cell {
                    let (x, y) = (
                        (w / 3.0) * ((ii + 1) as f32) - (w / 6.0),
                        (h / 3.0) * ((jj + 1) as f32) - (h / 6.0),
                    );
                    match player {
                        Player::Naughts => {
                            mb.circle(DrawMode::stroke(2.0), [x, y], 32.0, 0.1, (*player).into());
                        }
                        Player::Crosses => {
                            mb.line(
                                &[[x - 32.0, y - 32.0], [x + 32.0, y + 32.0]],
                                2.0,
                                (*player).into(),
                            )?;
                            mb.line(
                                &[[x + 32.0, y - 32.0], [x - 32.0, y + 32.0]],
                                2.0,
                                (*player).into(),
                            )?;
                        }
                    }
                }
            }
        }
        let mesh = mb.build(ctx)?;
        graphics::draw(ctx, &mesh, graphics::DrawParam::default())?;
        graphics::present(ctx)?;
        Ok(())
    }
}

pub fn main() -> ggez::GameResult {
    let cb = ggez::ContextBuilder::new("super_simple", "ggez")
        .window_setup(ggez::conf::WindowSetup::default().vsync(true));
    let (ctx, event_loop) = &mut cb.build()?;
    let state = &mut MainState::new()?;
    event::run(ctx, event_loop, state)
}
