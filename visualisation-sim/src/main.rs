use std::convert::Infallible;

use eframe::NativeOptions;
use egui::{CentralPanel, ColorImage, Image, ImageData, TextureHandle, TextureOptions};
use embedded_graphics::{Pixel, pixelcolor::Rgb888};

struct App {
    dimensions: (usize, usize),
    displaying_t1: bool,
    t1: TextureHandle,
    t2: TextureHandle,
}

impl App {
    pub fn new(dimensions: (usize, usize), cc: &eframe::CreationContext) -> Self {
        let t1 = cc.egui_ctx.load_texture(
            "t1",
            ColorImage::from_gray(
                [dimensions.0, dimensions.1],
                &vec![255; dimensions.0 * dimensions.1],
            ),
            TextureOptions::NEAREST,
        );
        let t2 = cc.egui_ctx.load_texture(
            "t2",
            ColorImage::from_gray(
                [dimensions.0, dimensions.1],
                &vec![255; dimensions.0 * dimensions.1],
            ),
            TextureOptions::NEAREST,
        );

        App {
            dimensions,
            displaying_t1: true,
            t1,
            t2,
        }
    }
}

impl embedded_graphics::geometry::Dimensions for App {
    fn bounding_box(&self) -> embedded_graphics::primitives::Rectangle {
        embedded_graphics::primitives::Rectangle::new(
            embedded_graphics::prelude::Point::zero(),
            embedded_graphics::prelude::Size::new(
                self.dimensions.0 as u32,
                self.dimensions.1 as u32,
            ),
        )
    }
}

impl embedded_graphics::draw_target::DrawTarget for App {
    type Color = Rgb888;

    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        todo!()
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            Image::new(&self.t1)
                .fit_to_original_size(10.0)
                .paint_at(ui, ui.max_rect())
        });
    }
}

fn main() {
    eframe::run_native(
        "visualisation sim",
        NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(App::new((100, 100), cc)))),
    )
    .unwrap();
}
