use std::fs;

use frequency_tree_compression::EncodingTree;


fn bytes_repr_for_bits(count: usize) -> usize {
    count / 8 + count % 8
} 


fn main() {

    let text = fs::read_to_string("filo.txt").unwrap();

    let (encoder, encoded) = EncodingTree::encode(text.chars());

    let decoded: String = encoder.decode(&encoded).unwrap().into_iter().collect();

    assert_eq!(text, decoded);

    let compressed_size = bytes_repr_for_bits(encoded.len());
    println!("Original size: {} bytes\nCompressed size: {} bytes\nCompression ratio: {}%", text.len(), compressed_size, (compressed_size as f64 / text.len() as f64 * 100.0) as i32);

}

