use core::pin::pin;
use core::{convert::Infallible, pin::Pin};

use embassy_futures::yield_now;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, RawMutex};
use embassy_sync::signal::Signal;
use embedded_graphics::{pixelcolor::BinaryColor, prelude::DrawTarget};
use sand_pile::{SandPile, SandpileStateUpdate};

use crate::FB_BYTES;

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

pub struct VisualisationState<'a> {
    current: CurrentState,
    fb_to_write: Option<&'a mut [u8; FB_BYTES]>,
    fb_to_send: Option<&'a mut [u8; FB_BYTES]>,
}

impl<'a> VisualisationState<'a> {
    pub fn new(fb_bytes: &'a mut [u8; FB_BYTES]) -> Self {
        VisualisationState {
            current: CurrentState::SandPile(SandPile::new()),
            fb_to_write: Some(fb_bytes),
            fb_to_send: None,
        }
    }

    pub async fn run(&mut self) {
        loop {
            yield_now().await;
        }
    }
}
