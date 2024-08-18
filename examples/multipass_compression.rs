use core::str;
use std::borrow::Cow;
use std::io;
use std::fs;
use std::str::Utf8Error;

use frequency_tree_compression::{compress, decompress, DecompressionError};


enum OwnedOrBorrowedBytes<'a> {
    Owned(Box<[u8]>),
    Borrowed(&'a [u8])
}

impl OwnedOrBorrowedBytes<'_> {

    pub const fn bytes(&self) -> &[u8] {
        match self {
            OwnedOrBorrowedBytes::Owned(bytes) => bytes,
            OwnedOrBorrowedBytes::Borrowed(bytes) => bytes,
        }
    }


    pub fn assume_owned(self) -> Box<[u8]> {
        if let Self::Owned(bytes) = self {
            bytes
        } else {
            unreachable!()
        }
    }

}


#[derive(Debug, Clone, Copy)]
enum MultipassCompressionError {

    MissingCompressionLevelSpecifier,
    MissingCompressedData,
    InvalidStringEncoding (Utf8Error),
    DecompressionError (DecompressionError)

}


enum CompressionLevel<'a> {
    Uncompressed (&'a [u8]),
    Compressed { level: u8, bytes: OwnedOrBorrowedBytes<'a> }
}

impl CompressionLevel<'_> {

    pub const fn level(&self) -> u8 {
        match self {
            CompressionLevel::Uncompressed(_) => 0,
            CompressionLevel::Compressed { level, .. } => *level,
        }
    }


    pub const fn bytes(&self) -> &[u8] {
        match self {
            CompressionLevel::Uncompressed(bytes) => bytes,
            CompressionLevel::Compressed { bytes, .. } => bytes.bytes(),
        }
    }


    pub fn serialize(&self, mut buf: impl io::Write) -> io::Result<()> {
        match self {
            CompressionLevel::Uncompressed(bytes) => {
                buf.write_all(&[0])?;
                buf.write_all(bytes)?;
            },
            CompressionLevel::Compressed { level, bytes } => {
                buf.write_all(&[*level])?;
                buf.write_all(bytes.bytes())?;
            },
        }

        Ok(())
    }


    pub fn deserialize<'a>(input: &'a [u8]) -> Result<CompressionLevel<'a>, MultipassCompressionError> {

        let level = *input.get(0).ok_or(MultipassCompressionError::MissingCompressionLevelSpecifier)?;

        if input.len() < 2 {
            return Err(MultipassCompressionError::MissingCompressedData);
        }

        let bytes = &input[1..];

        Ok(
            if level == 0 {
                CompressionLevel::Uncompressed (bytes)
            } else {
                CompressionLevel::Compressed {
                    level,
                    bytes: OwnedOrBorrowedBytes::Borrowed(bytes)
                }
            }
        )
    }

}


fn multipass_compress_string<'a>(text: &'a str, cap: Option<u8>, buf: impl io::Write) -> io::Result<u8> {

    let cap = cap.unwrap_or(u8::MAX);

    let mut res = CompressionLevel::Uncompressed(text.as_bytes());

    while res.level() < cap {

        let compressed = compress::<u8>(res.bytes().iter().cloned());

        println!("Compressing level {}: {} KiB", res.level(), compressed.len() / 1024);

        match res {
            CompressionLevel::Uncompressed(bytes) => {
                if compressed.len() < bytes.len() {
                    res = CompressionLevel::Compressed {
                        level: 1,
                        bytes: OwnedOrBorrowedBytes::Owned(compressed)
                    };
                } else {
                    break;
                }
            },
            CompressionLevel::Compressed { level, ref bytes } => {
                if compressed.len() < bytes.bytes().len() {
                    res = CompressionLevel::Compressed {
                        level: level + 1,
                        bytes: OwnedOrBorrowedBytes::Owned(compressed)
                    };
                } else {
                    break;
                }
            }
        }
    }

    res.serialize(buf)?;

    Ok(res.level())
}


fn multipass_decompress_string<'a>(input: &'a [u8]) -> Result<Cow<'a, str>, MultipassCompressionError> {

    let level = CompressionLevel::deserialize(input)?;

    match level {

        CompressionLevel::Uncompressed(bytes)
            => Ok(
                Cow::Borrowed(
                    str::from_utf8(bytes)
                        .map_err(|e| MultipassCompressionError::InvalidStringEncoding(e))?
                )
            ),

        CompressionLevel::Compressed { level, bytes } => {

            let mut bytes = OwnedOrBorrowedBytes::Borrowed(bytes.bytes());

            for _ in 0..level {

                let decompressed = decompress::<u8>(bytes.bytes()).map_err(|e| MultipassCompressionError::DecompressionError(e))?;
                bytes = OwnedOrBorrowedBytes::Owned(decompressed);

            }

            Ok(
                Cow::Owned(
                    String::from_utf8(bytes.assume_owned().into()).map_err(|e| MultipassCompressionError::InvalidStringEncoding(e.utf8_error()))?
                )
            )
        },
    }
}


fn main() {

    let text = fs::read_to_string("test_data/animal_farm.txt")
        .unwrap_or_else(|e| panic!("Could not read file: {e}"));

    let compression_cap = 3;

    let mut compressed = Vec::new();
    let level = multipass_compress_string(&text, Some(compression_cap), &mut compressed)
        .unwrap_or_else(|e| panic!("Failed to compress data: {}", e));

    let decompressed = multipass_decompress_string(&compressed)
        .unwrap_or_else(|e| panic!("Failed to decompress data: {:?}", e));

    assert_eq!(text, decompressed);

    println!("Original size: {} KiB\nMulticompressed at level {level}: {} KiB\nCompression ratio {:.2}",
        text.len() / 1024, compressed.len() / 1024, text.len() as f64 / compressed.len() as f64);

    // Usually, performing just one compression pass is the best approach in this case.
}
