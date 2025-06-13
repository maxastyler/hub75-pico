use core::marker::PhantomData;
use embedded_graphics::{pixelcolor::Rgb888, prelude::*};
use libm::{powf, roundf};

pub trait Lut {
    fn lookup(&self, color: Rgb888) -> (u16, u16, u16);
}

pub trait LutState {}
pub struct Uninit;
pub struct Init;
impl LutState for Uninit {}
impl LutState for Init {}

pub struct GammaLut<S> {
    r: [u16; 1 << 8],
    g: [u16; 1 << 8],
    b: [u16; 1 << 8],
    _state: PhantomData<S>,
}

impl GammaLut<Uninit> {
    pub const fn new() -> Self {
        Self {
            r: [0; 1 << 8],
            g: [0; 1 << 8],
            b: [0; 1 << 8],
            _state: PhantomData,
        }
    }

    pub fn init(mut self, gamma: (f32, f32, f32)) -> GammaLut<Init> {
        fn calculate_lookup_value(
            index: usize,
            source_max: u16,
            target_max: u16,
            gamma: f32,
        ) -> u16 {
            let max = target_max as f32;
            let remapped = index as f32 / source_max as f32 * max;
            let value = roundf(powf(remapped / max, gamma) * max);
            u16::try_from(value as u32).unwrap_or(0)
        }

        let mut i = 0;
        while i < self.r.len() {
            self.r[i] = calculate_lookup_value(i, Rgb888::MAX_R as u16, (1 << 8) - 1, gamma.0);
            self.g[i] = calculate_lookup_value(i, Rgb888::MAX_G as u16, (1 << 8) - 1, gamma.1);
            self.b[i] = calculate_lookup_value(i, Rgb888::MAX_B as u16, (1 << 8) - 1, gamma.2);
            i += 1;
        }

        GammaLut {
            r: self.r,
            g: self.g,
            b: self.b,
            _state: PhantomData,
        }
    }
}

impl Lut for GammaLut<Init>
where
    [(); 1 << 8]: Sized,
{
    fn lookup(&self, colour: Rgb888) -> (u16, u16, u16) {
        let r = self.r[colour.r() as usize];
        let g = self.g[colour.g() as usize];
        let b = self.b[colour.b() as usize];
        (r, g, b)
    }
}

pub struct Identity;

impl Lut for Identity {
    fn lookup(&self, colour: Rgb888) -> (u16, u16, u16) {
        (colour.r() as u16, colour.g() as u16, colour.b() as u16)
    }
}
