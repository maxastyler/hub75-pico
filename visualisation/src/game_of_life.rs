use embedded_graphics::{
    Pixel,
    pixelcolor::Rgb888,
    prelude::{Point, RgbColor},
};

use crate::{RngU32, StateUpdate, Visualisation, grid::Grid};

pub struct GameOfLife<Rng, const W: usize, const H: usize>
where
    [(); W * H]:,
{
    pub board_1: Grid<bool, W, H>,
    pub board_2: Grid<bool, W, H>,
    pub board_1_current: bool,
    rng: Rng,
}

impl<Rng: RngU32, const W: usize, const H: usize> GameOfLife<Rng, W, H>
where
    [(); W * H]:,
{
    pub fn new_with_random(n: usize, rng: Rng) -> Self {
        let mut this = GameOfLife {
            board_1: Grid::new(false),
            board_2: Grid::new(false),
            board_1_current: true,
            rng,
        };

        for _ in 0..n {
            this.board_1.buffer_mut()[(this.rng.next_u32() % (W * H) as u32) as usize] = true;
        }

        this
    }

    fn get_read_and_write(&mut self) -> (&mut Grid<bool, W, H>, &Grid<bool, W, H>) {
        if self.board_1_current {
            (&mut self.board_2, &self.board_1)
        } else {
            (&mut self.board_1, &self.board_2)
        }
    }

    fn step(&mut self) {
        let (write, read) = self.get_read_and_write();
        read.iter_with_index().for_each(|((x, y), val)| {
            let total = [
                (1, 0),
                (1, 1),
                (0, 1),
                (-1, 1),
                (-1, 0),
                (-1, -1),
                (0, -1),
                (1, -1),
            ]
            .into_iter()
            .filter_map(|(dx, dy)| match read.get(x + dx, y + dy) {
                Some(true) => Some(()),
                _ => None,
            })
            .count();

            if (total < 2) | (total > 3) {
                write.set(x, y, false);
            } else if total == 3 {
                write.set(x, y, true);
            } else {
                write.set(x, y, *val);
            }
        });

        self.board_1_current = !self.board_1_current;
    }
}

pub struct GameOfLifeUpdate {}

impl StateUpdate for GameOfLifeUpdate {}

impl<Rng: RngU32, const W: usize, const H: usize> Visualisation for GameOfLife<Rng, W, H>
where
    [(); W * H]:,
{
    type StateUpdate = GameOfLifeUpdate;

    fn update(&mut self, _delta_time_us: u32) -> bool {
        self.step();
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
        let board = if self.board_1_current {
            &self.board_1
        } else {
            &self.board_2
        };
        target
            .draw_iter(board.iter_with_index().map(|((x, y), val)| {
                if *val {
                    Pixel(Point::new(x as i32, y as i32), Rgb888::WHITE)
                } else {
                    Pixel(Point::new(x as i32, y as i32), Rgb888::BLACK)
                }
            }))
            .unwrap();
    }
}
