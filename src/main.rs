// translated from https://github.com/kjagiello/hub75-pio-rs/blob/262bca716990f0c7eb54b6d6f40578498a78a505/src/lib.rs

#![no_std]
#![no_main]
#![feature(generic_const_exprs)]

use defmt::*;
use embassy_executor::{Executor, Spawner};
use embassy_rp::dma::Channel;
use embassy_rp::gpio::{Level, Output, Pin};
use embassy_rp::multicore::{Stack, spawn_core1};
use embassy_rp::pac::DMA;
use embassy_rp::pac::dma::Dma;
use embassy_rp::pac::dma::regs::CtrlTrig;
use embassy_rp::pac::dma::vals::{DataSize, TreqSel};
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::{
    Config, Direction, FifoJoin, InterruptHandler as PioInterruptHandler, Pio, ShiftConfig,
    ShiftDirection,
};
use embassy_rp::{Peripheral, bind_interrupts};
use embassy_rp::{PeripheralRef, pac};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_time::Timer;
use embedded_graphics::pixelcolor::{Rgb555, Rgb888};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, PrimitiveStyle, StyledDrawable};
use fixed::FixedU32;
use fixed::types::extra::U8;
use pio::{ProgramWithDefines, pio_asm};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use hub75_pico::{Display, GammaLut, fb_bytes};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
});

struct FrameBuffer<'a, const W: usize, const H: usize, FB_CH, FB_L_CH, OE_CH, OE_L_CH>
where
    [(); fb_bytes(W, H, 8)]: Sized,
    FB_CH: Channel,
    FB_L_CH: Channel,
    OE_CH: Channel,
    OE_L_CH: Channel,
{
    data: &'a mut [u8; fb_bytes(W, H, 8)],
    display: &'a Display<'a, W, H, FB_CH, FB_L_CH, OE_CH, OE_L_CH>,
}

impl<'a, const W: usize, const H: usize, FB_CH, FB_L_CH, OE_CH, OE_L_CH> OriginDimensions
    for FrameBuffer<'a, W, H, FB_CH, FB_L_CH, OE_CH, OE_L_CH>
where
    [(); fb_bytes(W, H, 8)]: Sized,
    FB_CH: Channel,
    FB_L_CH: Channel,
    OE_CH: Channel,
    OE_L_CH: Channel,
{
    fn size(&self) -> Size {
        Size::new(W.try_into().unwrap(), H.try_into().unwrap())
    }
}

impl<'a, const W: usize, const H: usize, FB_CH, FB_L_CH, OE_CH, OE_L_CH>
    FrameBuffer<'a, W, H, FB_CH, FB_L_CH, OE_CH, OE_L_CH>
where
    [(); fb_bytes(W, H, 8)]: Sized,
    FB_CH: Channel,
    FB_L_CH: Channel,
    OE_CH: Channel,
    OE_L_CH: Channel,
{
    pub fn new(
        data: &'a mut [u8; fb_bytes(W, H, 8)],
        display: &'a Display<'a, W, H, FB_CH, FB_L_CH, OE_CH, OE_L_CH>,
    ) -> Self {
        FrameBuffer { data, display }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: Rgb888) {
        // invert the screen
        let x = W - 1 - x;
        let y = H - 1 - y;
        // Half of the screen
        let h = y > (H / 2) - 1;
        let shift = if h { 3 } else { 0 };
        let (c_r, c_g, c_b) = self.display.lut.lookup(color);
        let c_r: u16 = ((c_r as f32) * (self.display.brightness as f32 / 255f32)) as u16;
        let c_g: u16 = ((c_g as f32) * (self.display.brightness as f32 / 255f32)) as u16;
        let c_b: u16 = ((c_b as f32) * (self.display.brightness as f32 / 255f32)) as u16;
        let base_idx = x + ((y % (H / 2)) * W * 8);
        for b in 0..8 {
            // Extract the n-th bit of each component of the color and pack them
            let cr = c_r >> b & 0b1;
            let cg = c_g >> b & 0b1;
            let cb = c_b >> b & 0b1;
            let packed_rgb = (cb << 2 | cg << 1 | cr) as u8;
            let idx = base_idx + b * W;
            self.data[idx] &= !(0b111 << shift);
            self.data[idx] |= packed_rgb << shift;
        }
    }
}
impl<'a, const W: usize, const H: usize, FB_CH, FB_L_CH, OE_CH, OE_L_CH> DrawTarget
    for FrameBuffer<'a, W, H, FB_CH, FB_L_CH, OE_CH, OE_L_CH>
