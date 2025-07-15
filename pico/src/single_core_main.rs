// translated from https://github.com/kjagiello/hub75-pio-rs/blob/262bca716990f0c7eb54b6d6f40578498a78a505/src/lib.rs

#![no_std]
#![no_main]
#![feature(generic_const_exprs)]

use core::pin::pin;

use defmt::*;
use embassy_executor::{Executor, Spawner};
use embassy_futures::yield_now;
use embassy_rp::dma::Channel;
use embassy_rp::gpio::{Level, Output, Pin};
use embassy_rp::multicore::{Stack, spawn_core1};
use embassy_rp::pac::DMA;
use embassy_rp::pac::dma::Dma;
use embassy_rp::pac::dma::regs::CtrlTrig;
use embassy_rp::pac::dma::vals::{DataSize, TreqSel};
use embassy_rp::peripherals::{PIO0, PIO1};
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

use hub75_pico::{Comms, Display, FrameBuffer, GammaLut, fb_bytes};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
    PIO1_IRQ_0 => PioInterruptHandler<PIO1>;
});


#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    const W: usize = 64;
    const H: usize = 32;
    const B: usize = 8;

    let lut: GammaLut<_> = GammaLut::new().init((1.0, 1.0, 1.0));

    let mut fb_bytes_1 = pin!([0u8; fb_bytes(W, H, B)]);
    let mut fb_bytes_2 = pin!([0u8; fb_bytes(W, H, B)]);

    let comms = Comms::<10>::new(
        spawner,
        p.PIN_23,
        p.PIN_25,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH6,
        Pio::new(p.PIO1, Irqs),
    )
    .await;

    let mut display: Display<64, 32, _, _, _, _> = Display::new(
        &lut,
        Pio::new(p.PIO0, Irqs),
        fb_bytes_1.as_ptr() as *const [u8; fb_bytes(W, H, 8)],
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
        for _ in 0..10 {
            Circle::with_center(Point::new(i as i32 + 6, j as i32 - 8), 4)
                .draw_styled(&PrimitiveStyle::with_fill(Rgb888::YELLOW), &mut framebuffer)
                .unwrap();
        }

        let new = embassy_time::Instant::now();
        t += ((new - instant).as_millis() as f32) / 1000.0;
        instant = new;
        if reading_fb_1 {
            display.set_new_framebuffer(&*fb_bytes_2 as *const [u8; fb_bytes(W, H, 8)]);
            fb_bytes_1[..].fill(0);
            reading_fb_1 = false;
        } else {
            display.set_new_framebuffer(&*fb_bytes_1 as *const [u8; fb_bytes(W, H, 8)]);
            fb_bytes_2[..].fill(0);
            reading_fb_1 = true;
        }
        yield_now().await;
    }
}
