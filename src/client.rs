#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Player {
    Naughts,
    Crosses,
}

#[derive(Clone, Debug)]
struct Axis((usize, usize), (usize, usize));

// State seen by the client, used to render the game.
struct State {
    winner: Option<(Player, Axis)>,
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
    // Push commands to the simulator.
    fn push(&self, cmd: Command);
    // Receive the updated state from the simulator.
    fn on_state_change(&self, cb: Box<Fn(State)>);
}

// Client transforms hardware events into simulation commands,
// and renders the game state to the screen.
struct Client {
    sim: Box<Simulator>,
    state: State,
}

fn main() {}
