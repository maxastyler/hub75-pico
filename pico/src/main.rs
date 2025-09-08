// translated from https://github.com/kjagiello/hub75-pio-rs/blob/262bca716990f0c7eb54b6d6f40578498a78a505/src/lib.rs

#![no_std]
#![no_main]
#![feature(generic_const_exprs)]

use core::pin::pin;

use defmt::*;
use embassy_executor::{Executor, Spawner};
use embassy_futures::yield_now;
use embassy_rp::Peri;
use embassy_rp::dma::Channel;
use embassy_rp::gpio::{Level, Output, Pin};
use embassy_rp::multicore::{Stack, spawn_core1};
use embassy_rp::pac::DMA;
use embassy_rp::pac::dma::Dma;
use embassy_rp::pac::dma::regs::CtrlTrig;
use embassy_rp::pac::dma::vals::{DataSize, TreqSel};
use embassy_rp::peripherals::{
    DMA_CH0, DMA_CH1, DMA_CH2, DMA_CH3, PIN_0, PIN_1, PIN_2, PIN_3, PIN_4, PIN_5, PIN_6, PIN_7,
    PIN_8, PIN_9, PIN_10, PIN_11, PIN_12, TRNG,
};
use embassy_rp::peripherals::{PIO0, PIO1};
use embassy_rp::pio::{
    Config, Direction, FifoJoin, InterruptHandler as PioInterruptHandler, Pio, ShiftConfig,
    ShiftDirection,
};
use fixed::FixedU32;
use fixed::types::extra::U8;
use hub75_pico::{
    Comms, Display, FB_BYTES, FrameBuffer, GammaLut, Init, Irqs, Lut, fb_bytes, run_display_core,
};
use pio::{ProgramWithDefines, pio_asm};
use static_cell::{ConstStaticCell, StaticCell};
use {defmt_rtt as _, panic_probe as _};

static CORE_1_STACK: ConstStaticCell<Stack<120_000>> = ConstStaticCell::new(Stack::new());
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();

static GAMMA_LUT: StaticCell<GammaLut<Init>> = StaticCell::new();

#[embassy_executor::task]
async fn comms_and_display_runner(spawner: Spawner, p: embassy_rp::Peripherals) {
    const W: usize = 64;
    const H: usize = 32;

    let comms = Comms::<10>::new(
        spawner,
        p.PIN_23,
        p.PIN_25,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH8,
        Pio::new(p.PIO1, Irqs),
    )
    .await;
}

struct DisplayCoreTaskArgs {
    pio: Pio<'static, PIO0>,
    r1: Peri<'static, PIN_0>,
    g1: Peri<'static, PIN_1>,
    b1: Peri<'static, PIN_2>,
    r2: Peri<'static, PIN_3>,
    g2: Peri<'static, PIN_4>,
    b2: Peri<'static, PIN_5>,
    a: Peri<'static, PIN_6>,
    b: Peri<'static, PIN_7>,
    c: Peri<'static, PIN_8>,
    d: Peri<'static, PIN_9>,
    clk: Peri<'static, PIN_10>,
    lat: Peri<'static, PIN_11>,
    oe: Peri<'static, PIN_12>,
    fb_channel: Peri<'static, DMA_CH0>,
    fb_loop_channel: Peri<'static, DMA_CH1>,
    oe_channel: Peri<'static, DMA_CH2>,
    oe_loop_channel: Peri<'static, DMA_CH3>,
}

#[embassy_executor::task]
async fn run_display_core_task(
    frame_buffer_1: *mut [u8; FB_BYTES],
    frame_buffer_2: *mut [u8; FB_BYTES],
    lut: &'static GammaLut<Init>,
    pin_args: DisplayCoreTaskArgs,
) {
    let p = pin_args;

    run_display_core(
        frame_buffer_1,
        frame_buffer_2,
        lut,
        p.pio,
        p.r1,
        p.g1,
        p.b1,
        p.r2,
        p.g2,
        p.b2,
        p.a,
        p.b,
        p.c,
        p.d,
        p.clk,
        p.lat,
        p.oe,
        p.fb_channel,
        p.fb_loop_channel,
        p.oe_channel,
        p.oe_loop_channel,
    )
    .await;
}

#[cortex_m_rt::entry]
fn main() -> ! {
    let p = embassy_rp::init(Default::default());

    let mut fb_1: [u8; FB_BYTES] = [0; FB_BYTES];
    let mut fb_2: [u8; FB_BYTES] = [0; FB_BYTES];
    let lut: &'static GammaLut<Init> = GAMMA_LUT.init(GammaLut::new().init((1.0, 1.0, 1.0)));

    let core_1_stack = CORE_1_STACK.take();

    spawn_core1(p.CORE1, core_1_stack, move || {
        let executor1 = EXECUTOR1.init(Executor::new());
        executor1.run(|spawner| {
            let _ = spawner.spawn(comms_and_display_runner(spawner, unsafe {
                embassy_rp::Peripherals::steal()
            }));
        });
    });

    let executor0 = EXECUTOR0.init(Executor::new());

    executor0.run(move |spawner| {
        unwrap!(spawner.spawn(run_display_core_task(
            &mut fb_1 as *mut [u8; FB_BYTES],
            &mut fb_2 as *mut [u8; FB_BYTES],
            lut,
            DisplayCoreTaskArgs {
                pio: Pio::new(p.PIO0, Irqs),
                r1: p.PIN_0,
                g1: p.PIN_1,
                b1: p.PIN_2,
                r2: p.PIN_3,
                g2: p.PIN_4,
                b2: p.PIN_5,
                a: p.PIN_6,
                b: p.PIN_7,
                c: p.PIN_8,
                d: p.PIN_9,
                clk: p.PIN_10,
                lat: p.PIN_11,
                oe: p.PIN_12,
                fb_channel: p.DMA_CH0,
                fb_loop_channel: p.DMA_CH1,
                oe_channel: p.DMA_CH2,
                oe_loop_channel: p.DMA_CH3,
            }
        )))
    })
}
