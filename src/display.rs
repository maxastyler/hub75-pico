use core::hint::spin_loop;
use core::marker::PhantomData;

use embassy_rp::dma::Channel;
use embassy_rp::pac::common::{RW, Reg};
use embassy_rp::pac::dma::regs::CtrlTrig;
use embassy_rp::pac::dma::vals::{DataSize, TreqSel};
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::{
    Common, Config as PioConfig, Direction, FifoJoin, Instance as PioInstance, Pin, Pio, PioPin,
    ShiftDirection, StateMachine,
};
use embassy_rp::{Peripheral, PeripheralRef};
use fixed::FixedU32;
use fixed::types::extra::U8;
use static_cell::StaticCell;

use crate::lut::Lut;

/// The delays to use for 8 bit numbers
const DELAYS_8_BIT: [u32; 8] = delays();
const PTR_TO_DELAYS: &'static [u32; 8] = &DELAYS_8_BIT;
static PTR_TO_FRAMEBUFFER: StaticCell<*const [u8]> = StaticCell::new();

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

struct DisplayPeripherals<'a, PIO: PioInstance, FB_CH, FB_L_CH, OE_CH, OE_L_CH> {
    common: Common<'a, PIO>,
    rgb_sm: StateMachine<'a, PIO, 0>,
    row_sm: StateMachine<'a, PIO, 1>,
    oe_sm: StateMachine<'a, PIO, 2>,
    fb_channel: PeripheralRef<'a, FB_CH>,
    fb_loop_channel: PeripheralRef<'a, FB_L_CH>,
    oe_channel: PeripheralRef<'a, OE_CH>,
    oe_loop_channel: PeripheralRef<'a, OE_L_CH>,
}

fn setup_rgb_state_machine<'a, PIO: PioInstance, const N: usize, const W: usize>(
    common: &mut Common<'a, PIO>,
    sm: &mut StateMachine<'a, PIO, N>,
    clk: &Pin<'a, PIO>,
    r1: &Pin<'a, PIO>,
    g1: &Pin<'a, PIO>,
    b1: &Pin<'a, PIO>,
    r2: &Pin<'a, PIO>,
    g2: &Pin<'a, PIO>,
    b2: &Pin<'a, PIO>,
) {
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
        let mut cfg = PioConfig::default();
        cfg.use_program(&common.load_program(&rgb_prog.program), &[&clk]);
        cfg.set_out_pins(&[&r1, &g1, &b1, &r2, &g2, &b2]);
        cfg.clock_divider = fixed::FixedU32::from_num(4);
        cfg.shift_out.direction = ShiftDirection::Right;
        cfg.shift_out.auto_fill = true;
        cfg.fifo_join = FifoJoin::TxOnly;
        cfg
    };

    sm.set_config(&cfg);
    sm.set_pin_dirs(Direction::Out, &[&r1, &g1, &b1, &r2, &g2, &b2, &clk]);
    sm.set_enable(true);
    sm.tx().push(W as u32 - 1);
}

fn setup_row_state_machine<'a, PIO: PioInstance, const N: usize, const H: usize>(
    common: &mut Common<'a, PIO>,
    sm: &mut StateMachine<'a, PIO, N>,
    latch: &Pin<'a, PIO>,
    a: &Pin<'a, PIO>,
    b: &Pin<'a, PIO>,
    c: &Pin<'a, PIO>,
    d: &Pin<'a, PIO>,
    clock_divider: f32,
) {
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
        let mut cfg = PioConfig::default();
        cfg.use_program(&common.load_program(&row_prog.program), &[&latch]);
        cfg.set_out_pins(&[&a, &b, &c, &d]);
        cfg.clock_divider = FixedU32::<U8>::from_num(clock_divider);
        cfg
    };

    sm.set_config(&cfg);
    sm.set_pin_dirs(Direction::Out, &[&a, &b, &c, &d, &latch]);
    sm.set_enable(true);
    sm.tx().push(H as u32 / 2 - 1);
    // push the bit depth
    sm.tx().push(8 - 1);
}

fn setup_delay_state_machine<'a, PIO: PioInstance, const N: usize>(
    common: &mut Common<'a, PIO>,
    sm: &mut StateMachine<'a, PIO, N>,
    oe: &Pin<'a, PIO>,
    clock_divider: f32,
) {
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
        let mut cfg = PioConfig::default();
        cfg.use_program(&common.load_program(&delay_prog.program), &[&oe]);
        cfg.clock_divider = FixedU32::<U8>::from_num(clock_divider);
        cfg.shift_out.auto_fill = true;
        cfg.fifo_join = FifoJoin::TxOnly;
        cfg
    };

    sm.set_config(&cfg);
    sm.set_pin_dirs(Direction::Out, &[&oe]);
    sm.set_enable(true);
}

