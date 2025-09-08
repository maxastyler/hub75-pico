#![no_std]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

use core::convert::Infallible;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::DrawTarget;
pub use game_of_life::{GameOfLife, GameOfLifeUpdate};
pub use sand_pile::{SandPile, SandpileStateUpdate};
pub use test_vis::{TestVis, TestVisUpdate};
pub use turmite::{Turmite, TurmiteUpdate};

mod game_of_life;
mod grid;
mod sand_pile;
mod test_vis;
mod turmite;

pub trait RngU32 {
    fn next_u32(&mut self) -> u32;
}

pub trait StateUpdate {}

pub trait Visualisation {
    type StateUpdate: StateUpdate;
    /// The update function, returns true if we should draw a new frame
    fn update(&mut self, delta_time_us: u32) -> bool;
    fn draw<D: DrawTarget<Color = Rgb888, Error = Infallible>>(&mut self, target: &mut D);
}

pub enum CurrentStateUpdate {
    SandPile(SandpileStateUpdate),
    TestVis(TestVisUpdate),
    GameOfLife(GameOfLifeUpdate),
    Turmite(TurmiteUpdate),
}

pub enum CurrentState<Rng> {
    SandPile(SandPile<Rng, 64, 32>),
    TestVis(TestVis),
    GameOfLife(GameOfLife<Rng, 64, 32>),
    Turmite(Turmite<64, 32>),
}

impl<Rng: RngU32> CurrentState<Rng> {
    pub fn update(&mut self, delta_time_us: u32) -> bool {
        match self {
            CurrentState::SandPile(sand_pile) => sand_pile.update(delta_time_us),
            CurrentState::TestVis(test_vis) => test_vis.update(delta_time_us),
            CurrentState::GameOfLife(s) => s.update(delta_time_us),
            CurrentState::Turmite(s) => s.update(delta_time_us),
        }
    }

    pub fn draw<D: DrawTarget<Color = Rgb888, Error = Infallible>>(&mut self, target: &mut D) {
        match self {
            CurrentState::SandPile(sand_pile) => sand_pile.draw(target),
            CurrentState::TestVis(test_vis) => test_vis.draw(target),
            CurrentState::GameOfLife(s) => s.draw(target),
            CurrentState::Turmite(s) => s.draw(target),
        }
    }
}
