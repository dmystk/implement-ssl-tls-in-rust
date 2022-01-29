use std::string::String;
use std::vec::Vec;

const BASE64_ENCODE_TABLE: [u8; 64] = generate_encode_table_from(
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz01234567889+/"
);
const BASE64_DECODE_TABLE: [u8; 256] = generate_decode_table_from(
    &BASE64_ENCODE_TABLE
);
const PADDING: u8 = b'=';

const INVALID_VALUE: u8 = 0xFF;
const LOW_6_BITS: u32 = 0x3F;

/// Encode bytes by BASE64.
pub fn encode<T: AsRef<[u8]>>(input: T) -> String {
    let bit6s = into_bit6s(input.as_ref());
    let mut symbols: Vec<_> = bit6s.into_iter()
        .map(|bit6| { BASE64_ENCODE_TABLE[bit6 as usize] })
        .collect();

    while symbols.len() % 4 != 0 {
        symbols.push(PADDING);
    }

    String::from_utf8(symbols).unwrap()
}

/// Decode errors.
#[derive(Debug, PartialEq)]
pub enum DecodeError {
    InvalidLength,
    InvalidByte(usize, u8),
    InvalidLastSymbol(usize, u8),
}

/// Decode bytes by BASE64.
pub fn decode<T: AsRef<[u8]>>(input: T) -> Result<Vec<u8>, DecodeError> {
    let symbols = input.as_ref();
    if symbols.is_empty() {
        return Ok(vec!());
    }
    if let Err(e) = validate_decoding_target(symbols) {
        return Err(e);
    }

    let padding = count_padding(symbols);
    let bit6s: Vec<_> = symbols[..symbols.len()-padding].iter()
        .map(|symbol| { BASE64_DECODE_TABLE[*symbol as usize] })
        .collect();

    let bytes = into_bytes(bit6s.as_ref());

    Ok(bytes)
}

const fn generate_encode_table_from(symbols: &str) -> [u8; 64] {
    let symbols = symbols.as_bytes();
    let mut encode_table: [u8; 64] = [0; 64];
    let mut index = 0;
    while index < 64 {
        encode_table[index] = symbols[index];
        index += 1;
    }
    encode_table
}

const fn generate_decode_table_from(encode_table: &[u8; 64]) -> [u8; 256] {
    let mut decode_table = [INVALID_VALUE; 256];
    let mut index = 0;
    while index < 64 {
        decode_table[encode_table[index] as usize] = index as u8;
        index += 1;
    }
    decode_table
}

/// Convert bytes to 6-bits.
///
/// The capacity (not the number of elements) of the output vector is 4/3 times
/// bigger than the input size. (Rigorously the capacity is the smallest of the
/// numbers that are multiple of 4 and greater than or equal to 4/3 times the
/// input size.) The elements is 0-2 less than the capacity.
///
/// And if the input is not divisible by 6-bits, the lower bits of the last
/// 6-bit is filled by zeros.
fn into_bit6s(bytes: &[u8]) -> Vec<u8> {
    // The actual element is 0-2 less than the capacity, but these are consumed
    // for padding later.
    let bit6s_size = (bytes.len() + 2) / 3 * 4;
    let mut bit6s = Vec::with_capacity(bit6s_size);

    let mut rest = bytes;
    loop {
        match rest.len() {
            0 => {
                return bit6s;
            },
            1 => {
                let (bit6_1, bit6_2) = into_2_bit6(rest[0]);
                bit6s.push(bit6_1);
                bit6s.push(bit6_2);
                return bit6s;
            },
            2 => {
                let (bit6_1, bit6_2, bit6_3) = into_3_bit6(rest[0], rest[1]);
                bit6s.push(bit6_1);
                bit6s.push(bit6_2);
                bit6s.push(bit6_3);
                return bit6s;
            },
            _ => {
                let (bit6_1, bit6_2, bit6_3, bit6_4) = into_4_bit6(rest[0], rest[1], rest[2]);
                bit6s.push(bit6_1);
                bit6s.push(bit6_2);
                bit6s.push(bit6_3);
                bit6s.push(bit6_4);
            },
        }
        rest = &rest[3..];
    }
}

