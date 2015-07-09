extern crate image;
extern crate rustc_serialize;
extern crate docopt;
extern crate glob;

use docopt::Docopt;
use std::fs;
use std::fmt;
use std::path::{Path, PathBuf};
use std::ascii::AsciiExt;
use std::f32;
use image::GenericImage;
use image::Pixel;

static USAGE: &'static str = "
Image deduplicator. Implemented using the pHash perceptual hash algorithm.
This program moves all images from source to target, renaming similar images
as <canonical hash>-<dupe number>-<image hash> for easy recognition and deletion.

Usage: superdeduper <source> <target>
";

#[derive(RustcDecodable, Debug)]
struct Args {
    arg_source: String,
    arg_target: String,
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

trait ImageSignature: fmt::Display {
    fn new(image: &image::DynamicImage) -> Self;

    fn distance(&self, other: &Self) -> u32;
    fn is_similar(distance: u32) -> bool;
    // for human-interpretable measurements of similarity
    fn similarity(&self, other: &Self) -> f64;
}

#[derive(Debug)]
struct PHash(u64);

impl fmt::Display for PHash {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            &PHash(repr) => { write!(formatter, "{:016x}", repr) }
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

    fn distance(&self, other: &PHash) -> u32 {
        // metric: hamming distance of two hashes
        match (self, other) {
            (&PHash(h1), &PHash(h2)) => {
                (h1 ^ h2).count_ones()
            }
        }
    }

    fn is_similar(distance: u32) -> bool {
        distance < 8
    }

    fn similarity(&self, other: &PHash) -> f64 {
        1.0 - (self.distance(other) as f64 / 64.0)
    }
}

#[derive(Clone)]
struct ProcessedImage<T: ImageSignature> {
  sig: T,
  path: PathBuf,
  size: u64,
}

// read files and generate signatures for them
fn process_image<T: ImageSignature>(
      pathbuf: PathBuf,
      format: image::ImageFormat
  ) -> Option<ProcessedImage<T>> {
  fs::File::open(pathbuf.as_path()).ok().and_then(|file| {
      let im = image::load(file, format);

      match im {
        Err(err) => {
          // image could not be read by image library
          println!("[{}] {}", err, pathbuf.display());
          None
        },
        Ok(image) => { Some(image) }
      }
    }).map(|image| {
      ProcessedImage {
        sig: ImageSignature::new(&image),
        path: pathbuf,
        size: (image.width() as u64) * (image.height() as u64)
      }
  })
}

fn new_filename<T: ImageSignature>(
    old_path: &PathBuf,
    new_directory: &Path,
    canon_signature: &T,
    this_signature: &T,
    version: u32,
  ) -> PathBuf {
  let mut new_path = PathBuf::new();
  new_path.push(new_directory);

  // show indication of repetition and actual image signature
  if version != 0 {
    new_path.push(format!("{}-{}-{}", canon_signature, version, this_signature));
  } else {
    new_path.push(format!("{}", canon_signature));
  }

  match supported_extension(old_path) {
    Some(image::ImageFormat::GIF) => { new_path.set_extension("gif"); },
    Some(image::ImageFormat::PNG) => { new_path.set_extension("png"); },
    Some(image::ImageFormat::JPEG) => { new_path.set_extension("jpg"); },
    Some(image::ImageFormat::WEBP) => { new_path.set_extension("webp"); },
    _ => {}
  }

  new_path
}

fn main() {
  let args: Args = Docopt::new(USAGE)
                          .and_then(|d| d.decode())
                          .unwrap_or_else(|e| e.exit());

  // container for image metadata and signatures
  let mut processed_images = Vec::new();
  let new_directory = Path::new(&args.arg_target);

  // inline renaming not implemented, don't be destructive
  assert!(args.arg_source != args.arg_target);

  println!("[Reading images.]");
  // for each path in the directory
  for glob_result in glob::glob(&(args.arg_source + "/*")).unwrap() {
      let pathbuf: PathBuf = glob_result.unwrap();
      if fs::metadata(pathbuf.as_path()).unwrap().is_file() {
          supported_extension(pathbuf.as_path()).and_then(|format| {
              // create the image signature
              process_image::<PHash>(pathbuf, format)
          }).map(|processed_image| {
              println!("{} {}", processed_image.sig, processed_image.path.display());
              processed_images.push(processed_image);
          });
      }
  }
  println!("[{} files read.]", processed_images.len());

  println!("[Finding dupes. This might take a while!]");
  let mut dupes: Vec<Vec<ProcessedImage<PHash>>> = Vec::new();

  // get an image with largest resolution, find its neighbors until empty.
  // not very rustic because my rust is very rusty :(
  while !processed_images.is_empty() {
    let mut neighbors = Vec::new();
    let image = processed_images.pop().unwrap();

    let mut i = processed_images.len();

    loop {
      if i == 0 { break }
      i -= 1;
      if PHash::is_similar(processed_images[i].sig.distance(&image.sig)) {
        neighbors.push(processed_images.remove(i));
      }
    }

    neighbors.push(image);
    dupes.push(neighbors);
  }

  dupes.sort_by(|a, b| { b.len().cmp(&a.len()) });

  println!("[Moving files.]");
  match fs::create_dir_all(new_directory) {
    Err(err) => { println!("{}", err); },
    Ok(_) => { }
  }
  for group in dupes.iter() {
    assert!(group.len() > 0);

    // canonical image is the one with the largest file size
    let canon = &group[group.len() - 1].sig;
    for (i, image) in group.iter().enumerate() {
      let new_loc = new_filename(&image.path, &new_directory, canon, &image.sig, i as u32);
      println!("{} => {}", image.path.display(), new_loc.display());
      match fs::rename(&image.path, &new_loc) {
        Err(err) => { println!("{}", err); },
        Ok(_) => { }
      }
    }
  }

}
