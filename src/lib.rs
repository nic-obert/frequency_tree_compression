#![allow(incomplete_features)]
#![feature(generic_const_exprs)]


use core::slice;
use std::{collections::HashMap, mem};
use std::hash::Hash;

use bitvec_padded::{least_bytes_repr_for_bits, BitVec, BitView};


#[derive(Debug, Clone, Copy)]
pub enum DecompressionError {

    InvalidBitCode,
    InvalidDecodingTree (NodeDeserializationError),
    BitCodeDecodingError (DecodingError)

}


#[repr(u8)]
enum SerialSpecifier {

    Leaf,
    Parent,

}

impl TryFrom<u8> for SerialSpecifier {
    type Error = NodeDeserializationError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value > Self::Parent as u8 {
            Err(NodeDeserializationError::InvalidNodeTypeSpecifier (value))
        } else {
            Ok( unsafe { 
                mem::transmute(value)
            })
        }
    }
}


#[derive(Debug)]
enum Node<U> {

    Parent { count: usize, left: Box<Node<U>>, right: Box<Node<U>> },
    Leaf { count: usize, value: U },

}

impl<U> PartialEq for Node<U>
where
    U: Clone + PartialEq
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {

            (Self::Parent { left: l_left, right: l_right, .. }, Self::Parent { left: r_left, right: r_right, .. }) => l_left == r_left && l_right == r_right,
            
            (Self::Leaf { value: l_value, .. }, Self::Leaf { value: r_value, .. }) => l_value == r_value,
            
            _ => false,
        }
    }
}

impl<U> Node<U>
where
    U: Clone + PartialEq,
    [(); mem::size_of::<U>()]:
{

    pub const fn count(&self) -> usize {
        match self {
            Node::Parent { count, .. } |
            Node::Leaf { count, .. }
                => *count
        }
    }


    pub fn insert(&mut self, freq: usize, insert_value: U) {

        match self {
            
            Node::Parent { count, left, right } => {

                if right.count() > left.count() {
                    left.insert(freq, insert_value);
                } else {
                    right.insert(freq, insert_value);
                }

                *count += freq;
            },

            Node::Leaf { count, value } => {

                *self = Node::Parent {
                    count: *count + freq,
                    left: Box::new(Node::Leaf { count: *count, value: value.clone() }),
                    right: Box::new(Node::Leaf { count: freq, value: insert_value })
                };
            },

        }
    }


    pub fn encode(&self, encoding: Encoding, target: U) -> Option<Encoding> {

        match self {

            Node::Parent { left, right, .. } => {

                if let Some(ret) = left.encode(encoding.step_left(), target.clone()) {
                    Some(ret)
                } else {
                    right.encode(encoding.step_right(), target)
                }
            },

            Node::Leaf { value, .. } => {

                if *value == target {
                    Some(encoding)
                } else {
                    None
                }
            },
        }
    }


    pub fn deserialize(buf: &[u8]) -> Result<(Self, usize), NodeDeserializationError> {

        match SerialSpecifier::try_from(
            *buf.get(0)
                .ok_or(NodeDeserializationError::MissingNodeTypeSpecifier)?
        )? {
            
            SerialSpecifier::Leaf => {

                if buf.len() < 1 + mem::size_of::<U>() {
                    return Err(NodeDeserializationError::MissingNodeUnitData);
                }

                let value = unsafe {
                    &*(&buf[1..1 + mem::size_of::<U>()] as *const _ as *const [u8; mem::size_of::<U>()]) as &[u8; mem::size_of::<U>()]
                };

                Ok((
                    Self::Leaf { 
                        count: 0,
                        value: unsafe {
                            mem::transmute::<&[u8; mem::size_of::<U>()], &U>(value).clone()
                        }
                    },
                    1 + mem::size_of::<U>()
                ))
            },
            
            SerialSpecifier::Parent => {

                let (left, read1) = Self::deserialize(&buf[1..])?;
                let (right, read2) = Self::deserialize(&buf[1 + read1..])?;

                Ok((
                    Self::Parent {
                        count: 0,
                        left: Box::new(left),
                        right: Box::new(right)
                    },
                    1 + read1 + read2
                ))
            },

        }

    }


    pub fn serialize(&self, buf: &mut Vec<u8>) {

        match self {

            Node::Parent { left, right, .. } => {

                buf.push(SerialSpecifier::Parent as u8);

                left.serialize(buf);
                right.serialize(buf);
            },

            Node::Leaf { value, .. } => {

                buf.push(SerialSpecifier::Leaf as u8);

                let bytes = unsafe {
                    slice::from_raw_parts(
                        value as *const U as *const u8,
                        mem::size_of::<U>()
                    )
                };

                buf.extend_from_slice(bytes);
            },
        }
    }

}


#[derive(Debug, Clone, Copy)]
pub enum DecodingError {

    InvalidEncoding

}


/// Encodes a value in the tree
#[derive(Debug, Clone)]
struct Encoding {

    /// The actual encoded value
    bits: u64,