/// Convert 3 bytes to four 6-bits.
fn into_4_bit6(byte1: u8, byte2: u8, byte3: u8) -> (u8, u8, u8, u8) {
    let bit32 = u32::from_be_bytes([byte1, byte2, byte3, 0]);
    ( ((bit32 >> 26) & LOW_6_BITS) as u8
    , ((bit32 >> 20) & LOW_6_BITS) as u8
    , ((bit32 >> 14) & LOW_6_BITS) as u8
    , ((bit32 >>  8) & LOW_6_BITS) as u8
    )
}

/// Convert 2 bytes to three 6-bits.
fn into_3_bit6(byte1: u8, byte2: u8) -> (u8, u8, u8) {
    let bit32 = u32::from_be_bytes([byte1, byte2, 0, 0]);
    ( ((bit32 >> 26) & LOW_6_BITS) as u8
    , ((bit32 >> 20) & LOW_6_BITS) as u8
    , ((bit32 >> 14) & LOW_6_BITS) as u8
    )
}

/// Convert 1 byte to two 6-bits.
fn into_2_bit6(byte: u8) -> (u8, u8) {
    let bit32 = u32::from_be_bytes([byte, 0, 0, 0]);
    ( ((bit32 >> 26) & LOW_6_BITS) as u8
    , ((bit32 >> 20) & LOW_6_BITS) as u8
    )
}

/// Validate whether the input violates BASE64 encoded string.
/// This function check the below:
///     - the length of input is the multiple of 4
///     - the input doesn't contain invalid symbol
///         (only the element of BASE64_ENCODE_TABLE or PADDING are permitted)
///     - PADDING is set only in the last 1 or 2 element
///     - the input doesn't contain invalid last symbol
///         (all of the extra bits in the symbol must be 0 if padding exist)
fn validate_decoding_target(input: &[u8]) -> Result<(), DecodeError> {
    // nothing to do if empty
    if input.is_empty() {
        return Ok(());
    }

    // validate the length of the input bytes
    if input.len() % 4 != 0 {
        return Err(DecodeError::InvalidLength);
    }

    // validate the input contains invalid symbol
    let padding = count_padding(input);
    let invalid_value = input[..input.len()-padding].into_iter()
        .zip(0..input.len())
        .filter(|(value, _)| { BASE64_DECODE_TABLE[**value as usize] == INVALID_VALUE })
        .nth(0);
    if let Some((value, index)) = invalid_value {
        return Err(DecodeError::InvalidByte(index, *value));
    }

    // validate the input contains invalid last symbol
    let last_non_pad_index = input.len() - padding - 1;
    let last_non_pad_elem = input[last_non_pad_index];
    let mask = match padding {
        2 => 0b0000_1111,
        1 => 0b0000_0011,
        _ => 0b0000_0000,
    };
    if BASE64_DECODE_TABLE[last_non_pad_elem as usize] & mask != 0 {
        return Err(DecodeError::InvalidLastSymbol(last_non_pad_index, last_non_pad_elem));
    }

    Ok(())
}

fn count_padding(input: &[u8]) -> usize {
    let (last, last2) = (input[input.len()-1], input[input.len()-2]);
    if last == PADDING && last2 == PADDING {
        2
    } else if last == PADDING {
        1
    } else {
        0
    }
}

