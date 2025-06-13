// translated from https://github.com/kjagiello/hub75-pio-rs/blob/262bca716990f0c7eb54b6d6f40578498a78a505/src/lib.rs

#![no_std]
#![no_main]
#![feature(generic_const_exprs)]

use defmt::*;
use embassy_executor::{Executor, Spawner};
use embassy_rp::dma::Channel;
use embassy_rp::gpio::{Level, Output, Pin};
use embassy_rp::multicore::{Stack, spawn_core1};
use embassy_rp::pac;
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
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_time::Timer;
use fixed::FixedU32;
use fixed::types::extra::U8;
use pio::{ProgramWithDefines, pio_asm};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
});

/// Framebuffer size in bytes
#[doc(hidden)]
pub const fn fb_bytes(w: usize, h: usize, b: usize) -> usize {
    w * h / 2 * b
}

pub struct DisplayMemory<const W: usize, const H: usize, const B: usize>
where
    [(); fb_bytes(W, H, B)]: Sized,
{
    fbptr: [u32; 1],
    fb0: [u8; fb_bytes(W, H, B)],
    fb1: [u8; fb_bytes(W, H, B)],
    delays: [u32; B],
    delaysptr: [u32; 1],
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

impl<const W: usize, const H: usize, const B: usize> DisplayMemory<W, H, B>
where
    [(); fb_bytes(W, H, B)]: Sized,
{
    pub const fn new() -> Self {
        let fb0 = [0xff; fb_bytes(W, H, B)];
        let fb1 = [0xff; fb_bytes(W, H, B)];
        let fbptr: [u32; 1] = [0];
        let delays = delays();
        let delaysptr: [u32; 1] = [0];
        DisplayMemory {
            fbptr,
            fb0,
            fb1,
            delays,
            delaysptr,
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let mut dm: DisplayMemory<64, 32, 4> = DisplayMemory::new();

    let pio = p.PIO0;
    let Pio {
        mut common,
        mut sm0,
        mut sm1,
        mut sm2,
        ..
    } = Pio::new(pio, Irqs);

    embassy_rp::pac::pio::Pio::instr_mem()
    
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
        cfg.clock_divider = fixed::FixedU32::from_num(2);
        cfg.shift_out.direction = ShiftDirection::Right;
        cfg.shift_out.auto_fill = true;
        cfg.fifo_join = FifoJoin::TxOnly;
        cfg
    };

    sm0.set_config(&cfg);
    sm0.set_pin_dirs(Direction::Out, &[&r1, &g1, &b1, &r2, &g2, &b2, &clk]);

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

    let cfg = {
        let mut cfg = Config::default();
        cfg.use_program(&common.load_program(&row_prog.program), &[&lat]);
        cfg.set_out_pins(&[&a, &b, &c, &d]);
        cfg.clock_divider = FixedU32::<U8>::from_num(1) + FixedU32::<U8>::from_bits(0b1);
        cfg
    };

    sm1.set_config(&cfg);
    sm1.set_pin_dirs(Direction::Out, &[&a, &b, &c, &d, &lat]);


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
        cfg.clock_divider = FixedU32::<U8>::from_num(1) + FixedU32::<U8>::from_bits(0b1);
        cfg.shift_out.auto_fill = true;
        cfg.fifo_join = FifoJoin::TxOnly;
        cfg
    };

    sm2.set_config(&cfg);
    sm2.set_pin_dirs(Direction::Out, &[&oe]);

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
        .write(|c| c.0 = fb_bytes(64, 32, 8) as u32 / 4);
    fb_ch
        .regs()
        .write_addr()
        .write(|c| pac::PIO0.as_ptr() as u32 + 0x10);

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
        .write(|c| *c = pac::PIO0.as_ptr() as u32 + 0x10 + (2 * 4));

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

    sm0.restart();
    sm1.restart();
    sm2.restart();

    sm0.tx().push(64 - 1);
    sm1.tx().push(32 / 2 - 1);
    sm1.tx().push(8 - 1);
    info!("{}", sm0.get_addr());

    loop {
        if dm.fbptr[0] == dm.fb0.as_ptr() as u32 {
            dm.fbptr[0] = dm.fb1.as_ptr() as u32;
        } else {
            dm.fbptr[0] = dm.fb0.as_ptr() as u32;
        }
        // while !fb_loop_ch.regs().ctrl_trig().read().busy() {}

        info!("{}", sm0.get_addr());
        info!("{}", sm1.get_addr());
        info!("{}", sm2.get_addr());
        Timer::after_millis(100).await;
    }
}
