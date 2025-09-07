use super::{StateUpdate, Visualisation};

mod queue {
    use core::mem::MaybeUninit;

    pub struct Queue<T, const N: usize> {
        data: [MaybeUninit<T>; N],
        ptr: usize,
    }

    impl<T: Copy + PartialEq, const N: usize> Queue<T, N> {
        pub fn new() -> Self {
            Self {
                data: [MaybeUninit::uninit(); N],
                ptr: 0,
            }
        }

	#[allow(dead_code)]
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

        pub fn iter(&self) -> impl Iterator<Item = T> {
            (0..self.ptr).map(|i| unsafe { self.data[i].assume_init() })
        }

        pub fn push_unique(&mut self, val: T) -> Option<T> {
            if self.iter().all(|x| x != val) {
                self.push(val)
            } else {
                None
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
    row: u8,
    col: u8,
    n_updates_per_iteration: usize,
}

pub trait RngU32 {
    fn next_u32(&mut self) -> u32;
}

impl<const R: usize, const C: usize> SandPile<R, C>
where
    [(); R * C]:,
{
    pub fn new<Rng: RngU32>(mut rng: Rng) -> Self
    where
        [(); R * C]:,
    {
        // let mut trng = embassy_rp::trng::Trng::new(
        //     unsafe { TRNG::steal() },
        //     Irqs,
        //     embassy_rp::trng::Config::default(),
        // );

        let rn = rng.next_u32() as usize;
        let row: u8 = (rn % R) as u8;
        let col: u8 = ((rn >> 16) % C) as u8;

        SandPile {
            sand: [0; R * C],
            queue: Queue::new(),
            row,
            col,
            n_updates_per_iteration: 50,
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
        loop {
            let pos = self.get_mut(row, col, 0, 0).unwrap().0;
            if *pos < 4 {
                return;
            } else {
                *pos = *pos - 4;
                for (i, j) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    if let Some((other_pos, index)) = self.get_mut(row, col, i, j) {
                        *other_pos = *other_pos + 1;
                        if *other_pos >= 4 {
                            self.queue.push_unique(index);
                        }
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

    fn update(&mut self, _delta_time_us: u32) -> bool {
        for _ in 0..self.n_updates_per_iteration {
            // add to the selected row
            let pos = self.get_mut(self.row, self.col, 0, 0).unwrap().0;
            *pos = pos.saturating_add(1);
            self.queue.push_unique((self.row, self.col));

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
        target
            .draw_iter(
                (0..R)
                    .flat_map(|r| (0..C).map(move |c| (r, c)))
                    .zip(self.sand)
                    .map(|((r, c), p)| {
                        let colour = match p {
                            0 => Rgb888::BLACK,
                            1 => Rgb888::WHITE,
                            2 => Rgb888::RED,
                            3 => Rgb888::BLUE,
                            _ => Rgb888::CSS_HOT_PINK,
                        };
                        Pixel(Point::new(c as i32, r as i32), colour)
                    }),
            )
            .unwrap();
    }
}
