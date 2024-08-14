mod bitvec;

use std::{collections::HashMap, mem};
use std::hash::Hash;

use bitvec::{least_bytes_repr_for_bits, BitVec, BitView};


#[derive(Debug)]
enum Node<T> {

    Parent { count: usize, left: Box<Node<T>>, right: Box<Node<T>> },
    Leaf { count: usize, value: T },

}

impl<T> Node<T>
where
    T: PartialEq + Clone
{

    pub const fn count(&self) -> usize {
        match self {
            Node::Parent { count, .. } |
            Node::Leaf { count, .. }
                => *count
        }
    }


    pub fn insert(&mut self, freq: usize, insert_value: T) {

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


    pub fn encode(&self, encoding: Encoding, target: T) -> Option<Encoding> {

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


#[derive(Debug)]
pub struct EncodingTree<T> {

    root: Option<Node<T>>

}

impl<T> EncodingTree<T>
where
    T: PartialEq + Clone + Eq + Hash
{

    const fn new() -> Self {
        Self {
            root: None
        }
    }


    fn add_value(&mut self, freq: usize, value: T) {

        if let Some(root) = &mut self.root {
            root.insert(freq, value);
        } else {
            self.root = Some(Node::Leaf { count: freq, value });
        }
    }


    fn encode_value(&self, value: T) -> Encoding {
        self.root.as_ref()
            .unwrap()
            .encode(Encoding::new_zeroed(), value)
            .unwrap()
        }


    pub fn encode(data: impl Iterator<Item = T> + Clone) -> (Self, BitVec) {

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


    pub fn decode(&self, encoding: &BitView) -> Result<Box<[T]>, DecodingError> {

        let mut decoded = Vec::new();

        let mut node = self.root.as_ref().unwrap();

        for bit in encoding.iter_bits() {

            if let Node::Parent { left, right, .. } = node {

                let next_node = [left, right][bit as usize];
                match next_node.as_ref() {

                    Node::Parent { .. } => {
                        node = next_node;
                    },

                    Node::Leaf { value, .. } => {
                        decoded.push(value.clone());
                        node = self.root.as_ref().unwrap();
                    },
                }

            } else {
                unreachable!()
            }
        }

        if let Node::Leaf { value, .. } = node {
            decoded.push(value.clone());
        } else if node as *const Node<T> != self.root.as_ref().unwrap() as *const Node<T> {
            return Err(DecodingError::InvalidEncoding);
        }

        Ok(decoded.into_boxed_slice())
    }

}


fn sort_frequencies<T>(frequencies: &mut [(T, usize)]) {
    frequencies.sort_by_key(|pair| pair.1)
}


fn value_frequencies<T>(data: impl Iterator<Item = T>) -> Box<[(T, usize)]>
where 
    T: Eq + Hash
{

    let mut frequencies: HashMap<T, usize> = HashMap::new();

    for ch in data {

        frequencies.entry(ch)
            .and_modify(|counter| *counter += 1)
            .or_insert(1);
    }

    frequencies.drain().collect()
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

        dir.map(|entry| {

            let entry = entry.as_ref()
                .unwrap_or_else(|e| panic!("Could not read directory entry {:?}:\n{e}", entry));

            load_text(&entry.path())
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

        let decoded = encoder.decode(&compressed.as_bit_view())
            .unwrap()
            .into_iter()
            .collect::<String>();

        assert_eq!(text, decoded);

    }


    #[test]
    fn check_coherency() {

        for text in get_test_files() {

            let (encoder, compressed) = EncodingTree::encode(text.chars());

            let decoded = encoder.decode(&compressed.as_bit_view())
                .unwrap()
                .into_iter()
                .collect::<String>();

            assert_eq!(text, decoded);
        }
    }

}

