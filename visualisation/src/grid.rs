/// A grid of size `(W, H)` containing a value of type `T`
pub struct Grid<T, const W: usize, const H: usize>
where
    [(); W * H]: Sized,
{
    buffer: [T; W * H],
}

impl<T: Copy, const W: usize, const H: usize> Grid<T, W, H>
where
    [(); W * H]: Sized,
{
    pub fn new(fill_value: T) -> Self {
        Self {
            buffer: [fill_value; W * H],
        }
    }
}

impl<T, const W: usize, const H: usize> Grid<T, W, H>
where
    [(); W * H]: Sized,
{
    fn x_y_to_index(x: i32, y: i32) -> Option<usize> {
        if (y >= 0) & (y < H as i32) & (x >= 0) & (x < W as i32) {
            Some(y as usize * W + x as usize)
        } else {
            None
        }
    }

    pub fn buffer(&self) -> &[T; W * H] {
        &self.buffer
    }

    pub fn buffer_mut(&mut self) -> &mut [T; W * H] {
        &mut self.buffer
    }

    pub fn get(&self, x: i32, y: i32) -> Option<&T> {
        let index = Self::x_y_to_index(x, y)?;
        Some(&self.buffer[index])
    }

    pub fn get_mut(&mut self, x: i32, y: i32) -> Option<&mut T> {
        let index = Self::x_y_to_index(x, y)?;
        Some(&mut self.buffer[index])
    }

    pub fn set(&mut self, x: i32, y: i32, value: T) {
        if let Some(i) = Self::x_y_to_index(x, y) {
            self.buffer[i] = value;
        }
    }

    pub fn iter_coords() -> impl Iterator<Item = (i32, i32)> {
        (0..H as i32).flat_map(|y| (0..W as i32).map(move |x| (x, y)))
    }

    pub fn iter_with_index(&self) -> impl Iterator<Item = ((i32, i32), &T)> {
        Grid::<T, W, H>::iter_coords().zip(self.buffer.iter())
    }

    pub fn iter_mut_with_index(&mut self) -> impl Iterator<Item = ((i32, i32), &mut T)> {
        Grid::<T, W, H>::iter_coords().zip(self.buffer.iter_mut())
    }
}
