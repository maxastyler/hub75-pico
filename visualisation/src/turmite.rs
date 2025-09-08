use embedded_graphics::{
    Pixel,
    pixelcolor::Rgb888,
    prelude::{Point, RgbColor},
};

use crate::{StateUpdate, Visualisation, grid::Grid};

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum State {
    A,
    B,
}

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum Colour {
    A,
    B,
}

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum Turn {
    Straight,
    Left,
    Right,
    Back,
}

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum Direction {
    Up,
    Left,
    Right,
    Down,
}

pub struct TwoByTwoTurmiteRule([(State, Colour, Turn); 4]);

impl TwoByTwoTurmiteRule {
    pub const fn construct_rule(state: State, colour: Colour, turn: Turn) -> u8 {
        (state as u8 & 0b0000_0001)
            | (((colour as u8) << 1) & 0b0000_0010)
            | (((turn as u8) << 2) & 0b0000_1100)
    }

    /// create the rules from 4 tuples of (state, colour, turn)
    pub const fn new(rules: [(State, Colour, Turn); 4]) -> Self {
        TwoByTwoTurmiteRule(rules)
    }

    pub const fn get(&self, state: State, colour: Colour) -> (State, Colour, Turn) {
        match (state, colour) {
            (State::A, Colour::A) => self.0[0],
            (State::A, Colour::B) => self.0[1],
            (State::B, Colour::A) => self.0[2],
            (State::B, Colour::B) => self.0[3],
        }
    }
}

pub struct TurmiteState {
    internal: State,
    direction: Direction,
    x: i32,
    y: i32,
}

impl TurmiteState {
    pub fn next_x_y(&self) -> (i32, i32) {
        match self.direction {
            Direction::Up => (self.x, self.y - 1),
            Direction::Left => (self.x - 1, self.y),
            Direction::Right => (self.x + 1, self.y),
            Direction::Down => (self.x, self.y + 1),
        }
    }

    pub fn next_x_y_wrapped(&self, width: i32, height: i32) -> (i32, i32) {
        let (mut x, mut y) = self.next_x_y();
        if x < 0 {
            x = width - 1
        }
        if x >= width {
            x = 0
        }
        if y < 0 {
            y = height - 1
        }
        if y >= height {
            y = 0
        }
        (x, y)
    }

    pub fn next_dir(&self, turn: Turn) -> Direction {
        use Direction::*;
        match (self.direction, turn) {
            (Direction::Up, Turn::Straight) => Up,
            (Direction::Up, Turn::Left) => Left,
            (Direction::Up, Turn::Right) => Right,
            (Direction::Up, Turn::Back) => Down,
            (Direction::Left, Turn::Straight) => Left,
            (Direction::Left, Turn::Left) => Up,
            (Direction::Left, Turn::Right) => Down,
            (Direction::Left, Turn::Back) => Right,
            (Direction::Right, Turn::Straight) => Right,
            (Direction::Right, Turn::Left) => Up,
            (Direction::Right, Turn::Right) => Down,
            (Direction::Right, Turn::Back) => Left,
            (Direction::Down, Turn::Straight) => Down,
            (Direction::Down, Turn::Left) => Right,
            (Direction::Down, Turn::Right) => Left,
            (Direction::Down, Turn::Back) => Up,
        }
    }
}

pub struct Turmite<const W: usize, const H: usize>
where
    [(); W * H]:,
{
    rule: TwoByTwoTurmiteRule,
    state: TurmiteState,
    grid: Grid<Colour, W, H>,
}

impl<const W: usize, const H: usize> Turmite<W, H>
where
    [(); W * H]:,
{
    pub fn new() -> Self {
        Turmite {
            rule: TwoByTwoTurmiteRule::new([
                (State::A, Colour::B, Turn::Straight),
                (State::A, Colour::A, Turn::Straight),
                (State::B, Colour::B, Turn::Straight),
                (State::B, Colour::A, Turn::Straight),
            ]),
            state: TurmiteState {
                internal: State::A,
                direction: Direction::Right,
                x: 0,
                y: 0,
            },
            grid: Grid::new(Colour::A),
        }
    }

    fn step(&mut self) {
        if let Some(colour) = self.grid.get_mut(self.state.x, self.state.y) {
            let (new_state, new_colour, turn) = self.rule.get(self.state.internal, *colour);
            self.state.internal = new_state;
            self.state.direction = self.state.next_dir(turn);
            *colour = new_colour;
            let (x, y) = self.state.next_x_y_wrapped(W as i32, H as i32);
            self.state.x = x;
            self.state.y = y;
        }
    }
}

pub struct TurmiteUpdate;

impl StateUpdate for TurmiteUpdate {}

impl<const W: usize, const H: usize> Visualisation for Turmite<W, H>
where
    [(); W * H]:,
{
    type StateUpdate = TurmiteUpdate;

    fn update(&mut self, delta_time_us: u32) -> bool {
        self.step();
        true
    }

    fn draw<
        D: embedded_graphics::prelude::DrawTarget<
                Color = embedded_graphics::pixelcolor::Rgb888,
                Error = core::convert::Infallible,
            >,
    >(
        &mut self,
        target: &mut D,
    ) {
        target
            .draw_iter(self.grid.iter_with_index().map(|((x, y), colour)| {
                Pixel(
                    Point::new(x, y),
                    match colour {
                        Colour::A => Rgb888::BLACK,
                        Colour::B => Rgb888::WHITE,
                    },
                )
            }))
            .unwrap()
    }
}