    /// How many bits have meaning
    meaningful: u8

}

impl Encoding {

    /// Create a new `Encoding` object with all bits initialized to zero
    pub const fn new_zeroed() -> Self {
        Self {
            bits: 0,
            meaningful: 0
        }
    }


    pub const fn step_left(&self) -> Self {
        // No operation is necessary because on a well-formed steps argument the uninitialized bits are already 0
        Self {
            bits: self.bits,
            meaningful: self.meaningful + 1
        }
    }


    pub const fn step_right(&self) -> Self {
        Self {
            bits: (self.bits.to_be() | (1_u64 << 63-self.meaningful)).to_be(),
            meaningful: self.meaningful + 1
        }
    }


    #[allow(dead_code)]
    pub fn iter_bits(&self) -> impl Iterator<Item = bool> + '_ {
        (0..self.meaningful)
            .rev()
            .map(|i| (self.bits & (1_u64 << i)) != 0)
    }


    pub fn as_bits<'a>(&'a self) -> BitView<'a> {
        BitView::from_padded_bytes(
            & unsafe { mem::transmute::<&u64, &[u8; 8]>(&self.bits) } [0..least_bytes_repr_for_bits(self.meaningful as usize)],
            (8 - (self.meaningful % 8)) * (self.meaningful % 8 != 0) as u8
        )
    }

}


#[derive(Debug, PartialEq)]
pub struct DecodingTree<U: Clone> {

    root: Node<U>

}

impl<U> DecodingTree<U>
where
    U: Clone + PartialEq,
    [(); mem::size_of::<U>()]:
{

    /// Decode the data unit represented by the given bit code
    pub fn decode(&self, bitcode: &BitView) -> Result<Box<[U]>, DecodingError> {

        let mut decoded = Vec::new();

        let mut node = &self.root;

        for bit in bitcode.iter_bits() {

            if let Node::Parent { left, right, .. } = node {

                let next_node = [left, right][bit as usize];
                match next_node.as_ref() {

                    Node::Parent { .. } => {
                        node = next_node;
                    },

                    Node::Leaf { value, .. } => {
                        decoded.push(value.clone());
                        node = &self.root;
                    },
                }

            } else {
                unreachable!()
            }
        }

        if let Node::Leaf { value, .. } = node {
            decoded.push(value.clone());
        } else if node as *const Node<U> != &self.root as *const Node<U> {
            return Err(DecodingError::InvalidEncoding);
        }

        Ok(decoded.into_boxed_slice())
    }


    pub fn serialize(&self, buf: &mut Vec<u8>) {

        self.root.serialize(buf);
    }


    pub fn deserialize(input: &[u8]) -> Result<(Self, usize), NodeDeserializationError>
    where 
        [(); mem::size_of::<U>()]:
    {

        let (root, read) = Node::deserialize(input)?;

        Ok((
            Self {
                root
            },
            read
        ))
    }

}


#[derive(Debug, Clone, Copy)]
pub enum NodeDeserializationError {

    MissingNodeTypeSpecifier,
    InvalidNodeTypeSpecifier (u8),
    MissingNodeUnitData

}


#[derive(Debug, PartialEq)]
pub struct EncodingTree<U: Clone> {

    /// Root node of the binary tree
    root: Option<Node<U>>,

    /// Total number of leaf nodes in the tree
    leaf_count: usize,

}

impl<U> EncodingTree<U>
where
    U: Clone + Eq + Hash + PartialEq,
    [(); mem::size_of::<U>()]:
{

    const fn new() -> Self {
        Self {
            root: None,
            leaf_count: 0
        }
    }


    pub const fn leaf_node_count(&self) -> usize {
        self.leaf_count
    }


    pub const fn parent_node_count(&self) -> usize {
        self.leaf_count - (self.leaf_count > 1) as usize
    }


    pub const fn total_node_count(&self) -> usize {
        self.leaf_node_count() + self.parent_node_count()
    }


    fn add_value(&mut self, freq: usize, value: U) {

        if let Some(root) = &mut self.root {
            root.insert(freq, value);
        } else {
            self.root = Some(Node::Leaf { count: freq, value });
        }

        self.leaf_count += 1;
    }


    fn encode_value(&self, value: U) -> Encoding {
        self.root.as_ref()
            .unwrap()
            .encode(Encoding::new_zeroed(), value)
            .unwrap()
        }


    pub fn encode(data: impl Iterator<Item = U> + Clone) -> (Self, BitVec) {

        let mut frequencies = value_frequencies(data.clone());
        sort_frequencies(&mut frequencies);

        let mut encoder = Self::new();

        for (value, freq) in frequencies.iter() {
            encoder.add_value(*freq, value.clone());
        }

        let mut encoded = BitVec::new();

        for ch in data {
            encoded.extend_from_bits(
                &encoder.encode_value(ch).as_bits()
            );
        }

        (encoder, encoded)
    }


    /// Convert the `EncodingTree` into a `DecodingTree`
    /// Return `None` if the tree is not initialized
    pub fn into_decoder(self) -> Option<DecodingTree<U>> {
        Some(DecodingTree {
            root: self.root?
        })
    }

}