where
    [(); fb_bytes(W, H, 8)]: Sized,
    FB_CH: Channel,
    FB_L_CH: Channel,
    OE_CH: Channel,
    OE_L_CH: Channel,
{
    type Color = Rgb888;

    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            if coord.x < <usize as TryInto<i32>>::try_into(W).unwrap()
                && coord.y < <usize as TryInto<i32>>::try_into(H).unwrap()
                && coord.x >= 0
                && coord.y >= 0
            {
                self.set_pixel(coord.x as usize, coord.y as usize, color);
            }
        }

        Ok(())
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    const W: usize = 64;
    const H: usize = 32;
    const B: usize = 8;

    let lut: GammaLut<_> = GammaLut::new().init((1.0, 1.0, 1.0));

    let mut fb_bytes_1 = [0u8; fb_bytes(W, H, B)];
    let mut fb_bytes_2 = [0u8; fb_bytes(W, H, B)];

    let mut display: Display<64, 32, _, _, _, _> = Display::new(
        &lut,
        Pio::new(p.PIO0, Irqs),
        &fb_bytes_1 as *const [u8; fb_bytes(W, H, 8)],
        p.PIN_0,
        p.PIN_1,
        p.PIN_2,
        p.PIN_3,
        p.PIN_4,
        p.PIN_5,
        p.PIN_6,
        p.PIN_7,
        p.PIN_8,
        p.PIN_9,
        p.PIN_10,
        p.PIN_11,
        p.PIN_12,
        p.DMA_CH0,
        p.DMA_CH1,
        p.DMA_CH2,
        p.DMA_CH3,
    );

    let mut t: f32 = 0.0;
    let mut instant = embassy_time::Instant::now();
    let mut reading_fb_1 = true;
    loop {
        let i: i32 = (W / 2) as i32 + (15.0 * libm::sinf(3.0 * t)) as i32;
        let j: i32 = (H / 2) as i32 + (15.0 * libm::cosf(2.1 * t)) as i32;
        let mut framebuffer = if reading_fb_1 {
            FrameBuffer::new(&mut fb_bytes_2, &display)
        } else {
            FrameBuffer::new(&mut fb_bytes_1, &display)
        };

        Circle::with_center(Point::new(i as i32, j as i32), 30)
            .draw_styled(&PrimitiveStyle::with_fill(Rgb888::WHITE), &mut framebuffer)
            .unwrap();
        Circle::with_center(Point::new(i as i32, j as i32), 15)
            .draw_styled(&PrimitiveStyle::with_fill(Rgb888::RED), &mut framebuffer)
            .unwrap();
        Circle::with_center(Point::new((i + 4) as i32, (j + 4) as i32), 4)
            .draw_styled(&PrimitiveStyle::with_fill(Rgb888::BLUE), &mut framebuffer)
            .unwrap();
        Circle::with_center(Point::new(i as i32 - 4, j as i32 - 4), i.max(0) as u32 / 10)
            .draw_styled(&PrimitiveStyle::with_fill(Rgb888::GREEN), &mut framebuffer)
            .unwrap();
        Circle::with_center(Point::new(i as i32 + 6, j as i32 - 8), 4)
            .draw_styled(&PrimitiveStyle::with_fill(Rgb888::YELLOW), &mut framebuffer)
            .unwrap();

        Timer::after_millis(1).await;
        let new = embassy_time::Instant::now();
        t += ((new - instant).as_millis() as f32) / 1000.0;
        instant = new;
        if reading_fb_1 {
            display.set_new_framebuffer(&fb_bytes_2 as *const [u8; fb_bytes(W, H, 8)]);
            fb_bytes_1[..].fill(0);
            reading_fb_1 = false;
        } else {
            display.set_new_framebuffer(&fb_bytes_1 as *const [u8; fb_bytes(W, H, 8)]);
            fb_bytes_2[..].fill(0);
            reading_fb_1 = true;
        }
    }
}
