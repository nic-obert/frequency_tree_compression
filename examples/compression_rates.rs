use std::fs;

use frequency_tree_compression::compress;


fn main() {

    let text = fs::read_to_string("test_data/animal_farm.txt")
        .unwrap_or_else(|err| panic!("Could not open file {err}"));

    let compressed = compress(text.chars());

    println!("Animal Farm by George Orwell (Project Gutenberg edition)\nOriginal size: {} KiB\nCompressed size: {} KiB\nCompression rate: {:.2}",
        text.len() / 1024, compressed.len() / 1024, text.len() as f64 / compressed.len() as f64);

}

