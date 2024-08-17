use std::fs;

use frequency_tree_compression::{compress, decompress};


fn main() {

    let mut text = fs::read_to_string("test_data/lorem.txt")
        .unwrap_or_else(|err| panic!("Could not open file: {}", err));

    let mut char_count = text.chars().count();
    if char_count % 2 != 0 {
        text.push('\n');
        char_count += 1;
    }

    #[derive(Debug, PartialEq, Eq, Hash, Clone)]
    struct DoubleChar ([char; 2]);

    let mut dchars = Vec::with_capacity(char_count / 2);
    let mut it = text.chars();
    while let Some(ch1) = it.next() {
        dchars.push(DoubleChar(
            [ch1, it.next().unwrap()]
        ));
    }

    let compressed_dchar = compress::<DoubleChar>(dchars.iter().cloned());

    let compressed_regular = compress::<char>(text.chars());

    let decompressed = decompress::<DoubleChar>(&compressed_dchar).unwrap();

    let mut s = String::with_capacity(text.len());
    for dc in decompressed.iter() {
        s.push(dc.0[0]);
        s.push(dc.0[1]);
    }

    assert_eq!(s, text);

    println!("Original size: {} KiB\nRegular compressed size: {} KiB\nGrouped grapheme compressed size: {} KiB\nGrouped grapheme compression ratio: {:.2}\nGrouped grapheme improvement: {:.2}",
        text.len() / 1024, compressed_regular.len() / 1024, compressed_dchar.len() / 1024, text.len() as f64 / compressed_dchar.len() as f64, compressed_regular.len() as f64 / compressed_dchar.len() as f64);

}
