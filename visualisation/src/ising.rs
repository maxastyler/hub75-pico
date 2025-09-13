use embedded_graphics::{
    Pixel,
    pixelcolor::Rgb888,
    prelude::{Point, RgbColor},
};

use crate::{RngU32, StateUpdate, Visualisation, grid::Grid};

pub struct Ising<Rng, const W: usize, const H: usize>
where
    [(); W * H]:,
{
    grid: Grid<i8, W, H>,
    rng: Rng,
    beta: f32,
}

impl<Rng: RngU32, const W: usize, const H: usize> Ising<Rng, W, H>
where
    [(); W * H]:,
{
    pub fn new(beta: f32, mut rng: Rng) -> Self {
        let mut grid = Grid::new(0);
        grid.buffer_mut()
            .iter_mut()
            .for_each(|n| *n = if rng.next_u32() % 2 == 0 { 1 } else { -1 });
        Ising { grid, rng, beta }
    }

    fn step(&mut self) {
        let (x, y) = self.grid.random_coord(&mut self.rng);
        let nb: i8 = [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
            .into_iter()
            .map(|(ox, oy)| self.grid.get(ox, oy).copied().unwrap_or(0))
            .sum();
        let s = self.grid.get_mut(x, y).unwrap();
        let cost = 2 * *s * nb;

        if (cost < 0) | (self.rng.unit_f32() < libm::expf(-cost as f32 * self.beta)) {
            *s *= -1;
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum IsingUpdate {
    Reset,
}

impl StateUpdate for IsingUpdate {}

impl<Rng: RngU32, const W: usize, const H: usize> Visualisation<Rng> for Ising<Rng, W, H>
where
    [(); W * H]:,
{
    type StateUpdate = IsingUpdate;

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
        let _ = target.draw_iter(self.grid.iter_with_index().map(|((x, y), i)| {
            Pixel(
                Point::new(x, y),
                if *i == 1 {
                    Rgb888::WHITE
                } else {
                    Rgb888::BLACK
                },
            )
        }));
    }

    fn run_state_update(&mut self, state_update: Self::StateUpdate) {
        todo!()
    }

    fn reset(&mut self) {
        self.grid
            .buffer_mut()
            .iter_mut()
            .for_each(|n| *n = if self.rng.next_u32() % 2 == 0 { 1 } else { -1 });
    }

    fn new(rng: Rng) -> Self {
        Ising::new(1.0, rng)
    }
}
