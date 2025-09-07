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
    Comms, Display, FB_BYTES, FrameBuffer, GammaLut, Init, Irqs, Lut, fb_bytes,
    run_display_core,
};
use pio::{ProgramWithDefines, pio_asm};
use static_cell::{ConstStaticCell, StaticCell};
use {defmt_rtt as _, panic_probe as _};

static CORE_1_STACK: StaticCell<Stack<32768>> = StaticCell::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();

static GAMMA_LUT: StaticCell<GammaLut<Init>> = StaticCell::new();

// #[embassy_executor::task]
// async fn comms_and_display_runner(
//     spawner: Spawner,
//     p: embassy_rp::Peripherals,
//     filled_framebuffer_signal: &'static Signal<
//         CriticalSectionRawMutex,
//         &'static mut [u8; FB_BYTES],
//     >,
//     empty_framebuffer_signal: &'static Signal<CriticalSectionRawMutex, &'static mut [u8; FB_BYTES]>,
//     lut: &'static GammaLut<Init>,
// ) {
//     const W: usize = 64;
//     const H: usize = 32;

//     let comms = Comms::<10>::new(
//         spawner,
//         p.PIN_23,
//         p.PIN_25,
//         p.PIN_24,
//         p.PIN_29,
//         p.DMA_CH6,
//         Pio::new(p.PIO1, Irqs),
//     )
//     .await;

//     // let fb_1 = [0; FB_BYTES];
//     // let fb_2 = [0; FB_BYTES];

//     let mut display: Display<64, 32, PIO0, _, _, _, _> = Display::new(
//         lut,
//         Pio::new(p.PIO0, Irqs),
//         &mut fb_1 as *mut [u8; FB_BYTES],
//         &mut fb_2 as *mut [u8; FB_BYTES],
//         p.PIN_0,
//         p.PIN_1,
//         p.PIN_2,
//         p.PIN_3,
//         p.PIN_4,
//         p.PIN_5,
//         p.PIN_6,
//         p.PIN_7,
//         p.PIN_8,
//         p.PIN_9,
//         p.PIN_10,
//         p.PIN_11,
//         p.PIN_12,
//         p.DMA_CH0,
//         p.DMA_CH1,
//         p.DMA_CH2,
//         p.DMA_CH3,
//     );

//     // loop {
//     //     // if the framebuffer signal is empty, then we can write something to it
//     //     if filled_framebuffer_signal.signaled() {
//     //         if !empty_framebuffer_signal.signaled() {
//     //             let filled_fb = filled_framebuffer_signal.try_take().unwrap();
//     //             let used_fb = display.set_new_framebuffer(filled_fb);
//     //             used_fb.fill(0);
//     //             empty_framebuffer_signal.signal(used_fb);
//     //         }
//     //     }
//     //     yield_now().await;
//     // }
// }

// #[embassy_executor::task]
// async fn run_visualisation(
//     filled_framebuffer_signal: &'static Signal<
//         CriticalSectionRawMutex,
//         &'static mut [u8; FB_BYTES],
//     >,
//     empty_framebuffer_signal: &'static Signal<CriticalSectionRawMutex, &'static mut [u8; FB_BYTES]>,
//     lut: &'static GammaLut<Init>,
// ) {
//     let mut visualisation =
//         VisualisationState::new(filled_framebuffer_signal, empty_framebuffer_signal);
//     visualisation.run(lut).await;
// }

#[embassy_executor::task]
async fn run_display_core_task(
    frame_buffer_1: *mut [u8; FB_BYTES],
    frame_buffer_2: *mut [u8; FB_BYTES],
    lut: &'static GammaLut<Init>,
    pio: Pio<'static, PIO0>,
    r1: PIN_0,
    g1: PIN_1,
    b1: PIN_2,
    r2: PIN_3,
    g2: PIN_4,
    b2: PIN_5,
    a: PIN_6,
    b: PIN_7,
    c: PIN_8,
    d: PIN_9,
    clk: PIN_10,
    lat: PIN_11,
    oe: PIN_12,
    fb_channel: DMA_CH0,
    fb_loop_channel: DMA_CH1,
    oe_channel: DMA_CH2,
    oe_loop_channel: DMA_CH3,
) {
    run_display_core(
        frame_buffer_1,
        frame_buffer_2,
        lut,
        pio,
        r1,
        g1,
        b1,
        r2,
        g2,
        b2,
        a,
        b,
        c,
        d,
        clk,
        lat,
        oe,
        fb_channel,
        fb_loop_channel,
        oe_channel,
        oe_loop_channel,
    )
    .await;
}

// fn display_core_main() {
//     let p = embassy_rp::init(Default::default());

//     let fb_1: &'static mut [u8; FB_BYTES] = FB_1.init([0; FB_BYTES]);
//     let fb_2: &'static mut [u8; FB_BYTES] = FB_2.init([0; FB_BYTES]);
//     let lut: &'static GammaLut<Init> = GAMMA_LUT.init(GammaLut::new().init((1.0, 1.0, 1.0)));

//     let executor0 = EXECUTOR0.init(Executor::new());
//     executor0.run(move |spawner| {
//         unwrap!(spawner.spawn(comms_and_display_runner(
//             spawner,
//             p,
//             filled_framebuffer_signal,
//             empty_framebuffer_signal,
//             lut,
//         )))
//     });
// }

#[cortex_m_rt::entry]
fn main() -> ! {
    let p = embassy_rp::init(Default::default());

    let mut fb_1: [u8; FB_BYTES] = [0; FB_BYTES];
    let mut fb_2: [u8; FB_BYTES] = [0; FB_BYTES];
    let lut: &'static GammaLut<Init> = GAMMA_LUT.init(GammaLut::new().init((1.0, 1.0, 1.0)));

    let executor0 = EXECUTOR0.init(Executor::new()); // filled_framebuffer_signal.signal(fb_1);
    // executor0.run(move |spawner| {
    //     unwrap!(spawner.spawn(comms_and_display_runner(
    //         spawner,
    //         p,
    //         filled_framebuffer_signal,
    //         empty_framebuffer_signal,
    //         lut,
    //     )))
    // });

    executor0.run(move |spawner| {
        unwrap!(spawner.spawn(run_display_core_task(
            &mut fb_1 as *mut [u8; FB_BYTES],
            &mut fb_2 as *mut [u8; FB_BYTES],
            lut,
            Pio::new(p.PIO0, Irqs),
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
            p.DMA_CH3
        )))
    })
}
