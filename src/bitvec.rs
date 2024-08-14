

#[derive(Debug, PartialEq)]
pub struct BitVec {

    /// The actual raw bits
    raw_data: Vec<u8>,
    /// How many bits of padding the last byte contains.
    /// Padding bits have no meaning
    last_byte_padding: u8

}

impl BitVec {

    pub fn new() -> Self {
        Self {
            raw_data: Vec::new(),
            last_byte_padding: 0
        }
    }


    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            raw_data: Vec::with_capacity(least_bytes_repr_for_bits(capacity)),
            last_byte_padding: 0
        }
    }


    pub fn len_bits(&self) -> usize {
        self.raw_data.len() * 8 - self.last_byte_padding as usize
    }


    pub fn least_len_bytes(&self) -> usize {
        self.raw_data.len()
    }


    pub fn append_bit(&mut self, bit: bool) {
        
        if self.last_byte_padding == 0 {

            self.raw_data.push((bit as u8) << 7);
            self.last_byte_padding = 7;

        } else {

            // Unwrap is safe because an empty vec would have no padding because it has no bytes
            let last_byte = self.raw_data.last_mut().unwrap();

            *last_byte |= (bit as u8) << self.last_byte_padding - 1;

            self.last_byte_padding -= 1;
        }
    }


    pub fn extend_from_bits(&mut self, bit_view: &BitView) {
        
        if self.last_byte_padding == 0 {

            // The bits are aligned, so this is valid

            self.raw_data.extend_from_slice(bit_view.raw_data);
            self.last_byte_padding = bit_view.last_byte_padding;

        } else {

            // The bits are not aligned

            // TODO: use a more efficient algorithm (complete byte buffering would be good)

            for bit in bit_view.iter_bits() {
                self.append_bit(bit)
            }

        }
    }


    pub fn as_bit_view(&self) -> BitView {
        BitView {
            raw_data: &self.raw_data,
            last_byte_padding: self.last_byte_padding
        }
    }


    pub fn iter_bits(&self) -> BitIterator {
        BitIterator {
            bits: self.as_bit_view(),
            i: 0,
        }
    }


    pub fn as_padded_bytes(&self) -> (&[u8], u8) {
        (
            &self.raw_data,
            self.last_byte_padding
        )
    }


    pub fn from_bool_slice(bools: &[bool]) -> Self {
        
        let mut res = Self::with_capacity(bools.len());

        for &b in bools {
            res.append_bit(b)
        }

        res
    }


    pub fn to_bool_slice(&self) -> Box<[bool]>{
        self.iter_bits()
            .collect()
    }


    pub fn serialize(&self) -> Box<[u8]> {
        
        let mut buf = Vec::with_capacity(1 + self.least_len_bytes());

        buf.push(self.last_byte_padding);

        buf.extend_from_slice(&self.raw_data);

        buf.into_boxed_slice()
    }


    pub fn deserialize(input: &[u8]) -> Self {

        let last_byte_padding = input[0];

        Self {
            raw_data: input[1..].to_vec(),
            last_byte_padding
        }
    }


}


pub const fn least_bytes_repr_for_bits(bit_count: usize) -> usize {
    bit_count / 8 + (bit_count % 8 != 0) as usize
} 


#[derive(Clone)]
pub struct BitView<'a> {

    pub(super) raw_data: &'a [u8],
    pub(super) last_byte_padding: u8

}

impl<'a> BitView<'a> {

    pub fn iter_bits(&'a self) -> BitIterator<'a> {
        BitIterator {
            bits: self.clone(),
            i: 0
        }
    }


    pub const fn from_padded_bytes(bytes: &'a [u8], last_byte_padding: u8) -> BitView<'a> {
        Self {
            raw_data: bytes,
            last_byte_padding
        }
    }


    pub fn to_bool_slice(&self) -> Box<[bool]>{
        self.iter_bits()
            .collect()
    }

}


pub struct BitIterator<'a> {

    bits: BitView<'a>,
    i: usize

}

impl<'a> Iterator for BitIterator<'a> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        
        let byte_i = self.i / 8;

        let byte = *self.bits.raw_data.get(byte_i)?;

        let bit_in_byte_i = (self.i % 8) as u8;

        if byte_i == self.bits.raw_data.len()-1 && bit_in_byte_i >= (8 - self.bits.last_byte_padding) {
            return None;
        }

        self.i += 1;

        Some(
            (byte & (1_u8 << (7 - bit_in_byte_i))) != 0
        )
    }
}


#[cfg(test)]
mod tests {

    use super::*;


    #[test]
    fn check_view_clone() {

        let expected = [true, true, false, true, false, true, false, true, true, true];

        let v = BitVec::from_bool_slice(&expected);
        let view = v.as_bit_view();

        let clone = view.clone();

        assert_eq!(clone.to_bool_slice(), view.to_bool_slice());
        assert_eq!(*clone.to_bool_slice(), expected)
    }


    #[test]
    fn check_view_iter() {

        let expected = [true, true, false, true, false, true, false, true];

        let v = BitVec::from_bool_slice(&expected);

        let view = v.as_bit_view();

        assert_eq!(*view.to_bool_slice(), expected)
    }


    #[test]
    fn check_coherency() {

        let bools = [false, true, false, true, false, true];

        let v = BitVec::from_bool_slice(&bools);

        assert_eq!(bools.len(), v.len_bits());
        assert_eq!(v.least_len_bytes(), 1);

        assert_eq!(*v.to_bool_slice(), bools);

    }


    #[test]
    fn check_extend() {

        let a = [true, false, false, true, false];
        let b = [true, false, false, false, false, true];
        let c = [true, false, false, true, false, true, false, false, false, false, true];

        let mut va = BitVec::from_bool_slice(&a);
        let vb = BitVec::from_bool_slice(&b);

        assert_eq!(*va.to_bool_slice(), a);
        assert_eq!(*vb.to_bool_slice(), b);

        va.extend_from_bits(&vb.as_bit_view());

        assert_eq!(*va.to_bool_slice(), c);
    }


    #[test]
    fn check_serde() {

        let bools = [true, false, false, true, false, true, false, false, false, false, true];

        let v = BitVec::from_bool_slice(&bools);

        let ser = v.serialize();

        let des = BitVec::deserialize(&ser);

        assert_eq!(v, des);
    }

}

