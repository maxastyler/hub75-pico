use core::convert::Infallible;

use embedded_graphics::{pixelcolor::BinaryColor, prelude::DrawTarget};
use sand_pile::{SandPile, SandpileStateUpdate};

mod sand_pile;

pub trait StateUpdate {}

pub trait Visualisation {
    type StateUpdate: StateUpdate;
    fn update(&mut self, delta_time: embassy_time::Duration);
    fn draw<D: DrawTarget<Color = BinaryColor, Error = Infallible>>(&mut self, target: &mut D);
}

pub enum CurrentStateUpdate {
    SandPile(SandpileStateUpdate),
}

pub enum CurrentState {
    SandPile(SandPile<32, 64>),
}

pub struct VisualisationState {
    current: CurrentState,
}

impl VisualisationState {
    pub fn new() -> Self {
        VisualisationState {
            current: CurrentState::SandPile(SandPile::new()),
        }
    }
}
