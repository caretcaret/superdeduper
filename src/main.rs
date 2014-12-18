extern crate image;

use std::ascii::AsciiExt;
use std::io::{fs, File};
use std::io::fs::PathExtensions;
use std::path::Path;
use std::os;

// extension-based detection of filetype.
fn supported_extension(path: &Path) -> Option<image::ImageFormat> {
    path.extension_str().and_then(|ext| {
        match ext.to_ascii_lower().as_slice() {
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
    })
}

// defines anything that can act as a signature that we can use to compare images,
// whether that be a locality-sensitive hash or computer vision features, or some
// combination thereof.
trait ImageSignature {
    fn new(image: &image::DynamicImage) -> Self;

    fn similarity(&self, other: &Self) -> f64;
    fn similar(&self, other: &Self, threshold: f64) -> bool {
        self.similarity(other) >= threshold
    }
}

// every image maps to the same thing.
impl ImageSignature for () {
    #[allow(unused_variables)]
    fn new(image: &image::DynamicImage) { }
    #[allow(unused_variables)]
    fn similarity(&self, other: &()) -> f64 { 1.0f64 }
}

// read files and generate signatures for them
fn process_image<T: ImageSignature>(
        path: &Path,
        format: image::ImageFormat
    ) -> Option<T> {
    File::open(path).ok().and_then(|file| {
        image::load(file, format).ok()
    }).map(|image| {
        println!("{}", path.as_str().unwrap_or("<unknown path>"));
        ImageSignature::new(&image)
    })
}

fn main() {
    // get the directory name
    let dir = match os::args().as_slice() {
        [_, ref path, ..] => { Path::new(&path) },
        args => {
            println!("Usage: {} <directory>", args[0]);
            return;
        },
    };
    // for each path in the directory, process the image
    for path in fs::walk_dir(&dir).unwrap() {
        if path.is_file() {
            supported_extension(&path).map(|format| {
                process_image::<()>(&path, format)
            });
        }
    }
}
