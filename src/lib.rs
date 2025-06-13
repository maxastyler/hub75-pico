#![no_std]
#![no_main]
#![feature(generic_const_exprs)]

mod display;
mod lut;

pub use display::{Display, fb_bytes};
pub use lut::{GammaLut, Lut, LutState};
