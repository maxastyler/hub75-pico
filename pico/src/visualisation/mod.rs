use core::pin::pin;
use core::{convert::Infallible, pin::Pin};

use embedded_graphics::{pixelcolor::BinaryColor, prelude::DrawTarget};
use sand_pile::{SandPile, SandpileStateUpdate};

use crate::fb_bytes;

mod sand_pile;

const FB_BYTES: usize = fb_bytes(64, 32, 8);

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

pub struct VisualisationState<'a> {
    current: CurrentState,
    fb_to_write: Option<Pin<&'a mut [u8; FB_BYTES]>>,
    fb_to_send: Option<Pin<&'a mut [u8; FB_BYTES]>>,
}

impl<'a> VisualisationState<'a> {
    pub fn new(
        fb_bytes_1: Pin<&'a mut [u8; FB_BYTES]>,
        fb_bytes_2: Pin<&'a mut [u8; FB_BYTES]>,
    ) -> Self {
        VisualisationState {
            current: CurrentState::SandPile(SandPile::new()),
            fb_to_write: Some(fb_bytes_1),
            fb_to_send: Some(fb_bytes_2),
        }
    }
}
