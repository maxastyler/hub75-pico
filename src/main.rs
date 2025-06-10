#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::{Executor, Spawner};
use embassy_rp::bind_interrupts;
use embassy_rp::dma::Channel;
use embassy_rp::gpio::{Level, Output, Pin};
use embassy_rp::multicore::{Stack, spawn_core1};
use embassy_rp::pac::DMA;
use embassy_rp::pac::dma::Dma;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::{
    Config, Direction, FifoJoin, InterruptHandler as PioInterruptHandler, Pio, ShiftConfig,
    ShiftDirection,
};
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

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let mut i = 0;

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
        cfg.clock_divider = fixed::FixedU32::from_num(2);
        cfg.shift_out.direction = ShiftDirection::Right;
        cfg.shift_out.auto_fill = true;
        cfg.fifo_join = FifoJoin::TxOnly;
        cfg
    };

    sm0.set_config(&cfg);
    sm0.set_pin_dirs(Direction::Out, &[&r1, &g1, &b1, &r2, &g2, &b2, &clk]);
    sm0.tx().push(63);

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
    sm1.tx().push(32 / 2 - 1);
    sm1.tx().push(8 - 1);

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
    sm1.set_pin_dirs(Direction::Out, &[&oe]);
}
