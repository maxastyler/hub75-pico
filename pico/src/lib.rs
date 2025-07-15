#![no_std]
#![no_main]
#![feature(generic_const_exprs)]

mod comms;
mod display;
mod framebuffer;
mod lut;
mod visualisation;

pub const FB_BYTES: usize = fb_bytes(64, 32, 8);

pub use comms::Comms;
pub use display::{Display, fb_bytes};
pub use framebuffer::FrameBuffer;
pub use lut::{GammaLut, Lut, LutState, Init};
