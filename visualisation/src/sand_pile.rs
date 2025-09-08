use crate::{RngU32, grid::Grid};

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

pub struct SandPile<Rng, const W: usize, const H: usize>
where
    [(); W * H]:,
{
    sand: Grid<u8, W, H>,
    /// a list of coordinates to collapse
    collapse_queue: Queue<(u8, u8), 2048>,
    rng: Rng,
    x_drop: i32,
    y_drop: i32,
    /// if true, drop sand in random positions, otherwise use the x and y
    drop_randomly: bool,
}

impl<Rng: RngU32, const W: usize, const H: usize> SandPile<Rng, W, H>
where
    [(); W * H]:,
{
    pub fn new(mut rng: Rng) -> Self
    where
        [(); W * H]:,
    {
        let rn = rng.next_u32() as usize;
        let x = (rn % W) as i32;
        let y = ((rn >> 16) % H) as i32;
        SandPile {
            sand: Grid::new(0),
            collapse_queue: Queue::new(),
            rng,
            x_drop: x,
            y_drop: y,
            drop_randomly: true,
        }
    }

    /// pull from the queue until something happens in the grid.
    /// Returns false if nothing happened. Returns true if something happened
    fn pull_until_changed(&mut self) -> bool {
        while let Some((x, y)) = self.collapse_queue.pull() {
            if let Some(v) = self.sand.get_mut(x as i32, y as i32)
                && *v >= 4
            {
                *v -= 4;
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let (ox, oy) = (x as i32 + dx, y as i32 + dy);
                    if let Some(other) = self.sand.get_mut(ox, oy) {
                        *other += 1;
                        if *other >= 4 {
                            self.collapse_queue.push_unique((ox as u8, oy as u8));
                        }
                    }
                }
                return true;
            }
        }
        false
    }

    fn place_sand(&mut self) -> bool {
        let (x, y) = if self.drop_randomly {
            let rn = self.rng.next_u32() as usize;
            let x = (rn % W) as i32;
            let y = ((rn >> 16) % H) as i32;
            (x, y)
        } else {
            (self.x_drop, self.y_drop)
        };

        if let Some(sand) = self.sand.get_mut(x, y) {
            *sand += 1;
            if *sand >= 4 {
                self.collapse_queue.push_unique((x as u8, y as u8));
            }
            true
        } else {
            false
        }
    }

    /// Update the simulation until something changes
    fn step_until_changed(&mut self) {
        if !self.pull_until_changed() {
            while !self.place_sand() {}
        }
    }
}

pub enum SandpileStateUpdate {
    Reset,
}

impl StateUpdate for SandpileStateUpdate {}

impl<Rng: RngU32, const W: usize, const H: usize> Visualisation for SandPile<Rng, W, H>
where
    [(); W * H]:,
{
    type StateUpdate = SandpileStateUpdate;

    fn update(&mut self, _delta_time_us: u32) -> bool {
        // for _ in 0..self.n_updates_per_iteration {
        //     // add to the selected row
        //     let pos = self.get_mut(self.row, self.col, 0, 0).unwrap().0;
        //     *pos = pos.saturating_add(1);
        //     self.collapse_queue.push_unique((self.row, self.col));

        //     while let Some((row, col)) = self.collapse_queue.pull() {
        //         self.update_position(row, col);
        //     }
        // }
        for _ in 0..100 {
            self.step_until_changed();
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
            .draw_iter(self.sand.iter_with_index().map(|((x, y), v)| {
                let colour = match v {
                    0 => Rgb888::BLACK,
                    1 => Rgb888::WHITE,
                    2 => Rgb888::RED,
                    3 => Rgb888::BLUE,
                    _ => Rgb888::CSS_HOT_PINK,
                };
                Pixel(Point::new(x, y), colour)
            }))
            .unwrap();
    }
}
