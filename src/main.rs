use std::{fs, io::Read};

use frequency_tree_compression::EncodingTree;


fn main() {


    // let text = fs::read_to_string("test_data/text.txt").unwrap();

    // let (encoder, encoded) = EncodingTree::encode(text.chars());

    // let bytes: Vec<u8> = encoded.bytes().map(|res| res.unwrap()).collect();

    // // println!("Bytes: {:?}", bytes)

    // let bits = bytes.view_bits::<Lsb0>();

    // let decoded: String = encoder.decode(bits).unwrap().into_iter().collect();

    // assert_eq!(text, decoded);

    // let compressed_size = bytes_repr_for_bits(encoded.len());
    // println!("Original size: {} bytes\nCompressed size: {} bytes\nCompression ratio: {}%", text.len(), compressed_size, (compressed_size as f64 / text.len() as f64 * 100.0) as i32);


}

