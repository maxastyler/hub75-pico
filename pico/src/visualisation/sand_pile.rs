use super::{StateUpdate, Visualisation};
use crate::Irqs;
use core::mem::MaybeUninit;
use defmt::*;
use embassy_rp::peripherals::TRNG;

mod queue {
    use core::mem::MaybeUninit;

    pub struct Queue<T, const N: usize> {
        data: [MaybeUninit<T>; N],
        ptr: usize,
    }

    impl<T: Copy, const N: usize> Queue<T, N> {
        pub fn new() -> Self {
            Self {
                data: [MaybeUninit::uninit(); N],
                ptr: 0,
            }
        }

	pub fn len(&self) -> usize {
	    self.ptr
	}

        pub fn push(&mut self, val: T) -> Option<T> {
            if self.ptr < N {
                self.data[self.ptr] = MaybeUninit::new(val);
                self.ptr += 1;
                None
            } else {
                Some(val)
            }
        }

        pub fn pull(&mut self) -> Option<T> {
            self.ptr = self.ptr.checked_sub(1)?;
            Some(unsafe { self.data[self.ptr].assume_init() })
        }
    }
}
use embedded_graphics::{
    Pixel,
    pixelcolor::Rgb888,
    prelude::{Point, RgbColor, WebColors},
};
use queue::Queue;

pub struct SandPile<const R: usize, const C: usize>
where
    [(); R * C]:,
{
    sand: [u8; R * C],
    queue: Queue<(u8, u8), 2048>,
    trng: embassy_rp::trng::Trng<'static, TRNG>,
}

impl<const R: usize, const C: usize> SandPile<R, C>
where
    [(); R * C]:,
{
    pub fn new() -> Self
    where
        [(); R * C]:,
    {
        let trng = embassy_rp::trng::Trng::new(
            unsafe { TRNG::steal() },
            Irqs,
            embassy_rp::trng::Config::default(),
        );
        SandPile {
            sand: [0; R * C],
            queue: Queue::new(),
            trng,
        }
    }

    fn get_mut(
        &mut self,
        row: u8,
        col: u8,
        offset_row: i8,
        offset_col: i8,
    ) -> Option<(&mut u8, (u8, u8))> {
        let r = row.checked_add_signed(offset_row)? as usize;
        if r >= R {
            return None;
        }
        let c = col.checked_add_signed(offset_col)? as usize;
        if c >= C {
            return None;
        }
        Some((&mut self.sand[r * C + c], (r as u8, c as u8)))
    }

    fn update_position(&mut self, row: u8, col: u8) {
        let pos = self.get_mut(row, col, 0, 0).unwrap().0;
        *pos = pos.saturating_add(1);
        while *self.get_mut(row, col, 0, 0).unwrap().0 >= 4 {
            *self.get_mut(row, col, 0, 0).unwrap().0 -= 4;
            for (i, j) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                if let Some((pos, index)) = self.get_mut(row, col, i, j) {
                    *pos = pos.saturating_add(1);
                    if *pos >= 4 {
                        self.queue.push(index);
                    }
                }
            }
        }
    }
}

pub enum SandpileStateUpdate {
    Reset,
}

impl StateUpdate for SandpileStateUpdate {}

impl<const R: usize, const C: usize> Visualisation for SandPile<R, C>
where
    [(); R * C]:,
{
    type StateUpdate = SandpileStateUpdate;

    fn update(&mut self, delta_time: embassy_time::Duration) -> bool {
        for _ in 0..10 {
            let rn = self.trng.blocking_next_u32() as usize;
            let row: u8 = (rn % R) as u8;
            let col: u8 = ((rn >> 16) % C) as u8;
            self.update_position(row, col);

	    info!("Queue len: {}", self.queue.len());

            while let Some((row, col)) = self.queue.pull() {
                self.update_position(row, col);
            }
        }
        true
    }

    fn draw<
        D: embedded_graphics::prelude::DrawTarget<
                Color = embedded_graphics::pixelcolor::Rgb888,
                Error = core::convert::Infallible,
            >,
    >(
        &mut self,
        target: &mut D,
    ) {
        unwrap!(
            target.draw_iter(
                (0..R)
                    .flat_map(|r| { (0..C).map(move |c| (r, c)) })
                    .zip(self.sand)
                    .map(|((r, c), p)| {
                        let colour = match p {
                            0 => Rgb888::BLACK,
                            1 => Rgb888::GREEN,
                            2 => Rgb888::RED,
                            3 => Rgb888::YELLOW,
                            _ => Rgb888::CSS_HOT_PINK,
                        };
                        Pixel(Point::new(c as i32, r as i32), colour)
                    })
            )
        );
    }
}
