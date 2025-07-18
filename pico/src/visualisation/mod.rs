use core::pin::pin;
use core::{convert::Infallible, pin::Pin};

use embassy_futures::yield_now;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, RawMutex};
use embassy_sync::signal::Signal;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::{pixelcolor::BinaryColor, prelude::DrawTarget};
use sand_pile::{SandPile, SandpileStateUpdate};

use crate::{FB_BYTES, FrameBuffer, Lut};

mod sand_pile;

pub trait StateUpdate {}

pub trait Visualisation {
    type StateUpdate: StateUpdate;
    /// The update function, returns true if we should draw a new frame
    fn update(&mut self, delta_time: embassy_time::Duration) -> bool;
    fn draw<D: DrawTarget<Color = Rgb888, Error = Infallible>>(&mut self, target: &mut D);
}

pub enum CurrentStateUpdate {
    SandPile(SandpileStateUpdate),
}

pub enum CurrentState {
    SandPile(SandPile<32, 64>),
}

impl CurrentState {
    pub fn update(&mut self, delta_time: embassy_time::Duration) -> bool {
        match self {
            CurrentState::SandPile(sand_pile) => sand_pile.update(delta_time),
        }
    }

    pub fn draw<D: DrawTarget<Color = Rgb888, Error = Infallible>>(&mut self, target: &mut D) {
        match self {
            CurrentState::SandPile(sand_pile) => sand_pile.draw(target),
        }
    }
}

pub struct VisualisationState {
    current: CurrentState,
    filled_framebuffer_signal:
        &'static Signal<CriticalSectionRawMutex, &'static mut [u8; FB_BYTES]>,
    empty_framebuffer_signal: &'static Signal<CriticalSectionRawMutex, &'static mut [u8; FB_BYTES]>,
}

impl VisualisationState {
    pub fn new(
        filled_framebuffer_signal: &'static Signal<
            CriticalSectionRawMutex,
            &'static mut [u8; FB_BYTES],
        >,
        empty_framebuffer_signal: &'static Signal<
            CriticalSectionRawMutex,
            &'static mut [u8; FB_BYTES],
        >,
    ) -> Self {
        VisualisationState {
            current: CurrentState::SandPile(SandPile::new()),
            filled_framebuffer_signal,
            empty_framebuffer_signal,
        }
    }

    pub async fn run(&mut self, lut: &'static impl Lut) {
        let mut last_time = embassy_time::Instant::now();
        loop {
            let new_time = embassy_time::Instant::now();
            if self.current.update(new_time - last_time) {
                let empty_framebuffer = self.empty_framebuffer_signal.wait().await;
                let mut fb: FrameBuffer<64, 32> = FrameBuffer::new(empty_framebuffer, lut, 255);
                self.current.draw(&mut fb);
            }
	    last_time = new_time;
            yield_now().await;
        }
    }
}
