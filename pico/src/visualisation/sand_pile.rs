use super::{StateUpdate, Visualisation};

pub struct SandPile<const R: usize, const C: usize>
where
    [(); R * C]:,
{
    sand: [u8; R * C],
}

impl<const R: usize, const C: usize> SandPile<R, C>
where
    [(); R * C]:,
{
    pub fn new() -> Self
    where
        [(); R * C]:,
    {
        SandPile { sand: [0; R * C] }
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
        todo!()
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
        todo!()
    }
}