fn setup_framebuffer_channel<const W: usize, const H: usize, FB_CH: Channel, FB_L_CH: Channel>(
    fb_channel: PeripheralRef<'_, FB_CH>,
    fb_loop_channel: PeripheralRef<'_, FB_L_CH>,
    pio_dreq_sel: TreqSel,
    framebuffer: *const [u8; fb_bytes(W, H, 8)],
    rgb_state_machine_tx_register: &Reg<u32, RW>,
) {
    fb_channel.regs().al1_ctrl().write(|c| {
        let mut t = CtrlTrig(*c);
        t.set_incr_read(true);
        t.set_incr_write(false);
        t.set_data_size(DataSize::SIZE_WORD);
        t.set_treq_sel(pio_dreq_sel);
        t.set_irq_quiet(true);
        t.set_chain_to(fb_loop_channel.number());
        t.set_en(true);
        *c = t.0;
    });

    fb_channel
        .regs()
        .read_addr()
        .write(|c| *c = framebuffer as u32);
    fb_channel
        .regs()
        .trans_count()
        .write(|c| c.0 = fb_bytes(W, H, 8) as u32 / 4);
    fb_channel
        .regs()
        .write_addr()
        .write(|c| *c = rgb_state_machine_tx_register.as_ptr() as u32);
}

/// `framebuffer_pointer_location`: the pointer to the pointer to the framebuffer
fn setup_framebuffer_loop_channel<
    const W: usize,
    const H: usize,
    FB_L_CH: Channel,
    FB_CH: Channel,
>(
    fb_loop_channel: PeripheralRef<'_, FB_L_CH>,
    fb_channel: PeripheralRef<'_, FB_CH>,
    framebuffer_pointer_location: &mut *const [u8],
) {
    fb_loop_channel.regs().al1_ctrl().write(|c| {
        let mut t = CtrlTrig(*c);
        t.set_incr_read(false);
        t.set_incr_write(false);
        t.set_data_size(DataSize::SIZE_WORD);
        t.set_treq_sel(TreqSel::PERMANENT);
        t.set_irq_quiet(true);
        t.set_chain_to(fb_channel.number());
        t.set_en(true);
        *c = t.0;
    });

    fb_loop_channel
        .regs()
        .read_addr()
        .write(|c| *c = (framebuffer_pointer_location as *mut *const [u8]) as u32);
    fb_loop_channel.regs().trans_count().write(|c| c.0 = 1);
    fb_loop_channel
        .regs()
        .al2_write_addr_trig()
        .write(|c| *c = fb_channel.regs().read_addr().as_ptr() as u32);
}

/// `pio_dreq_sel` is the dreq trigger for the oe state machine
fn setup_oe_channel<OE_CH: Channel, OE_L_CH: Channel>(
    oe_channel: PeripheralRef<'_, OE_CH>,
    oe_loop_channel: PeripheralRef<'_, OE_L_CH>,
    pio_dreq_sel: TreqSel,
    oe_state_machine_tx_register: &Reg<u32, RW>,
) {
    oe_channel.regs().al1_ctrl().write(|c| {
        let mut t = CtrlTrig(*c);
        t.set_incr_read(true);
        t.set_incr_write(false);
        t.set_data_size(DataSize::SIZE_WORD);
        t.set_treq_sel(pio_dreq_sel);
        t.set_irq_quiet(true);
        t.set_chain_to(oe_loop_channel.number());
        t.set_en(true);
        *c = t.0;
    });
    oe_channel
        .regs()
        .read_addr()
        .write(|c| *c = DELAYS_8_BIT.as_ptr() as u32);
    oe_channel
        .regs()
        .trans_count()
        .write(|c| c.0 = DELAYS_8_BIT.len() as u32);
    oe_channel
        .regs()
        .write_addr()
        .write(|c| *c = oe_state_machine_tx_register.as_ptr() as u32);
}

/// `pio_dreq_sel` is the dreq trigger for the oe state machine
fn setup_oe_loop_channel<const W: usize, const H: usize, OE_CH: Channel, OE_L_CH: Channel>(
    oe_loop_channel: PeripheralRef<'_, OE_L_CH>,
    oe_channel: PeripheralRef<'_, OE_CH>,
) {
    oe_loop_channel.regs().al1_ctrl().write(|c| {
        let mut t = CtrlTrig(*c);

        t.set_incr_read(false);
        t.set_incr_write(false);
        t.set_data_size(DataSize::SIZE_WORD);
        t.set_treq_sel(TreqSel::PERMANENT);
        t.set_irq_quiet(true);
        t.set_chain_to(oe_channel.number());
        t.set_en(true);

        *c = t.0;
    });

    oe_loop_channel
        .regs()
        .read_addr()
        .write(|c| *c = (&PTR_TO_DELAYS as *const &[u32; 8]) as u32);
    oe_loop_channel.regs().trans_count().write(|c| c.0 = 1);
    oe_loop_channel
        .regs()
        .al2_write_addr_trig()
        .write(|c| *c = oe_channel.regs().read_addr().as_ptr() as u32);
}

