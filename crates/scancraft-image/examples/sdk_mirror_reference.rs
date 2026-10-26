//! Synthetic pixels only; stdout is binary for an optional offline comparison.
//! Does not select a scanner orientation, calibration or device setting.
use scancraft_image::GrayImage;
use std::io::{self, Write};

fn main() -> io::Result<()> {
    let mut output = io::BufWriter::new(io::stdout().lock());
    for (width, height) in [(1, 1), (1, 7), (2, 3), (7, 3), (8, 5), (1920, 1200)] {
        let count = width as usize * height as usize;
        let w = width as usize;
        let pixels: Vec<u8> = (0..count)
            .map(|i| ((i * 73 + (i / w) * 19 + (i % w) * 11) ^ ((i * 7) >> 8)) as u8)
            .collect();
        let mut reflected = vec![0; count];
        GrayImage::new(width, height, &pixels)
            .map_err(io::Error::other)?
            .reflect_x_into(&mut reflected)
            .map_err(io::Error::other)?;
        output.write_all(&reflected)?;
    }
    output.flush()
}
