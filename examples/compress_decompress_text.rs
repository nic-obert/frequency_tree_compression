use std::fs;

use frequency_tree_compression::{compress, decompress};


fn main() {

    // Compress 

    let original_text = fs::read_to_string("test_data/lorem.txt")
        .unwrap_or_else(|err| panic!("Could not open file: {}", err));

    let compressed = compress(original_text.chars());

    fs::write("test_data/compressed/lorem.txt.compressed", compressed)
        .unwrap_or_else(|err| panic!("Could not write to file: {}", err));


    // Decompress

    let compressed = fs::read("test_data/compressed/lorem.txt.compressed")
        .unwrap_or_else(|err| panic!("Could not read file {}", err));

    let decompressed = decompress::<char>(&compressed)
        .unwrap_or_else(|err| panic!("Could not decompress data {:?}", err));

    let decompressed_text: String = decompressed.iter().collect();

    assert_eq!(original_text, decompressed_text);
}