/// Convert 6-bits to bytes.
/// This function panics if the remainder dividing 6-bits' length by 4 is 1.
fn into_bytes(bit6s: &[u8]) -> Vec<u8> {
    if bit6s.len() % 4 == 1 {
        panic!("Invalid 6-bits length. {}:{}", file!(), line!());
    }

    // Since decreasing padding correspond to decreasing output size, the
    // output size can be calculated by below:
    //      (input-size) * 3 / 4 - (padding-count)
    // But 6-bits is excluded max 2 padding, so we need to calculate:
    //      roundUp((6-bits-size) * 3 / 4) - (padding-count)
    let padding = (4 - (bit6s.len() % 4)) % 4;
    let bytes_size = ((bit6s.len() + 3) / 4 * 3) - padding;
    let mut bytes = Vec::with_capacity(bytes_size);

    let mut rest = bit6s;
    loop {
        match rest.len() {
            0 => {
                return bytes;
            },
            2 => {
                let byte = into_1_byte(rest[0], rest[1]);
                bytes.push(byte);
                return bytes;
            },
            3 => {
                let (byte1, byte2) = into_2_byte(rest[0], rest[1], rest[2]);
                bytes.push(byte1);
                bytes.push(byte2);
                return bytes;
            },
            _ => {
                let (byte1, byte2, byte3) = into_3_byte(rest[0], rest[1], rest[2], rest[3]);
                bytes.push(byte1);
                bytes.push(byte2);
                bytes.push(byte3);
            },
        }
        rest = &rest[4..];
    }
}

/// Convert four 6-bits to 3 bytes.
fn into_3_byte(bit6_1: u8, bit6_2: u8, bit6_3: u8, bit6_4: u8) -> (u8, u8, u8) {
    ( (bit6_1 << 2) + (bit6_2 >> 4)
    , (bit6_2 << 4) + (bit6_3 >> 2)
    , (bit6_3 << 6) + (bit6_4 >> 0)
    )
}

/// Convert three 6-bits to 2 bytes.
fn into_2_byte(bit6_1: u8, bit6_2: u8, bit6_3: u8) -> (u8, u8) {
    ( (bit6_1 << 2) + (bit6_2 >> 4)
    , (bit6_2 << 4) + (bit6_3 >> 2)
    )
}

