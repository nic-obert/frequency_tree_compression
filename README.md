# Frequency tree lossless compression

A lossless compression algorithm implemented in Rust, inspired by Huffman coding. 
It analyzes the specific input data to construct a binary tree encoder that represent the frequency of each unit in the input data, achieving higher compression rates as the input size grows.

- [Frequency tree lossless compression](#frequency-tree-lossless-compression)
- [Basic usage](#basic-usage)
  - [Compressing and decompressing text](#compressing-and-decompressing-text)
- [How it works](#how-it-works)
  - [Generating the encoder](#generating-the-encoder)
  - [Encoding data](#encoding-data)
  - [Serializing the encoded data](#serializing-the-encoded-data)
  - [Deserializing](#deserializing)
  - [Decoding the compressed data](#decoding-the-compressed-data)
- [Some compression theory](#some-compression-theory)
  - [Lossless compression is contraint analysis](#lossless-compression-is-contraint-analysis)
  - [Lossless compression is contingent state reduction](#lossless-compression-is-contingent-state-reduction)
  - [Further optimizations of Huffman coding](#further-optimizations-of-huffman-coding)
- [License](#license)


# Basic usage

## Compressing and decompressing text

Compress a text file

```rust
let original_text = fs::read_to_string("test_data/lorem.txt")
    .unwrap_or_else(|err| panic!("Could not open file: {}", err));

let compressed = compress(original_text.chars());

fs::write("test_data/compressed/lorem.txt.compressed", compressed)
    .unwrap_or_else(|err| panic!("Could not write to file: {}", err));
```

Decompress back into text

```rust
let compressed = fs::read("test_data/compressed/lorem.txt.compressed")
    .unwrap_or_else(|err| panic!("Could not read file {}", err));

let decompressed = decompress::<char>(&compressed)
    .unwrap_or_else(|err| panic!("Could not decompress data {:?}", err));

let decompressed_text: String = decompressed.iter().collect();

assert_eq!(original_text, decompressed_text);
```

# How it works

To achieve high compression rates, this technique analyzes the input data to generate a fine-tuned encoder specific to the input data. This means that a given encoder may not be used to encode or decode data different from that it was generated from.

## Generating the encoder

The first step in generating the encoder is to analyze the input data. The encoder takes as input an iterable of data units (e.g. a string is an iterable of characters) and counts the frequency of each unit in the input data (e.g. how many times each character appears in the string).  
Once the unit frequency table is calculated, it's sorted in descending frequency order. This is so that the more frequent units are prioritized by the encoder in the following step.

After that, an empty binary tree is constructed and each frequency-unit pair is inserted in descending frequency order. The node insertion algorithm keeps track of the total frequency of each branch to maintain the tree well-balanced. This means that a parent node chooses to insert the new node in the child branch with the least total frequency, ensuring the nodes with higher total frequency are placed closer to the root. The total frequency of a node is the sum of the frequency of all leaf nodes below said node.

## Encoding data

Once the encoding tree is constructed, the input data is finally encoded. Each data unit is encoded separately and in the order they appear in the input data. For each data unit the encoder performs a depth-first search of the encoding tree, keeping record of which steps it took, until it finds the leaf node that represents that specific unit. The path toward the leaf node is represented as a series of bits, each representing the branching direction at a specific depth: a value of `0` means that the encoder descended through the left child node and a value of `1` means the encoder descended through the right child node.  
The encoded data unit is the path to its node in the encoding tree, represented as a packed bit sequence.

The whole input data is thus converted to a packed sequence of bits. Because most architectures work with bytes, and not bits, the underlying memory structure that holds the bit sequence is actually a vector of bytes. The last byte of the underlying vector can contain some bits as padding, allowing the encoded data to be byte-aligned. The number of padding bits at the end of the last byte are stored in a single-byte value placed in front of the encoded data so that the decoder knows which bits are meaningful and which are padding.

## Serializing the encoded data

In order to serialize the encoded data in a way that can be later decoded, it's necessary to also include the encoding tree. The final compressed data is thus composed of the serialized encoding tree, the last byte padding specifer, and the encoded padded bit sequence.

## Deserializing

The deserialization is pretty straight-forward. In order, the serialized encoding tree, the last byte padding specifier, and the padded bit sequence are read and correctly deserialized.

## Decoding the compressed data

The encoding tree is used to decode the encoded data. An empty vector is creted to store the output data as it progressibely gets decoded.  
The decoder iterates over the encoded bit sequence, stopping at the final padding or when the end of the sequence is reached. For every bit in the sequence, starting from the root node of the encoding tree, the decoder takes a step left or right, depending on the bit's value: `0` is left, `1` is right. The decoder descends the tree in this fashion until it reaches a leaf node. The value of said leaf node is the original data unit value, so it gets pushed onto the output vector. When a leaf node is reached, the decoder resets the search back form the root node and continues with the next bit.

# Some compression theory

Lossless compression is based on assumptions about the data being compressed. Normally, if we were to consider all possible states of the input data as valid, lossless compression would be impossible because we would be losing data. However, if the data format being compressed has some constraints, we can apply them to improve the compression rate.  

## Lossless compression is contraint analysis

Consider the following example: a text-based file format only allows the character `a` to be followed by the character `b` so that every `a` is guaranteed to be part of the character group `ab`. Knowing that, what can we do to compress an input file?  
Assuming the file format uses the utf-8 character encoding, the character `a` is represented as the 1-byte value `97` and `b` as the 1-byte value `98`. The character sequence `ab` is thus represented as the 2-byte sequence `[97, 98]`. By applying the aforementioned file format constraints, we can assume that every `a` (byte `97`) is followed by `b` (byte `98`). So, the group `ab` (bytes `[97, 98]`) can be replaced with just one `a` (byte `97`), halving the bytes needed to represent the `ab` group.

When decompressing the file, whenever the byte `97` is encountered, it's replaced with the character sequence `ab`.

## Lossless compression is contingent state reduction

Consider the following example: the ISO 8859-1 ASCII encoding represents each character as a 1-byte value so that there are 256 possible characters. How many unique characters does your text file contain, though? I doubt most plain text files contain the character `Æ` (byte `198`)? The same goes with most control characters.  
By analyzing the specific input data to be compressed, we can reduce the number of possible states a data unit can assume, thus reducing the number of bits needed to represent each unique state.  
You can then use a Huffman coding-like technique such as the one presented in this repository to compress the plain text file so that every character needs less than one byte to be stored (exactly one byte in the worst case scenario).

## Further optimizations of Huffman coding

Knowledge is power, they say. It's definitely true when it comes to lossless compression. Let's say we want to compress a utf-8-encoded text file that contains a piece of English literature, "Animal Farm" of George Orwell, for instance. We know that written languages are composed of graphemes and some graphemes usually occur in groups. Within a language, there are many grapheme groups that are regularly observed, while other grapheme groups are rarely found or simply don't exist. Take, for instance, the grapheme groups `wq`, `qw`, `kp`, `yy`. They are not present in the novel. Meanwhile, the grapheme group `as` is found 1187 times in the novel.  
This is to say that written languages have a set of grapheme groups that are very common and many that are less frequent, and some that are not used at all. This is a perfect application for a Huffman coding-like technique that uses two characters as data unit instead of just one character and correctly adds padding for odd character counts.

# License

This repository and all the files contained within are published under the [MIT license](LICENSE).

