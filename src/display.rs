use embassy_rp::PeripheralRef;
use embassy_rp::pio::{Instance as PioInstance, Pio};

use crate::lut::Lut;

/// Framebuffer size in bytes
#[doc(hidden)]
pub const fn fb_bytes(w: usize, h: usize, b: usize) -> usize {
    w * h / 2 * b
}

/// Computes an array with number of clock ticks to wait for every n-th color bit
const fn delays<const B: usize>() -> [u32; B] {
    let mut ds = [0; B];
    let mut i = 0;
    while i < B {
        ds[i] = (1 << i) - 1;
        i += 1;
    }
    ds
}

pub struct DisplayMemory<const W: usize, const H: usize>
where
    [(); fb_bytes(W, H, 8)]: Sized,
{
    frame_buffer_ptr: [u32; 1],
    frame_buffer_0: [u8; fb_bytes(W, H, 8)],
    frame_buffer_1: [u8; fb_bytes(W, H, 8)],
    delays: [u32; 8],
    delays_ptr: [u32; 1],
    brightness: u8,
}

impl<const W: usize, const H: usize> DisplayMemory<W, H>
where
    [(); fb_bytes(W, H, 8)]: Sized,
{
    pub const fn new() -> Self {
        let frame_buffer_ptr = [0];
        let frame_buffer_0 = [0; fb_bytes(W, H, 8)];
        let frame_buffer_1 = [0; fb_bytes(W, H, 8)];
        let delays = delays();
        let delays_ptr = [0];

        DisplayMemory {
            frame_buffer_ptr,
            frame_buffer_0,
            frame_buffer_1,
            delays,
            delays_ptr,
            brightness: 255,
        }
    }

    pub fn swap_buffers<C: embassy_rp::dma::Channel>(&mut self, fb_loop_ch: &PeripheralRef<'_, C>) {
        if self.frame_buffer_ptr[0] == (self.frame_buffer_0.as_ptr() as u32) {
            self.frame_buffer_ptr[0] = self.frame_buffer_1.as_ptr() as u32;
            while !fb_loop_ch.regs().ctrl_trig().read().busy() {}
            self.frame_buffer_0[0..].fill(0);
        } else {
            self.frame_buffer_ptr[0] = self.frame_buffer_0.as_ptr() as u32;
            while !fb_loop_ch.regs().ctrl_trig().read().busy() {}
            self.frame_buffer_1[0..].fill(0);
        }
    }

    pub fn set_brightness(&mut self, brightness: u8) {
        self.brightness = brightness
    }
}

// struct Display<'a, const W: usize, const H: usize, PIO: PioInstance> {
//     lut: &'a dyn Lut,
//     memory: DisplayMemory<W, H>,
//     pio: Pio<'a, PIO>,
// }
