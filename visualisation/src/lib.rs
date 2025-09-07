#![no_std]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

use core::convert::Infallible;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::DrawTarget;
pub use sand_pile::{RngU32, SandPile, SandpileStateUpdate};
pub use test_vis::{TestVis, TestVisUpdate};

mod sand_pile;
mod test_vis;

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
}

pub enum CurrentState {
    SandPile(SandPile<32, 64>),
    TestVis(TestVis),
}

impl CurrentState {
    pub fn update(&mut self, delta_time_us: u32) -> bool {
        match self {
            CurrentState::SandPile(sand_pile) => sand_pile.update(delta_time_us),
            CurrentState::TestVis(test_vis) => test_vis.update(delta_time_us),
        }
    }

    pub fn draw<D: DrawTarget<Color = Rgb888, Error = Infallible>>(&mut self, target: &mut D) {
        match self {
            CurrentState::SandPile(sand_pile) => sand_pile.draw(target),
            CurrentState::TestVis(test_vis) => test_vis.draw(target),
        }
    }
}
