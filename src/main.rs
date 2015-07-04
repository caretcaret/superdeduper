extern crate image;
extern crate rustc_serialize;
extern crate docopt;
extern crate glob;

use docopt::Docopt;
use std::fs;
use std::fmt;
use std::path::Path;
use std::collections::HashMap;
use std::ascii::AsciiExt;
use std::f32;
use image::GenericImage;
use image::Pixel;

static USAGE: &'static str = "
Usage: superdeduper <dir>
";

#[derive(RustcDecodable, Debug)]
struct Args {
    arg_dir: String,
}

// extension-based detection of filetype
fn supported_extension(path: &Path) -> Option<image::ImageFormat> {
    match path.extension() {
        None => { None },
        Some(ext) => {
            match ext.to_str().unwrap().to_ascii_lowercase().as_ref() {
                "gif" => { Some(image::ImageFormat::GIF) },
                "png" => { Some(image::ImageFormat::PNG) },
                "png-large" => { Some(image::ImageFormat::PNG) },
                "jpg" => { Some(image::ImageFormat::JPEG) },
                "jpeg" => { Some(image::ImageFormat::JPEG) },
                "jpe" => { Some(image::ImageFormat::JPEG) },
                "jpg-large" => { Some(image::ImageFormat::JPEG) },
                "webp" => { Some(image::ImageFormat::WEBP) },
                _ => { None },
            }
        }
    }
}

trait ImageSignature {
    fn new(image: &image::DynamicImage) -> Self;

    fn similarity(&self, other: &Self) -> f64;
    fn is_similar(&self, other: &Self) -> bool {
        self.similarity(other) >= 0.99f64
    }
}

#[derive(Debug)]
struct PHash(u64);

impl fmt::LowerHex for PHash {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            &PHash(repr) => { repr.fmt(formatter) }
        }
    }
}

impl ImageSignature for PHash {
    fn new(image: &image::DynamicImage) -> PHash {
        // Grayscale and resize image to 32x32
        let resized = image.grayscale().resize_exact(32, 32, image::FilterType::Nearest);
        // Compute top-left 8x8 of discrete cosine transform
        // indexed by (n from 0 to 31, k from 0 to 7)
        let mut cosines: [f32; 256] = [0.0; 256];
        let mut transformed: [f32; 64] = [0.0; 64];
        // compute cosine terms
        for i in 0..32 {
            for j in 0..8 {
                cosines[8 * i + j] = f32::cos(f32::consts::PI / 32.0 * (i as f32 + 0.5) * j as f32); 
            }
        }
        // compute transform terms
        for k1 in 0..8 {
            for k2 in 0..8 {
                for n1 in 0..32 {
                    for n2 in 0..32 {
                        match resized.get_pixel(n1 as u32, n2 as u32).channels4() {
                            (r, _, _, _) => {
                                transformed[8 * k1 + k2] += cosines[8 * n1 + k1] * cosines[8 * n2 + k2] * (r as f32 - 128.0);
                            }
                        }
                    }
                }
            }
        }
        // Compute average value, excluding DC factor at (0, 0)
        let mut average = 0.0f32;
        for i in 1..64 {
            average += transformed[i] / 63.0;
        }

        // Compare each pixel to average value
        let mut hash_value = 0u64;
        for i in 0..64 {
            if transformed[i] >= average {
                hash_value |= 1 << i;
            }
        }
        PHash(hash_value)
    }

    fn similarity(&self, other: &PHash) -> f64 {
        // metric: hamming distance of two hashes
        match (self, other) {
            (&PHash(h1), &PHash(h2)) => {
                ((h1 ^ h2).count_zeros() as f64) / 64.0
            }
        }
    }
}

// read files and generate signatures for them
fn process_image<T: ImageSignature>(
        path: &Path,
        format: image::ImageFormat
    ) -> Option<T> {
    fs::File::open(path).ok().and_then(|file| {
        image::load(file, format).ok()
    }).map(|image| {
        ImageSignature::new(&image)
    })
}

fn main() {
    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());

    // container for signatures
    let mut signatures = HashMap::new();

    // generate a cache file for the signatures we'll be generating
    // TODO

    // for each path in the directory
    for glob_result in glob::glob(&(args.arg_dir + "/*")).unwrap() {
        let path = glob_result.unwrap();
        if fs::metadata(&path).unwrap().is_file() {
            supported_extension(&path).and_then(|format| {
                // create the image signature
                process_image::<PHash>(&path, format)
            }).map(|sig| {
                println!("{:016x} {}", sig, path.display());
                // add it to the hash table
                signatures.insert(path.clone(), sig);
            });
        }
    }

}
