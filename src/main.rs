extern crate "rustc-serialize" as rustc_serialize;
extern crate docopt;
extern crate image;

use docopt::Docopt;
use std::ascii::AsciiExt;
use std::collections::{HashMap, HashSet};
use std::fmt::Show;
use std::hash::Hash;
use std::io::{fs, File};
use std::io::fs::PathExtensions;
use std::path::Path;

#[deriving(Decodable, Show)]
enum SignatureType {
    Constant,
}

static USAGE: &'static str = "
Usage: dedupe [options] <directory>

Options:
    -v, --verbose      Show filenames and hashes during processing.
    --signature TYPE   Use a particular image signature to detect similarity.
                       Valid values: constant [default: constant]
";

#[derive(RustcDecodable, Show)]
struct Args {
    arg_directory: String,
}

// defines anything that can act as a signature that we can use to compare images,
// whether that be a locality-sensitive hash or computer vision features, or some
// combination thereof.
trait ImageSignature: Show {
    fn new(image: &image::DynamicImage) -> Self;

    fn similarity(&self, other: &Self) -> f64;
    fn is_similar(&self, other: &Self) -> bool {
        self.similarity(other) >= 0.99f64
    }
}

// every image maps to the same thing.
impl ImageSignature for () {
    #[allow(unused_variables)]
    fn new(image: &image::DynamicImage) { }
    #[allow(unused_variables)]
    fn similarity(&self, other: &()) -> f64 { 1.0f64 }
}

//struct HashUnionFind<T> {
//    map: HashMap<T>
//}
//
//trait UnionFind<T: Hash + Eq> {
//    fn make_set(&self, elt: &T);
//    fn union(&self, e1: &T, e2: &T);
//    fn components_with(&self, pred: FnOnce<(HashSet<T>,), bool>)
//        -> Iterator<HashSet<T>>;
//
//    fn components(&self) -> Iterator<HashSet<T>> {
//        self.components_with(|_| { true })
//    }
//}
//
//impl<T> UnionFind for HashUnionFind<T> {
//    fn make_set(&self, elt: &T) {
//        unimplemented!();
//    }
//
//    fn union(&self, e1: &T, e2: &T) {
//        unimplemented!();
//    }
//
//    fn components_with(&self, pred: FnOnce<(HashSet<T>,), bool>)
//        -> Iterator<HashSet<T>> {
//        unimplemented!();
//    }
//}

// extension-based detection of filetype.
fn supported_extension(path: &Path) -> Option<image::ImageFormat> {
    path.extension_str().and_then(|ext| {
        match ext.to_ascii_lowercase().as_slice() {
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

// read files and generate signatures for them
fn process_image<T: ImageSignature>(
        path: &Path,
        format: image::ImageFormat
    ) -> Option<T> {
    File::open(path).ok().and_then(|file| {
        image::load(file, format).ok()
    }).map(|image| {
        ImageSignature::new(&image)
    })
}

fn main() {
    // parse args
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());
    let dir = Path::new(&args.arg_directory);

    // container for signatures
    let mut signatures = HashMap::new();

    // generate a cache file for the signatures we'll be generating
    // TODO

    // for each path in the directory
    for path in fs::walk_dir(&dir).unwrap() {
        if path.is_file() {
            supported_extension(&path).and_then(|format| {
                // create the image signature
                process_image::<()>(&path, format)
            }).map(|sig| {
                // add it to the hash table
                signatures.insert(path.clone(), sig);
                println!("{}: {:?}", path.display(), sig);
            });
        }
    }

    //// naive pairwise comparison using union-find
    //let mut signature_sets = HashUnionFind::new();
    //for path in signatures.keys() {
    //    signature_sets.make_set(path);
    //}
    //for (path1, hash1) in signatures.iter() {
    //    for (path2, hash2) in signatures.iter() {
    //        if path1 == path2 {
    //            continue
    //        }
    //        if hash1.is_similar(&hash2) {
    //           signatures_sets.union(path1, path2) 
    //        }
    //    }
    //}

    //println!("{}", signature_sets.components_with(|component| component.len() >= 2));
}
