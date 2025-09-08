use embedded_graphics::{
    Pixel,
    pixelcolor::Rgb888,
    prelude::{Point, RgbColor},
};

use crate::{RngU32, StateUpdate, Visualisation};

pub struct GameOfLife<const W: usize, const H: usize>
where
    [(); W * H]: Sized,
{
    pub board_1: [bool; W * H],
    pub board_2: [bool; W * H],
    pub board_1_current: bool,
}

impl<const W: usize, const H: usize> GameOfLife<W, H>
where
    [(); W * H]: Sized,
{
    pub fn new() -> Self {
        GameOfLife {
            board_1: [false; W * H],
            board_2: [false; W * H],
            board_1_current: true,
        }
    }

    pub fn new_with_random<Rng: RngU32>(n: usize, mut rng: Rng) -> Self {
        let mut this = Self::new();

        for _ in 0..n {
            this.board_1[(rng.next_u32() % (W * H) as u32) as usize] = true;
        }

        this
    }

    fn get_read_and_write(&mut self) -> (&mut [bool; W * H], &[bool; W * H]) {
        if self.board_1_current {
            (&mut self.board_2, &self.board_1)
        } else {
            (&mut self.board_1, &self.board_2)
        }
    }

    fn step(&mut self) {
        let (write, read) = self.get_read_and_write();
        for x in 0..W {
            for y in 0..H {
                let x = x as i32;
                let y = y as i32;
                let mut total: u8 = 0;
                for (dx, dy) in [
                    (1, 0),
                    (1, 1),
                    (0, 1),
                    (-1, 1),
                    (-1, 0),
                    (-1, -1),
                    (0, -1),
                    (1, -1),
                ] {
                    let px = (x + dx) as i32;
                    let py = (y + dy) as i32;
                    if (px >= 0) & (px < W as i32) & (py >= 0) & (py < H as i32) {
                        let index = px + W as i32 * py;
                        if read[index as usize] {
                            total += 1;
                        }
                    }
                }
                let index = (x + W as i32 * y) as usize;
                if (total < 2) | (total > 3) {
                    write[index] = false;
                } else if total == 3 {
                    write[index] = true
                } else {
                    write[index] = read[index]
                }
            }
        }
        self.board_1_current = !self.board_1_current;
    }
}

pub struct GameOfLifeUpdate {}

impl StateUpdate for GameOfLifeUpdate {}

impl<const W: usize, const H: usize> Visualisation for GameOfLife<W, H>
where
    [(); W * H]: Sized,
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
            .draw_iter(
                (0..H)
                    .flat_map(|y| (0..W).map(move |x| (x, y)))
                    .map(|(x, y)| {
                        if board[y * W + x] {
                            Pixel(Point::new(x as i32, y as i32), Rgb888::WHITE)
                        } else {
                            Pixel(Point::new(x as i32, y as i32), Rgb888::BLACK)
                        }
                    }),
            )
            .unwrap();
    }
}