pub struct Display<'a, const W: usize, const H: usize, FB_CH, FB_L_CH, OE_CH, OE_L_CH> {
    pub brightness: u8,
    pub lut: &'a dyn Lut,
    peripherals: DisplayPeripherals<'a, PIO0, FB_CH, FB_L_CH, OE_CH, OE_L_CH>,
    ptr_to_framebuffer: &'static mut *const [u8],
}

impl<'a, const W: usize, const H: usize, FB_CH, FB_L_CH, OE_CH, OE_L_CH>
    Display<'a, W, H, FB_CH, FB_L_CH, OE_CH, OE_L_CH>
where
    FB_CH: Channel,
    FB_L_CH: Channel,
    OE_CH: Channel,
    OE_L_CH: Channel,
{
    pub fn new(
        lut: &'a impl Lut,
        pio: Pio<'a, PIO0>,
        frame_buffer: *const [u8; fb_bytes(W, H, 8)],
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
    ) -> Self {
        let ptr_to_framebuffer: &'static mut *const [u8] = PTR_TO_FRAMEBUFFER.init(frame_buffer);

        let Pio {
            mut common,
            sm0: mut rgb_sm,
            sm1: mut row_sm,
            sm2: mut oe_sm,
            ..
        } = pio;
        let (clk, r1, g1, b1, r2, g2, b2) = (
            common.make_pio_pin(clk),
            common.make_pio_pin(r1),
            common.make_pio_pin(g1),
            common.make_pio_pin(b1),
            common.make_pio_pin(r2),
            common.make_pio_pin(g2),
            common.make_pio_pin(b2),
        );
        setup_rgb_state_machine::<_, 0, W>(
            &mut common,
            &mut rgb_sm,
            &clk,
            &r1,
            &g1,
            &b1,
            &r2,
            &g2,
            &b2,
        );

        let (latch, a, b, c, d) = (
            common.make_pio_pin(lat),
            common.make_pio_pin(a),
            common.make_pio_pin(b),
            common.make_pio_pin(c),
            common.make_pio_pin(d),
        );

        setup_row_state_machine::<_, 1, H>(&mut common, &mut row_sm, &latch, &a, &b, &c, &d, 1.2);

        let oe = common.make_pio_pin(oe);

        setup_delay_state_machine(&mut common, &mut oe_sm, &oe, 1.2);

        let mut fb_channel = fb_channel.into_ref();
        let mut fb_loop_channel = fb_loop_channel.into_ref();
        let mut oe_channel = oe_channel.into_ref();
        let mut oe_loop_channel = oe_loop_channel.into_ref();

        setup_framebuffer_channel(
            fb_channel.reborrow(),
            fb_loop_channel.reborrow(),
            TreqSel::PIO0_TX0,
            frame_buffer,
            &embassy_rp::pac::PIO0.txf(0),
        );

        setup_framebuffer_loop_channel::<W, H, _, _>(
            fb_loop_channel.reborrow(),
            fb_channel.reborrow(),
            ptr_to_framebuffer,
        );

        setup_oe_channel(
            oe_channel.reborrow(),
            oe_loop_channel.reborrow(),
            TreqSel::PIO0_TX2,
            &embassy_rp::pac::PIO0.txf(2),
        );

        setup_oe_loop_channel::<W, H, _, _>(oe_loop_channel.reborrow(), oe_channel.reborrow());

        Display {
            lut,
            peripherals: DisplayPeripherals {
                common,
                rgb_sm,
                row_sm,
                oe_sm,
                fb_channel,
                fb_loop_channel,
                oe_channel,
                oe_loop_channel,
            },
            brightness: 255,
            ptr_to_framebuffer,
        }
    }

    pub fn set_new_framebuffer(&mut self, frame_buffer: *const [u8; fb_bytes(W, H, 8)]) {
        *self.ptr_to_framebuffer = frame_buffer;
        // while !self
        //     .peripherals
        //     .fb_loop_channel
        //     .regs()
        //     .ctrl_trig()
        //     .read()
        //     .busy()

        // {}
    }
}
