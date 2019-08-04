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

enum Axis {
    Column(usize),
    Row(usize),
    LeftDiagonal,
    RightDiagonal,
}

struct MainState {
    winner: Option<(Player, Axis)>,
    turn: Player,
    grid: Vec<Vec<Option<Player>>>,
    rows: usize,
    columns: usize,
    row_scores: Vec<(usize, usize)>,
    column_scores: Vec<(usize, usize)>,
    diagonal_scores: Vec<(usize, usize)>,
}

impl MainState {
    fn new() -> ggez::GameResult<MainState> {
        let (rows, columns) = (3, 3);
        let s = MainState {
            winner: None,
            turn: Player::Naughts,
            grid: vec![vec![None; rows]; columns],
            rows: rows,
            columns: columns,
            row_scores: vec![(0, 0); rows],
            column_scores: vec![(0, 0); columns],
            diagonal_scores: vec![(0, 0); 2],
        };
        Ok(s)
    }

    fn build_grid(&self, ctx: &ggez::Context, mb: &mut MeshBuilder) -> ggez::GameResult {
        let ((w, h), stroke, color) = (graphics::drawable_size(ctx), 2.0, self.turn.color());
        let column_width = w / self.columns as f32;
        for ii in 1..self.columns {
            let offset = column_width * ii as f32;
            mb.line(&[[offset, 0.0], [offset, h]], stroke, color)?;
        }
        let row_height = h / self.rows as f32;
        for ii in 1..self.rows {
            let offset = row_height * ii as f32;
            mb.line(&[[0.0, offset], [w, offset]], stroke, color)?;
        }
        Ok(())
    }

    fn build_players(&self, ctx: &ggez::Context, mb: &mut MeshBuilder) -> ggez::GameResult {
        let ((w, h), size) = (graphics::drawable_size(ctx), 16.0);
        let column_width = w / self.columns as f32;
        let row_height = h / self.rows as f32;
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
        if let Some((_, axis)) = &self.winner {
            let (w, h) = graphics::drawable_size(ctx);
            let padding = 20.0;
            let stroke = 2.0;
            let coords = match axis {
                Axis::Column(n) => {
                    let n = (*n) as f32;
                    let column_size = w / self.columns as f32;
                    let x = column_size * n + column_size / 2.0 - stroke / 2.0;
                    [[x, padding], [x, h - padding]]
                }
                Axis::Row(n) => {
                    let n = (*n) as f32;
                    let row_size = h / self.rows as f32;
                    let y = row_size * n + row_size / 2.0 - stroke / 2.0;
                    [[padding, y], [w - padding, y]]
                }
                Axis::LeftDiagonal => [[padding, padding], [w - padding, h - padding]],
                Axis::RightDiagonal => [[w - padding, padding], [padding, h - padding]],
            };
            mb.line(&coords, stroke, [1.0, 1.0, 1.0, 1.0].into())?;
        }
        Ok(())
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
                *self = MainState::new().unwrap();
            }
            _ => {}
        }
    }

    fn mouse_button_up_event(&mut self, ctx: &mut Context, _btn: MouseButton, x: f32, y: f32) {
        if self.winner.is_some() {
            return;
        }
        let (w, h) = graphics::drawable_size(ctx);
        let col = (x / w * self.columns as f32) as usize;
        let row = (y / h * self.rows as f32) as usize;
        if self.grid[col][row].is_some() {
            return;
        }
        self.grid[col][row] = Some(self.turn);
        match self.turn {
            Player::Crosses => {
                self.row_scores[row].0 += 1;
                self.column_scores[col].0 += 1;
                if row == col {
                    self.diagonal_scores[0].0 += 1;
                }
                if row + col == self.columns.min(self.rows) - 1 {
                    self.diagonal_scores[1].0 += 1;
                }
            }
            Player::Naughts => {
                self.row_scores[row].1 += 1;
                self.column_scores[col].1 += 1;
                if row == col {
                    self.diagonal_scores[0].1 += 1;
                }
                if row + col == self.columns.min(self.rows) - 1 {
                    self.diagonal_scores[1].1 += 1;
                }
            }
        };
        // Check win condition.
        if self.row_scores[row] == (self.rows, 0) {
            self.winner = Some((Player::Crosses, Axis::Row(row)));
        }
        if self.row_scores[row] == (0, self.rows) {
            self.winner = Some((Player::Naughts, Axis::Row(row)));
        }
        if self.column_scores[col] == (self.columns, 0) {
            self.winner = Some((Player::Crosses, Axis::Column(col)));
        }
        if self.column_scores[col] == (0, self.columns) {
            self.winner = Some((Player::Naughts, Axis::Column(col)));
        }
        if self.diagonal_scores[0] == (3, 0) {
            self.winner = Some((Player::Crosses, Axis::LeftDiagonal));
        }
        if self.diagonal_scores[0] == (0, 3) {
            self.winner = Some((Player::Naughts, Axis::LeftDiagonal));
        }
        if self.diagonal_scores[1] == (3, 0) {
            self.winner = Some((Player::Crosses, Axis::RightDiagonal));
        }
        if self.diagonal_scores[1] == (0, 3) {
            self.winner = Some((Player::Naughts, Axis::RightDiagonal));
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

pub fn main() -> ggez::GameResult {
    let cb = ggez::ContextBuilder::new("Tick Tack Toe", "Jack Mordaunt")
        .window_setup(ggez::conf::WindowSetup::default().vsync(true));
    let (ctx, event_loop) = &mut cb.build()?;
    let state = &mut MainState::new()?;
    event::run(ctx, event_loop, state)
}
