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
use lut::{GammaLut, Identity, Lut};
use pio::{ProgramWithDefines, pio_asm};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

mod lut;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
});

/// Framebuffer size in bytes
#[doc(hidden)]
pub const fn fb_bytes(w: usize, h: usize, b: usize) -> usize {
    w * h / 2 * b
}

pub struct DisplayMemory<'a, const W: usize, const H: usize, const B: usize>
where
    [(); fb_bytes(W, H, B)]: Sized,
{
    fbptr: [u32; 1],
    fb0: [u8; fb_bytes(W, H, B)],
    fb1: [u8; fb_bytes(W, H, B)],
    delays: [u32; B],
    delaysptr: [u32; 1],
    lut: &'a dyn Lut,
    brightness: u8,
}

/// Computes an array with number of clock ticks to wait for every n-th color bit
const fn delays<const B: usize>() -> [u32; B] {
    let mut arr = [0; B];
    let mut i = 0;
    while i < arr.len() {
        arr[i] = (1 << i) - 1;
        i += 1;
    }
    arr
}

impl<'a, const W: usize, const H: usize, const B: usize> DisplayMemory<'a, W, H, B>
where
    [(); fb_bytes(W, H, B)]: Sized,
{
    pub const fn new(lut: &'a impl lut::Lut) -> Self {
        let fb0 = [0; fb_bytes(W, H, B)];
        let fb1 = [0; fb_bytes(W, H, B)];
        let fbptr: [u32; 1] = [0];
        let delays = delays();
        let delaysptr: [u32; 1] = [0];
        DisplayMemory {
            fbptr,
            fb0,
            fb1,
            delays,
            delaysptr,
            lut,
            brightness: 255,
        }
    }

    pub fn swap_buffers<Ch: embassy_rp::dma::Channel>(
        &mut self,
        fb_loop_ch: &PeripheralRef<'_, Ch>,
    ) {
        if self.fbptr[0] == (self.fb0.as_ptr() as u32) {
            self.fbptr[0] = self.fb1.as_ptr() as u32;
            while !fb_loop_ch.regs().ctrl_trig().read().busy() {}
            self.fb0[0..].fill(0);
        } else {
            self.fbptr[0] = self.fb0.as_ptr() as u32;
            while !fb_loop_ch.regs().ctrl_trig().read().busy() {}
            self.fb1[0..].fill(0);
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: Rgb888) {
        // invert the screen
        let x = W - 1 - x;
        let y = H - 1 - y;
        // Half of the screen
        let h = y > (H / 2) - 1;
        let shift = if h { 3 } else { 0 };
        let (c_r, c_g, c_b) = self.lut.lookup(color);
        let c_r: u16 = ((c_r as f32) * (self.brightness as f32 / 255f32)) as u16;
        let c_g: u16 = ((c_g as f32) * (self.brightness as f32 / 255f32)) as u16;
        let c_b: u16 = ((c_b as f32) * (self.brightness as f32 / 255f32)) as u16;
        let base_idx = x + ((y % (H / 2)) * W * B);
        for b in 0..B {
            // Extract the n-th bit of each component of the color and pack them
            let cr = c_r >> b & 0b1;
            let cg = c_g >> b & 0b1;
            let cb = c_b >> b & 0b1;
            let packed_rgb = (cb << 2 | cg << 1 | cr) as u8;
            let idx = base_idx + b * W;
            if self.fbptr[0] == (self.fb0.as_ptr() as u32) {
                self.fb1[idx] &= !(0b111 << shift);
                self.fb1[idx] |= packed_rgb << shift;
            } else {
                self.fb0[idx] &= !(0b111 << shift);
                self.fb0[idx] |= packed_rgb << shift;
            }
        }
    }

    pub fn set_brightness(&mut self, brightness: u8) {
        self.brightness = brightness
    }
}

impl<'a, const W: usize, const H: usize, const B: usize> OriginDimensions
    for DisplayMemory<'a, W, H, B>
where
    [(); fb_bytes(W, H, B)]: Sized,
{
    fn size(&self) -> Size {
        Size::new(W.try_into().unwrap(), H.try_into().unwrap())
    }
}

impl<'a, const W: usize, const H: usize, const B: usize> DrawTarget for DisplayMemory<'a, W, H, B>
where
    [(); fb_bytes(W, H, B)]: Sized,
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

    let lut = Identity;

    const W: usize = 64;
    const H: usize = 32;
    const B: usize = 8;

    let lut: GammaLut<_> = GammaLut::new().init((1.0, 1.0, 1.0));

    let mut dm: DisplayMemory<W, H, B> = DisplayMemory::new(&lut);

    let pio = p.PIO0;
    let Pio {
        mut common,
        mut sm0,
        mut sm1,
        mut sm2,
        ..
    } = Pio::new(pio, Irqs);

    let mut r1 = common.make_pio_pin(p.PIN_0);
    let mut g1 = common.make_pio_pin(p.PIN_1);
    let mut b1 = common.make_pio_pin(p.PIN_2);
    let mut r2 = common.make_pio_pin(p.PIN_3);
    let mut g2 = common.make_pio_pin(p.PIN_4);
    let mut b2 = common.make_pio_pin(p.PIN_5);
    let mut a = common.make_pio_pin(p.PIN_6);
    let mut b = common.make_pio_pin(p.PIN_7);
    let mut c = common.make_pio_pin(p.PIN_8);
    let mut d = common.make_pio_pin(p.PIN_9);
    let mut clk = common.make_pio_pin(p.PIN_10);
    let mut lat = common.make_pio_pin(p.PIN_11);
    let mut oe = common.make_pio_pin(p.PIN_12);

    let rgb_prog = pio::pio_asm!(
        ".side_set 1",
        "out isr, 32    side 0b0",
        ".wrap_target",
        "mov x isr      side 0b0",
        // Wait for the row program to set the ADDR pins
        "pixel:",
        "out pins, 8    side 0b0",
        "jmp x-- pixel  side 0b1", // clock out the pixel
        "irq 4          side 0b0", // tell the row program to set the next row
        "wait 1 irq 5   side 0b0",
        ".wrap",
    );

    let cfg = {
        let mut cfg = Config::default();
        cfg.use_program(&common.load_program(&rgb_prog.program), &[&clk]);
        cfg.set_out_pins(&[&r1, &g1, &b1, &r2, &g2, &b2]);
        cfg.clock_divider = fixed::FixedU32::from_num(4);
        cfg.shift_out.direction = ShiftDirection::Right;
        cfg.shift_out.auto_fill = true;
        cfg.fifo_join = FifoJoin::TxOnly;
        cfg
    };

    sm0.set_config(&cfg);
    sm0.set_pin_dirs(Direction::Out, &[&r1, &g1, &b1, &r2, &g2, &b2, &clk]);
    sm0.set_enable(true);
    sm0.tx().push(W as u32 - 1);

    let row_prog = pio::pio_asm!(
        ".side_set 1",
        "pull           side 0b0", // Pull the height / 2 into OSR
        "out isr, 32    side 0b0", // and move it to OSR
        "pull           side 0b0", // Pull the color depth - 1 into OSR
        ".wrap_target",
        "mov x, isr     side 0b0",
        "addr:",
        "mov pins, ~x   side 0b0", // Set the row address
        "mov y, osr     side 0b0",
        "row:",
        "wait 1 irq 4   side 0b0", // Wait until the data is clocked in
        "nop            side 0b1",
        "irq 6          side 0b1", // Display the latched data
        "irq 5          side 0b0", // Clock in next row
        "wait 1 irq 7   side 0b0", // Wait for the OE cycle to complete
        "jmp y-- row    side 0b0",
        "jmp x-- addr   side 0b0",
        ".wrap",
    );

    let other_frac = 1.2;

    let cfg = {
        let mut cfg = Config::default();
        cfg.use_program(&common.load_program(&row_prog.program), &[&lat]);
        cfg.set_out_pins(&[&a, &b, &c, &d]);
        cfg.clock_divider = FixedU32::<U8>::from_num(other_frac);
        cfg
    };

    sm1.set_config(&cfg);
    sm1.set_pin_dirs(Direction::Out, &[&a, &b, &c, &d, &lat]);
    sm1.set_enable(true);
    sm1.tx().push(H as u32 / 2 - 1);
    sm1.tx().push(B as u32 - 1);

    let delay_prog = pio::pio_asm!(
        ".side_set 1",
        ".wrap_target",
        "out x, 32      side 0b1",
        "wait 1 irq 6   side 0b1",
        "delay:",
        "jmp x-- delay  side 0b0",
        "irq 7          side 0b1",
        ".wrap",
    );

    let cfg = {
        let mut cfg = Config::default();
        cfg.use_program(&common.load_program(&delay_prog.program), &[&oe]);
        cfg.clock_divider = FixedU32::<U8>::from_num(other_frac);
        cfg.shift_out.auto_fill = true;
        cfg.fifo_join = FifoJoin::TxOnly;
        cfg
    };

    sm2.set_config(&cfg);
    sm2.set_pin_dirs(Direction::Out, &[&oe]);
    sm2.set_enable(true);

    let mut fb_ch = p.DMA_CH0.into_ref();
    let mut fb_loop_ch = p.DMA_CH1.into_ref();
    let mut oe_ch = p.DMA_CH2.into_ref();
    let mut oe_loop_ch = p.DMA_CH3.into_ref();

    dm.fbptr[0] = dm.fb0.as_ptr() as u32;
    dm.delaysptr[0] = dm.delays.as_ptr() as u32;
    fb_ch.regs().al1_ctrl().write(|c| {
        let mut t = CtrlTrig(*c);
        t.set_incr_read(true);
        t.set_incr_write(false);
        t.set_data_size(DataSize::SIZE_WORD);
        t.set_treq_sel(TreqSel::PIO0_TX0);
        t.set_irq_quiet(true);
        t.set_chain_to(fb_loop_ch.number());
        t.set_en(true);
        *c = t.0;
    });

    fb_ch.regs().read_addr().write(|c| *c = dm.fbptr[0]);
    fb_ch
        .regs()
        .trans_count()
        .write(|c| c.0 = fb_bytes(W, H, B) as u32 / 4);
    fb_ch
        .regs()
        .write_addr()
        .write(|c| *c = pac::PIO0.txf(0).as_ptr() as u32);

    fb_loop_ch.regs().al1_ctrl().write(|c| {
        let mut t = CtrlTrig(*c);
        t.set_incr_read(false);
        t.set_incr_write(false);
        t.set_data_size(DataSize::SIZE_WORD);
        t.set_treq_sel(TreqSel::PERMANENT);
        t.set_irq_quiet(true);
        t.set_chain_to(fb_ch.number());
        t.set_en(true);
        *c = t.0;
    });

    fb_loop_ch
        .regs()
        .read_addr()
        .write(|c| *c = dm.fbptr.as_ptr() as u32);
    fb_loop_ch.regs().trans_count().write(|c| c.0 = 1);
    fb_loop_ch
        .regs()
        .al2_write_addr_trig()
        .write(|c| *c = fb_ch.regs().read_addr().as_ptr() as u32);

    oe_ch.regs().al1_ctrl().write(|c| {
        let mut t = CtrlTrig(*c);
        t.set_incr_read(true);
        t.set_incr_write(false);
        t.set_data_size(DataSize::SIZE_WORD);
        t.set_treq_sel(TreqSel::PIO0_TX2);
        t.set_irq_quiet(true);
        t.set_chain_to(oe_loop_ch.number());
        t.set_en(true);
        *c = t.0;
    });
    oe_ch
        .regs()
        .read_addr()
        .write(|c| *c = dm.delays.as_ptr() as u32);
    oe_ch
        .regs()
        .trans_count()
        .write(|c| c.0 = dm.delays.len() as u32);
    oe_ch
        .regs()
        .write_addr()
        .write(|c| *c = pac::PIO0.txf(2).as_ptr() as u32);

    oe_loop_ch.regs().al1_ctrl().write(|c| {
        let mut t = CtrlTrig(*c);

        t.set_incr_read(false);
        t.set_incr_write(false);
        t.set_data_size(DataSize::SIZE_WORD);
        t.set_treq_sel(TreqSel::PERMANENT);
        t.set_irq_quiet(true);
        t.set_chain_to(oe_ch.number());
        t.set_en(true);

        *c = t.0;
    });

    oe_loop_ch
        .regs()
        .read_addr()
        .write(|c| *c = dm.delaysptr.as_ptr() as u32);
    oe_loop_ch
        .regs()
        .trans_count()
        .write(|c| c.0 = dm.delaysptr.len() as u32);
    oe_loop_ch
        .regs()
        .al2_write_addr_trig()
        .write(|c| *c = oe_ch.regs().read_addr().as_ptr() as u32);

    let mut t: f32 = 0.0;
    let mut instant = embassy_time::Instant::now();
    loop {
        let i: i32 = (W / 2) as i32 + (15.0 * libm::sinf(3.0 * t)) as i32;
        let j: i32 = (H / 2) as i32 + (15.0 * libm::cosf(2.1 * t)) as i32;
        Circle::with_center(Point::new(i as i32, j as i32), 30)
            .draw_styled(&PrimitiveStyle::with_fill(Rgb888::WHITE), &mut dm)
            .unwrap();
        Circle::with_center(Point::new(i as i32, j as i32), 15)
            .draw_styled(&PrimitiveStyle::with_fill(Rgb888::RED), &mut dm)
            .unwrap();
        Circle::with_center(Point::new((i + 4) as i32, (j + 4) as i32), 4)
            .draw_styled(&PrimitiveStyle::with_fill(Rgb888::BLUE), &mut dm)
            .unwrap();
        Circle::with_center(Point::new(i as i32 - 4, j as i32 - 4), i.max(0) as u32 / 10)
            .draw_styled(&PrimitiveStyle::with_fill(Rgb888::GREEN), &mut dm)
            .unwrap();
        Circle::with_center(Point::new(i as i32 + 6, j as i32 - 8), 4)
            .draw_styled(&PrimitiveStyle::with_fill(Rgb888::YELLOW), &mut dm)
            .unwrap();

        dm.swap_buffers(&fb_loop_ch);
        Timer::after_millis(1).await;
        let new = embassy_time::Instant::now();
        t += ((new - instant).as_millis() as f32) / 1000.0;
        instant = new;
    }
}
