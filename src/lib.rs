#![no_std]
#![no_main]
#![feature(generic_const_exprs)]

mod display;
mod framebuffer;
mod lut;

pub use display::{Display, fb_bytes};
pub use framebuffer::FrameBuffer;
pub use lut::{GammaLut, Lut, LutState};
