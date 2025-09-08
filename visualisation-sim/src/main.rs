#![feature(generic_const_exprs)]
use std::{
    convert::Infallible,
    time::{Duration, Instant},
};

use eframe::NativeOptions;
use egui::{CentralPanel, ColorImage, Image, ImageData, TextureHandle, TextureOptions};
use embedded_graphics::{Pixel, pixelcolor::Rgb888, prelude::RgbColor};
use rand::RngCore;
use visualisation::{GameOfLife, RngU32, SandPile, TestVis};

struct Buffer<const W: usize, const H: usize>
where
    [(); W * H * 3]: Sized,
{
    buffer: [u8; W * H * 3],
}

struct App<const W: usize, const H: usize>
where
    [(); W * H * 3]: Sized,
{
    state: visualisation::CurrentState,
    texture: TextureHandle,
    buffer: Buffer<W, H>,
    last_update: Instant,
}

struct RandU32Rng;

impl RngU32 for RandU32Rng {
    fn next_u32(&mut self) -> u32 {
        rand::rng().next_u32()
    }
}

impl<const W: usize, const H: usize> App<W, H>
where
    [(); W * H * 3]: Sized,
{
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let buffer = [0; W * H * 3];
        let texture = cc.egui_ctx.load_texture(
            "texture",
            ColorImage::from_rgb([W, H], &buffer),
            TextureOptions::NEAREST,
        );

        let mut gol = GameOfLife::new_with_random(1000, RandU32Rng);

        App {
            state: visualisation::CurrentState::GameOfLife(gol),
            texture,
            buffer: Buffer { buffer },
            last_update: Instant::now(),
        }
    }

    pub fn clear(&mut self) {
        self.buffer.buffer.fill(0);
    }

    pub fn blit(&mut self) {
        self.texture.set(
            ColorImage::from_rgb([W, H], &self.buffer.buffer),
            TextureOptions::NEAREST,
        );
    }
}

impl<const W: usize, const H: usize> embedded_graphics::geometry::Dimensions for Buffer<W, H>
where
    [(); W * H * 3]: Sized,
{
    fn bounding_box(&self) -> embedded_graphics::primitives::Rectangle {
        embedded_graphics::primitives::Rectangle::new(
            embedded_graphics::prelude::Point::zero(),
            embedded_graphics::prelude::Size::new(W as u32, H as u32),
        )
    }
}

impl<const W: usize, const H: usize> embedded_graphics::draw_target::DrawTarget for Buffer<W, H>
where
    [(); W * H * 3]: Sized,
{
    type Color = Rgb888;

    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, colour) in pixels {
            if (point.x >= 0) & (point.x < W as i32) & (point.y >= 0) & (point.y < H as i32) {
                let index = (W * point.y as usize + point.x as usize) * 3;
                self.buffer[index + 0] = colour.r();
                self.buffer[index + 1] = colour.g();
                self.buffer[index + 2] = colour.b();
            }
        }
        Ok(())
    }
}

impl<const W: usize, const H: usize> eframe::App for App<W, H>
where
    [(); W * H * 3]: Sized,
{
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let time_since_last = self.last_update.elapsed();
        if time_since_last > Duration::from_millis(100) {
            self.last_update = Instant::now();
            self.state.update(time_since_last.as_micros() as u32);
            self.clear();
            self.state.draw(&mut self.buffer);
            self.blit();
        }

        CentralPanel::default().show(ctx, |ui| {
            Image::new(&self.texture).paint_at(ui, ui.max_rect())
        });
        ctx.request_repaint();
    }
}

fn main() {
    eframe::run_native(
        "visualisation sim",
        NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(App::<64, 32>::new(cc)))),
    )
    .unwrap();
}
