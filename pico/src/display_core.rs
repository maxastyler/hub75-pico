#![allow(non_camel_case_types)]

use embassy_rp::Peri;
use embassy_rp::dma::Channel;
use embassy_rp::pio::{Instance, Pio, PioPin};
use embassy_time::Duration;

use crate::{Display, FB_BYTES, Irqs, Lut};
use visualisation::{CurrentState, GameOfLife, Ising, SandPile, Turmite};

struct Trng<'d> {
    trng: embassy_rp::trng::Trng<'d, embassy_rp::peripherals::TRNG>,
}

impl<'d> Trng<'d> {
    fn new() -> Self {
        Trng {
            trng: embassy_rp::trng::Trng::new(
                unsafe { embassy_rp::peripherals::TRNG::steal() },
                Irqs,
                embassy_rp::trng::Config::default(),
            ),
        }
    }
}

impl<'d> visualisation::RngU32 for Trng<'d> {
    fn next_u32(&mut self) -> u32 {
        self.trng.blocking_next_u32()
    }
}

pub async fn run_display_core<L: Lut + Copy>(
    frame_buffer_1: *mut [u8; FB_BYTES],
    frame_buffer_2: *mut [u8; FB_BYTES],
    lut: L,
    pio: Pio<'static, impl Instance>,
    r1: Peri<'static, impl PioPin>,
    g1: Peri<'static, impl PioPin>,
    b1: Peri<'static, impl PioPin>,
    r2: Peri<'static, impl PioPin>,
    g2: Peri<'static, impl PioPin>,
    b2: Peri<'static, impl PioPin>,
    a: Peri<'static, impl PioPin>,
    b: Peri<'static, impl PioPin>,
    c: Peri<'static, impl PioPin>,
    d: Peri<'static, impl PioPin>,
    clk: Peri<'static, impl PioPin>,
    lat: Peri<'static, impl PioPin>,
    oe: Peri<'static, impl PioPin>,
    fb_channel: Peri<'static, impl Channel>,
    fb_loop_channel: Peri<'static, impl Channel>,
    oe_channel: Peri<'static, impl Channel>,
    oe_loop_channel: Peri<'static, impl Channel>,
) -> ! {
    let mut display: Display<64, 32, _, _, _, _, _, _> = Display::new(
        lut,
        pio,
        frame_buffer_1,
        frame_buffer_2,
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
    );

    // let mut state = CurrentState::GameOfLife(GameOfLife::new_with_random(1000, Trng::new()));
    // let mut turmite = Turmite::new();
    // let mut state: CurrentState<Trng> = CurrentState::Turmite(turmite);
    // let mut state: CurrentState<Trng> = CurrentState::SandPile(SandPile::new(Trng::new()));
    let mut state: CurrentState<Trng> = CurrentState::Ising(Ising::new(1.0, Trng::new()));

    let mut start_time = embassy_time::Instant::now();

    loop {
        let elapsed = start_time.elapsed();
        start_time = embassy_time::Instant::now();
        state.update(elapsed.as_micros() as u32);
        let mut current_framebuffer = display.get_framebuffer();
        current_framebuffer.fill(0);
        state.draw(&mut current_framebuffer);
        display.swap_framebuffers();
        if let Some(t) = Duration::from_millis(1000 / 60).checked_sub(start_time.elapsed()) {
            embassy_time::Timer::after(t).await;
        }
    }
}
