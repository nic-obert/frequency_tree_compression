use std::collections::HashMap;
use std::mem;
use std::fs;


#[derive(Debug)]
enum Node {

    Parent { count: usize, left: Box<Node>, right: Box<Node> },
    Leaf { count: usize, value: char },

}

impl Node {

    pub const fn count(&self) -> usize {
        match self {
            Node::Parent { count, .. } |
            Node::Leaf { count, .. }
                => *count
        }
    }


    pub fn insert(&mut self, freq: usize, insert_value: char) {

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
                    left: Box::new(Node::Leaf { count: *count, value: *value }),
                    right: Box::new(Node::Leaf { count: freq, value: insert_value })
                };
            },

        }
    }


    pub fn encode(&self, encoding: Encoding, target: char) -> Option<Encoding> {

        match self {

            Node::Parent { left, right, .. } => {

                if let Some(ret) = left.encode(encoding.step_left(), target) {
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


    pub fn decode(&self, encoding: Encoding, depth: u8) -> Result<char, DecodingError> {
        
        match self {

            Node::Parent { left, right, .. } => {

                if depth == encoding.meaningful {
                    Err(DecodingError::InvalidEncoding)
                } else {
                    [left, right][encoding.step_at(depth) as usize]
                        .decode(encoding, depth + 1)
                }
            },

            Node::Leaf { value, .. } => {

                if depth == encoding.meaningful {
                    Ok(*value)
                } else {
                    Err(DecodingError::InvalidEncoding)
                }
            },
        }
    }

}


#[derive(Debug, Clone, Copy)]
enum DecodingError {

    InvalidEncoding

}


#[repr(u8)]
#[allow(dead_code)]
enum StepDirection {

    Left = 0,
    Right = 1

}

impl StepDirection {

    pub const unsafe fn from_number(n: u64) -> Self {
        mem::transmute(n as u8)
    }

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

    pub const fn new() -> Self {
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
            bits: self.bits | (1_u64 << self.meaningful),
            meaningful: self.meaningful + 1
        }
    }


    pub const fn step_at(&self, depth: u8) -> StepDirection {
        unsafe {
            StepDirection::from_number(self.bits >> depth)
        }
    }

}


#[derive(Debug)]
struct EncodingTree {

    root: Option<Node>

}

impl EncodingTree {

    const fn new() -> Self {
        Self {
            root: None
        }
    }


    fn add_value(&mut self, freq: usize, value: char) {

        if let Some(root) = &mut self.root {
            root.insert(freq, value);
        } else {
            self.root = Some(Node::Leaf { count: freq, value });
        }
    }


    fn encode_value(&self, value: char) -> Encoding {
        self.root.as_ref()
            .unwrap()
            .encode(Encoding::new(), value)
            .unwrap()
        }


    fn decode_value(&self, encoding: Encoding) -> char {
        self.root.as_ref()
            .unwrap()
            .decode(encoding, 0)
            .unwrap()
    }


    pub fn encode(text: &str) -> (Self, Box<[Encoding]>) {

        let mut frequencies = char_frequencies(text);
        sort_frequencies(&mut frequencies);

        let mut encoder = Self::new();

        for &(value, freq) in frequencies.into_iter() {
            encoder.add_value(freq, value);
        }

        let encoded = text.chars()
            .map(|ch| encoder.encode_value(ch))
            .collect::<Box<[Encoding]>>();

        (encoder, encoded)
    }


    pub fn decode(&self, encoding: &[Encoding]) -> Result<String, DecodingError> {
        
        let mut decoded = String::with_capacity(encoding.len());

        for enc in encoding {
            decoded.push(
                self.decode_value(enc.clone())
            )
        }

        Ok(decoded)
    }

}


fn sort_frequencies(frequencies: &mut [(char, usize)]) {
    frequencies.sort_by_key(|pair| pair.1)
}


fn char_frequencies(text: &str) -> Box<[(char, usize)]> {

    let mut frequencies: HashMap<char, usize> = HashMap::new();

    for ch in text.chars() {

        frequencies.entry(ch)
            .and_modify(|counter| *counter += 1)
            .or_insert(1);
    }

    frequencies.drain().collect()
}



fn main() {

    let text = fs::read_to_string("text.txt").unwrap();

    let (encoder, encoded) = EncodingTree::encode(&text);

    // println!("{:#?}\n\n\n{:#?}", encoder, encoded);

    let decoded = encoder.decode(&encoded).unwrap();

    assert_eq!(text, decoded)

}

