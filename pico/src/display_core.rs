use embassy_rp::Peripheral;
use embassy_rp::dma::Channel;
use embassy_rp::pio::{Instance, Pio, PioPin};
use embassy_time::Duration;

use crate::{Display, FB_BYTES, FrameBuffer, Irqs, Lut};
use visualisation::{CurrentState, SandPile, TestVis};

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

pub async fn run_display_core<'a, PIO: Instance, L: Lut + Copy, FB_CH, FB_L_CH, OE_CH, OE_L_CH>(
    frame_buffer_1: *mut [u8; FB_BYTES],
    frame_buffer_2: *mut [u8; FB_BYTES],
    lut: L,
    pio: Pio<'a, PIO>,
    r1: impl Peripheral<P = impl PioPin + 'a> + 'a,
    g1: impl Peripheral<P = impl PioPin + 'a> + 'a,
    b1: impl Peripheral<P = impl PioPin + 'a> + 'a,
    r2: impl Peripheral<P = impl PioPin + 'a> + 'a,
    g2: impl Peripheral<P = impl PioPin + 'a> + 'a,
    b2: impl Peripheral<P = impl PioPin + 'a> + 'a,
    a: impl Peripheral<P = impl PioPin + 'a> + 'a,
    b: impl Peripheral<P = impl PioPin + 'a> + 'a,
    c: impl Peripheral<P = impl PioPin + 'a> + 'a,
    d: impl Peripheral<P = impl PioPin + 'a> + 'a,
    clk: impl Peripheral<P = impl PioPin + 'a> + 'a,
    lat: impl Peripheral<P = impl PioPin + 'a> + 'a,
    oe: impl Peripheral<P = impl PioPin + 'a> + 'a,
    fb_channel: impl Peripheral<P = FB_CH> + 'a,
    fb_loop_channel: impl Peripheral<P = FB_L_CH> + 'a,
    oe_channel: impl Peripheral<P = OE_CH> + 'a,
    oe_loop_channel: impl Peripheral<P = OE_L_CH> + 'a,
) where
    PIO: Instance,
    FB_CH: Channel,
    FB_L_CH: Channel,
    OE_CH: Channel,
    OE_L_CH: Channel,
{
    let mut display: Display<64, 32, PIO, _, _, _, _, _> = Display::new(
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

    let mut state = CurrentState::SandPile(SandPile::new(Trng::new()));

    let mut start_time = embassy_time::Instant::now();

    loop {
        let elapsed = start_time.elapsed();
        start_time = embassy_time::Instant::now();
        state.update(elapsed);
        let mut current_framebuffer = display.get_framebuffer();
        current_framebuffer.fill(0);
        state.draw(&mut current_framebuffer);
        display.swap_framebuffers();
        if let Some(t) = Duration::from_millis(1000 / 60).checked_sub(start_time.elapsed()) {
            embassy_time::Timer::after(t).await;
        }
    }
}
