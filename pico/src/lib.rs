#![no_std]
#![no_main]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

mod comms;
mod display;
mod display_core;
mod framebuffer;
mod lut;
// mod visualisation;

pub const FB_BYTES: usize = fb_bytes(64, 32, 8);

pub use comms::Comms;
pub use display::{Display, fb_bytes};
pub use display_core::run_display_core;
use embassy_rp::{
    bind_interrupts,
    peripherals::{PIO0, PIO1, TRNG},
};
pub use framebuffer::FrameBuffer;
pub use lut::{GammaLut, Init, Lut, LutState};

bind_interrupts!(pub struct Irqs {
    PIO0_IRQ_0 => embassy_rp::pio::InterruptHandler<PIO0>;
    PIO1_IRQ_0 => embassy_rp::pio::InterruptHandler<PIO1>;
    TRNG_IRQ => embassy_rp::trng::InterruptHandler<TRNG>;
});