/// Convert two 6-bits to 1 byte.
fn into_1_byte(bit6_1: u8, bit6_2: u8) -> u8 {
    (bit6_1 << 2) + (bit6_2 >> 4)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fmt::Display;

    /// Assert the equality of 2 collections.
    fn assert_eq_collection<V: PartialEq + Display, L: AsRef<[V]>, R: AsRef<[V]>>(left: L, right: R) {
        let left = left.as_ref();
        let right = right.as_ref();

        // check length
        if left.len() != right.len() {
            assert!(false, concat!(
                "assertion failed `(left.len() == right.len())`\n",
                "  left: {}\n",
                " right: {}",
            ), left.len(), right.len());
        }

        // check elements
        let index = 0..left.len();
        let result = left.iter()
            .zip(right)
            .zip(index)
            .filter(|((a, b), _)| { a != b })
            .nth(0);
        if let Some(((a, b), i)) = result {
            assert!(false, concat!(
                "assertion failed `(left == right)`\n",
                " index: {}\n",
                "  left: {}\n",
                " right: {}",
            ), i, a, b);
        }
    }

    #[test]
    fn test_encode_9_bytes() {
        let input = [
            0b000000_00, 0b0001_0000, 0b10_000011,  //  0,  1,  2,  3,
            0b011010_01, 0b1011_0111, 0b00_011101,  // 26, 27, 28, 29,
            0b110100_11, 0b0101_1101, 0b10_110111,  // 52, 53, 54, 55,
        ];
        let output = encode(input);
        assert_eq!(output, "ABCDabcd0123");
    }

    #[test]
    fn test_encode_3_bytes() {
        let input = [0b000000_00, 0b0001_0000, 0b10_000011];  // 0, 1, 2, 3,
        let output = encode(input);
        assert_eq!(output, "ABCD");
    }

    #[test]
    fn test_encode_2_bytes() {
        let input = [0b011010_01, 0b1011_1111];  // 26, 27, 60
        let output = encode(input);
        assert_eq!(output, "ab8=");
    }

    #[test]
    fn test_encode_1_byte() {
        let input = [0b110100_11];  // 52, 48
        let output = encode(input);
        assert_eq!(output, "0w==");
    }

    #[test]
    fn test_encode_0_byte() {
        let input = [];
        let output = encode(input);
        assert_eq!(output, "");
    }

    #[test]
    fn test_decode_no_padding_symbol() {
        let input = "ABCDabcd0123";
        let output = decode(input);
        assert!(output.is_ok());

        let output = output.unwrap();
        let expected: [u8; 9] = [
            0b000000_00, 0b0001_0000, 0b10_000011,  //  0,  1,  2,  3,
            0b011010_01, 0b1011_0111, 0b00_011101,  // 26, 27, 28, 29,
            0b110100_11, 0b0101_1101, 0b10_110111,  // 52, 53, 54, 55,
        ];
        assert_eq_collection(output, expected);
    }

    #[test]
    fn test_decode_one_padding_symbol() {
        let input = "abcd010=";
        let output = decode(input);
        assert!(output.is_ok());

        let output = output.unwrap();
        let expected: [u8; 5] = [
            0b011010_01, 0b1011_0111, 0b00_011101,  // 26, 27, 28, 29,
            0b110100_11, 0b0101_1101,               // 52, 53, 52,
        ];
        assert_eq_collection(output, expected);
    }

    #[test]
    fn test_decode_two_padding_symbol() {
        let input = "0w==";
        let output = decode(input);
        assert!(output.is_ok());

        let output = output.unwrap();
        let expected: [u8; 1] = [0b110100_11];  // 52, 48,
        assert_eq_collection(output, expected);
    }

    #[test]
    fn test_decode_empty() {
        let input = "";
        let output = decode(input);
        assert!(output.is_ok());

        let output = output.unwrap();
        let expected = [];
        assert_eq_collection(output, expected);
    }

    #[test]
    fn test_decode_invalid_length() {
        let input = "abcdefg";
        let output = decode(input);
        assert!(output.is_err());

        let output = output.unwrap_err();
        assert_eq!(output, DecodeError::InvalidLength);
    }

    #[test]
    fn test_decode_invalid_byte_irregular_symbol() {
        let input = "a!cd";
        let output = decode(input);
        assert!(output.is_err());

        let output = output.unwrap_err();
        assert_eq!(output, DecodeError::InvalidByte(1, b'!'));
    }

    #[test]
    fn test_decode_invalid_byte_invalid_padding_symbol() {
        let input = "abcdef=h";
        let output = decode(input);
        assert!(output.is_err());

        let output = output.unwrap_err();
        assert_eq!(output, DecodeError::InvalidByte(6, b'='));
    }

    #[test]
    fn test_decode_invalid_last_symbol_one_padding_symbol() {
        let input = "abcdefC=";  // C -> 0b0000_0010
        let output = decode(input);
        assert!(output.is_err());

        let output = output.unwrap_err();
        assert_eq!(output, DecodeError::InvalidLastSymbol(6, b'C'));
    }

    #[test]
    fn test_decode_invalid_last_symbol_two_padding_symbol() {
        let input = "abcdeI==";  // 'I' -> 0b0000_1000
        let output = decode(input);
        assert!(output.is_err());

        let output = output.unwrap_err();
        assert_eq!(output, DecodeError::InvalidLastSymbol(5, b'I'));
    }

    #[test]
    fn test_inverse_property_encoding_to_decoding() {
        let input = [
            0b000000_00, 0b0001_0000, 0b10_000011,  //  0,  1,  2,  3,
            0b011010_01, 0b1011_0111, 0b00_011101,  // 26, 27, 28, 29,
            0b110100_11, 0b0101_1101, 0b10_110111,  // 52, 53, 54, 55,
        ];
        let encoded = encode(input);
        let output = decode(encoded);
        assert!(output.is_ok());
        assert_eq!(output.unwrap(), input);
    }

    #[test]
    fn test_inverse_property_decoding_to_encoding() {
        let input = "ABCDabcd0123";
        let decoded = decode(input);
        assert!(decoded.is_ok());

        let output = encode(decoded.unwrap());
        assert_eq!(output, input);
    }
}
