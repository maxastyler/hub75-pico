use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, StyledDrawable},
};

use super::{StateUpdate, Visualisation};

#[derive(serde::Serialize, serde::Deserialize)]
pub enum TestVisUpdate {}

impl StateUpdate for TestVisUpdate {}

pub struct TestVis {
    time: f32,
}

impl TestVis {
    pub fn new() -> Self {
        TestVis { time: 0.0 }
    }
}

impl Visualisation for TestVis {
    type StateUpdate = TestVisUpdate;

    fn update(&mut self, delta_time_us: u32) -> bool {
        self.time += (delta_time_us as f32) / 1000_000.0;
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
        let i: i32 = (64 / 2) as i32 + (15.0 * libm::sinf(3.0 * self.time)) as i32;
        let j: i32 = (32 / 2) as i32 + (15.0 * libm::cosf(2.1 * self.time)) as i32;

        Circle::with_center(Point::new(0, 0), 30)
            .draw_styled(&PrimitiveStyle::with_fill(Rgb888::CSS_BROWN), target)
            .unwrap();

        Circle::with_center(Point::new(0, 32), 30)
            .draw_styled(&PrimitiveStyle::with_fill(Rgb888::CSS_DARK_GREEN), target)
            .unwrap();

        Circle::with_center(Point::new(i as i32, j as i32), 30)
            .draw_styled(&PrimitiveStyle::with_fill(Rgb888::WHITE), target)
            .unwrap();
        Circle::with_center(Point::new(i as i32, j as i32), 15)
            .draw_styled(&PrimitiveStyle::with_fill(Rgb888::RED), target)
            .unwrap();
        Circle::with_center(Point::new((i + 4) as i32, (j + 4) as i32), 4)
            .draw_styled(&PrimitiveStyle::with_fill(Rgb888::BLUE), target)
            .unwrap();
        Circle::with_center(Point::new(i as i32 - 4, j as i32 - 4), i.max(0) as u32 / 10)
            .draw_styled(&PrimitiveStyle::with_fill(Rgb888::GREEN), target)
            .unwrap();
        for _ in 0..10 {
            Circle::with_center(Point::new(i as i32 + 6, j as i32 - 8), 4)
                .draw_styled(&PrimitiveStyle::with_fill(Rgb888::YELLOW), target)
                .unwrap();
        }
    }
}
