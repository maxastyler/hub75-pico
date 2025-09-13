#![no_std]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

use core::convert::Infallible;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::DrawTarget;
pub use game_of_life::{GameOfLife, GameOfLifeUpdate};
pub use ising::{Ising, IsingUpdate};
pub use sand_pile::{SandPile, SandPileStateUpdate};
pub use test_vis::{TestVis, TestVisUpdate};
pub use turmite::{Turmite, TurmiteUpdate};

mod game_of_life;
mod grid;
mod ising;
mod sand_pile;
mod test_vis;
mod turmite;

pub trait RngU32 {
    fn next_u32(&mut self) -> u32;
    /// random number between 0 and 1
    fn unit_f32(&mut self) -> f32 {
        let n = (self.next_u32() % 100_000) as f32 / 100_000.0;
        n
    }
}

pub trait StateUpdate: serde::Serialize + for<'de> serde::Deserialize<'de> {}

pub trait Visualisation<Rng: RngU32> {
    type StateUpdate: StateUpdate;
    fn new(rng: Rng) -> Self;
    fn reset(&mut self);
    fn run_state_update(&mut self, state_update: Self::StateUpdate);
    /// The update function, returns true if we should draw a new frame
    fn update(&mut self, delta_time_us: u32) -> bool;
    fn draw<D: DrawTarget<Color = Rgb888, Error = Infallible>>(&mut self, target: &mut D);
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum VisualisationUpdate {
    SandPile(SandPileStateUpdate),
    TestVis(TestVisUpdate),
    GameOfLife(GameOfLifeUpdate),
    Turmite(TurmiteUpdate),
    Ising(IsingUpdate),
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum SetState {
    SandPile,
    TestVis,
    GameOfLife,
    Turmite,
    Ising,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum Action {
    Reset,
    SetVisualisation(SetState),
}

pub enum CurrentVisualisationState<Rng> {
    SandPile(SandPile<Rng, 64, 32>),
    TestVis(TestVis),
    GameOfLife(GameOfLife<Rng, 64, 32>),
    Turmite(Turmite<64, 32>),
    Ising(Ising<Rng, 64, 32>),
}

pub 

impl<Rng: RngU32> CurrentVisualisationState<Rng> {
    pub fn update(&mut self, delta_time_us: u32) -> bool {
        match self {
            CurrentVisualisationState::SandPile(sand_pile) => sand_pile.update(delta_time_us),
            CurrentVisualisationState::TestVis(test_vis) => {
                <TestVis as Visualisation<Rng>>::update(test_vis, delta_time_us)
            }
            CurrentVisualisationState::GameOfLife(s) => s.update(delta_time_us),
            CurrentVisualisationState::Turmite(s) => {
                <Turmite<64, 32> as Visualisation<Rng>>::update(s, delta_time_us)
            }
            CurrentVisualisationState::Ising(s) => s.update(delta_time_us),
        }
    }

    pub fn draw<D: DrawTarget<Color = Rgb888, Error = Infallible>>(&mut self, target: &mut D) {
        match self {
            CurrentVisualisationState::SandPile(sand_pile) => sand_pile.draw(target),
            CurrentVisualisationState::TestVis(test_vis) => {
                <TestVis as Visualisation<Rng>>::draw(test_vis, target)
            }
            CurrentVisualisationState::GameOfLife(s) => s.draw(target),
            CurrentVisualisationState::Turmite(s) => {
                <Turmite<64, 32> as Visualisation<Rng>>::draw(s, target)
            }
            CurrentVisualisationState::Ising(s) => s.draw(target),
        }
    }
}
