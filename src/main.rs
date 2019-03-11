extern crate crossbeam;

extern crate image;
use image::png::PNGEncoder;
use image::ColorType;

extern crate num;
use num::Complex;

extern crate num_cpus;

use std::fs::File;
use std::str::FromStr;

fn escape_time(c: Complex<f64>, limit: u32) -> Option<u32> {
    let mut z = c;
    for i in 1..limit {
        let re_sq = z.re * z.re;
        let im_sq = z.im * z.im;
        z = Complex {
            re: re_sq - im_sq + c.re,
            im: 2.0 * z.re * z.im + c.im,
        };
        if re_sq + im_sq > 4.0 {
            return Some(i);
        }
    }
    None
}

fn parse_pair<T: FromStr>(s: &str, sep: char) -> Option<(T, T)> {
    s.find(sep).and_then(|idx| {
        s[..idx]
            .parse::<T>()
            .and_then(|t1| {
                s[idx + sep.len_utf8()..]
                    .parse::<T>()
                    .and_then(|t2| Ok((t1, t2)))
            })
            .ok()
    })
}

fn parse_complex(s: &str) -> Option<Complex<f64>> {
    parse_pair(s, ',').map(|(re, im)| Complex { re, im })
}

fn pixel_to_point(
    bounds: (usize, usize),
    pixel: (usize, usize),
    upper_left: Complex<f64>,
    lower_right: Complex<f64>,
) -> Complex<f64> {
    let (width, height) = (
        lower_right.re - upper_left.re,
        upper_left.im - lower_right.im,
    );
    Complex {
        re: upper_left.re + pixel.0 as f64 * width / bounds.0 as f64,
        im: upper_left.im - pixel.1 as f64 * height / bounds.1 as f64,
    }
}

fn render(
    pixels: &mut [u8],
    bounds: (usize, usize),
    upper_left: Complex<f64>,
    lower_right: Complex<f64>,
) {
    assert!(pixels.len() == bounds.0 * bounds.1);

    for row in 0..bounds.1 {
        for col in 0..bounds.0 {
            let point = pixel_to_point(bounds, (col, row), upper_left, lower_right);
            let time = escape_time(point, 255).unwrap_or(255);
            pixels[row * bounds.0 + col] = 255 - time as u8;
        }
    }
}

fn write_image(
    filename: &str,
    pixels: &[u8],
    bounds: (usize, usize),
) -> Result<(), std::io::Error> {
    let output = File::create(filename)?;
    let encoder = PNGEncoder::new(output);
    encoder.encode(
        &pixels,
        bounds.0 as u32,
        bounds.1 as u32,
        ColorType::Gray(8),
    )?;
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 5 {
        eprintln!("Useage: mandelbrot FILENAME PIXELS UPPERLEFT LOWERRIGHT");
        std::process::exit(1);
    }
    let bounds = parse_pair(&args[2], 'x').expect("error parsing image dimensions");
    let upper_left = parse_complex(&args[3]).expect("error parsing the upper left corner");
    let lower_right = parse_complex(&args[4]).expect("error parsing the lower right corner");
    let mut pixels = vec![0; bounds.0 * bounds.1];

    let threads = num_cpus::get();
    let rows_per_band = bounds.1 / threads + 1;
    {
        let bands: Vec<&mut [u8]> = pixels.chunks_mut(rows_per_band * bounds.0).collect();
        crossbeam::scope(|spawner| {
            for (i, band) in bands.into_iter().enumerate() {
                let top = rows_per_band * i;
                let height = band.len() / bounds.0;
                let band_bounds = (bounds.0, height);
                let band_upper_left = pixel_to_point(bounds, (0, top), upper_left, lower_right);
                let band_lower_right =
                    pixel_to_point(bounds, (bounds.0, top + height), upper_left, lower_right);
                spawner.spawn(move |_| {
                    render(band, band_bounds, band_upper_left, band_lower_right);
                });
            }
        })
        .expect("error generating image");
    }
    write_image(&args[1], &pixels, bounds).expect("error writing PNG file");
}