fn sort_frequencies<T>(frequencies: &mut [(T, usize)]) {
    frequencies.sort_by_key(|pair| pair.1)
}


fn value_frequencies<U, I>(data: I) -> Box<[(U, usize)]>
where 
    U: Eq + Hash,
    I: Iterator<Item = U>
{

    let mut frequencies: HashMap<U, usize> = HashMap::new();

    for unit in data {

        frequencies.entry(unit)
            .and_modify(|counter| *counter += 1)
            .or_insert(1);
    }

    frequencies.drain().collect()
}


pub fn compress<U>(input: impl Iterator<Item = U> + Clone) -> Box<[u8]> 
where 
    U: Clone + Eq + Hash,
    [(); mem::size_of::<U>()]:
{

    let (encoder, bitcode) = EncodingTree::encode(input);

    let tree_repr_size = (1 + mem::size_of::<U>()) * encoder.leaf_node_count() + encoder.parent_node_count();
    let bitcode_repr_size = 1 + bitcode.least_len_bytes();

    let mut res = Vec::with_capacity(tree_repr_size + bitcode_repr_size);

    encoder.into_decoder().unwrap().serialize(&mut res);

    bitcode.serialize(&mut res);

    res.into_boxed_slice()
}


pub fn decompress<U>(input: &[u8]) -> Result<Box<[U]>, DecompressionError>
where 
    U: Clone + PartialEq,
    [(); mem::size_of::<U>()]:
{
    
    let (decoder, read) = DecodingTree::deserialize(input).map_err(|e| DecompressionError::InvalidDecodingTree(e))?;

    let bitcode = BitVec::deserialize(&input[read..]).map_err(|_| DecompressionError::InvalidBitCode)?;

    let decoded = decoder.decode(&bitcode.as_bit_view()).map_err(|e| DecompressionError::BitCodeDecodingError(e))?;

    Ok(decoded)
}


#[cfg(test)]
mod tests {

    use std::{fs, path::Path};

    use rand::{rngs::StdRng, Rng, SeedableRng};

    use super::*;


    const TEST_DATA_DIR: &'static str = "test_data";


    fn load_text<P>(file_path: &P) -> String 
    where
        P: AsRef<Path> + ?Sized
    {
        fs::read_to_string(&file_path).unwrap_or_else(
            |e| panic!("Could not read file {}:\n{}", file_path.as_ref().display(), e))
    }


    fn get_test_files() -> impl Iterator<Item = String> {
        
        let dir = fs::read_dir(TEST_DATA_DIR)
            .unwrap_or_else(|e| panic!("Could not read test boards directory {TEST_DATA_DIR}:\n{e}"));

        dir.flat_map(|entry| {

            let entry = entry.as_ref()
                .unwrap_or_else(|e| panic!("Could not read directory entry {:?}:\n{e}", entry));

            let meta = entry.metadata()
                .unwrap_or_else(|e| panic!("Could not read metadata of entry {:?}:\n{e}", entry));

            if meta.is_file() {
                Some(
                    load_text(&entry.path())
                )
            } else {
                None
            }
        })
    }


    #[test]
    fn check_encoding() {

        let mut rng = StdRng::seed_from_u64(0);

        for _ in 0..100 {

            let mut enc = Encoding::new_zeroed();

            for _ in 0..8 {
                if rng.gen_bool(0.5) {
                    enc = enc.step_left();
                } else {
                    enc = enc.step_right()
                }
            }
    
            let v = enc.as_bits();
            let expected = enc.iter_bits().collect::<Vec<bool>>();
    
            assert_eq!(*v.to_bool_slice(), expected);
        }
    }


    #[test]
    fn small_coherency() {

        let text = "He";

        let (encoder, compressed) = EncodingTree::encode(text.chars());

        let decoded = encoder.into_decoder().unwrap().decode(&compressed.as_bit_view())
            .unwrap()
            .iter()
            .collect::<String>();

        assert_eq!(text, decoded);

    }


    #[test]
    fn check_coherency() {

        for text in get_test_files() {

            let (encoder, compressed) = EncodingTree::encode(text.chars());

            let decoder = encoder.into_decoder().unwrap();

            let decoded = decoder.decode(&compressed.as_bit_view())
                .unwrap()
                .iter()
                .collect::<String>();

            assert_eq!(text, decoded);

            let mut ser = Vec::new();
            decoder.serialize(&mut ser);

            let des = DecodingTree::<char>::deserialize(&ser).unwrap().0;

            assert_eq!(decoder, des);
        }
    }


    #[test]
    fn check_compression_decompression() {

        for text in get_test_files() {

            let compressed = compress(text.chars());

            let decompressed = decompress::<char>(&compressed).unwrap();

            let s: String = decompressed.iter().collect();

            assert_eq!(text, s);
        }
    }

}

