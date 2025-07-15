use embassy_rp::dma::Channel;
use embedded_graphics::{
    Pixel,
    pixelcolor::Rgb888,
    prelude::{DrawTarget, OriginDimensions, Size},
};

use crate::{Display, fb_bytes};

pub struct FrameBuffer<'a, const W: usize, const H: usize, FB_CH, FB_L_CH, OE_CH, OE_L_CH>
where
    [(); fb_bytes(W, H, 8)]: Sized,
    FB_CH: Channel,
    FB_L_CH: Channel,
    OE_CH: Channel,
    OE_L_CH: Channel,
{
    data: &'a mut [u8; fb_bytes(W, H, 8)],
    display: &'a Display<'a, W, H, FB_CH, FB_L_CH, OE_CH, OE_L_CH>,
}

impl<'a, const W: usize, const H: usize, FB_CH, FB_L_CH, OE_CH, OE_L_CH> OriginDimensions
    for FrameBuffer<'a, W, H, FB_CH, FB_L_CH, OE_CH, OE_L_CH>
where
    [(); fb_bytes(W, H, 8)]: Sized,
    FB_CH: Channel,
    FB_L_CH: Channel,
    OE_CH: Channel,
    OE_L_CH: Channel,
{
    fn size(&self) -> Size {
        Size::new(W.try_into().unwrap(), H.try_into().unwrap())
    }
}

impl<'a, const W: usize, const H: usize, FB_CH, FB_L_CH, OE_CH, OE_L_CH>
    FrameBuffer<'a, W, H, FB_CH, FB_L_CH, OE_CH, OE_L_CH>
where
    [(); fb_bytes(W, H, 8)]: Sized,
    FB_CH: Channel,
    FB_L_CH: Channel,
    OE_CH: Channel,
    OE_L_CH: Channel,
{
    pub fn new(
        data: &'a mut [u8; fb_bytes(W, H, 8)],
        display: &'a Display<'a, W, H, FB_CH, FB_L_CH, OE_CH, OE_L_CH>,
    ) -> Self {
        FrameBuffer { data, display }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: Rgb888) {
        // invert the screen
        let x = W - 1 - x;
        let y = H - 1 - y;
        // Half of the screen
        let h = y > (H / 2) - 1;
        let shift = if h { 3 } else { 0 };
        let (c_r, c_g, c_b) = self.display.lut.lookup(color);
        let c_r: u16 = ((c_r as f32) * (self.display.brightness as f32 / 255f32)) as u16;
        let c_g: u16 = ((c_g as f32) * (self.display.brightness as f32 / 255f32)) as u16;
        let c_b: u16 = ((c_b as f32) * (self.display.brightness as f32 / 255f32)) as u16;
        let base_idx = x + ((y % (H / 2)) * W * 8);
        for b in 0..8 {
            // Extract the n-th bit of each component of the color and pack them
            let cr = c_r >> b & 0b1;
            let cg = c_g >> b & 0b1;
            let cb = c_b >> b & 0b1;
            let packed_rgb = (cb << 2 | cg << 1 | cr) as u8;
            let idx = base_idx + b * W;
            self.data[idx] &= !(0b111 << shift);
            self.data[idx] |= packed_rgb << shift;
        }
    }
}
impl<'a, const W: usize, const H: usize, FB_CH, FB_L_CH, OE_CH, OE_L_CH> DrawTarget
    for FrameBuffer<'a, W, H, FB_CH, FB_L_CH, OE_CH, OE_L_CH>
where
    [(); fb_bytes(W, H, 8)]: Sized,
    FB_CH: Channel,
    FB_L_CH: Channel,
    OE_CH: Channel,
    OE_L_CH: Channel,
{
    type Color = Rgb888;

    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            if coord.x < <usize as TryInto<i32>>::try_into(W).unwrap()
                && coord.y < <usize as TryInto<i32>>::try_into(H).unwrap()
                && coord.x >= 0
                && coord.y >= 0
            {
                self.set_pixel(coord.x as usize, coord.y as usize, color);
            }
        }

        Ok(())
    }
}
