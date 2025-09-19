//! Variable-length integer encoding (LEB128)
//!
//! This module provides fast varint encoding/decoding using the LEB128 format.
//! Used for length prefixes in variable-length fields.

use crate::error::{Error, Result};

/// Maximum bytes needed for a u32 varint (5 bytes)
pub const MAX_VARINT_U32_SIZE: usize = 5;

/// Maximum bytes needed for a u64 varint (10 bytes)  
pub const MAX_VARINT_U64_SIZE: usize = 10;

/// Encode a u32 as varint into the given buffer
///
/// Returns the number of bytes written, or Error::ShortBuffer if insufficient space.
#[inline]
pub fn encode_u32(value: u32, buf: &mut [u8]) -> Result<usize> {
    let mut value = value;
    let mut pos = 0;

    loop {
        if pos >= buf.len() {
            return Err(Error::ShortBuffer);
        }

        if value < 0x80 {
            buf[pos] = value as u8;
            return Ok(pos + 1);
        }

        buf[pos] = (value as u8) | 0x80;
        value >>= 7;
        pos += 1;
    }
}

/// Decode a u32 varint from the given buffer
///
/// Returns (value, bytes_consumed) or an error.
#[inline]
pub fn decode_u32(buf: &[u8]) -> Result<(u32, usize)> {
    let mut result = 0u32;
    let mut shift = 0;
    let mut pos = 0;

    loop {
        if pos >= buf.len() {
            return Err(Error::UnexpectedEof);
        }

        if shift >= 32 {
            return Err(Error::Overflow);
        }

        let byte = buf[pos];
        pos += 1;

        result |= ((byte & 0x7F) as u32) << shift;

        if byte & 0x80 == 0 {
            return Ok((result, pos));
        }

        shift += 7;
    }
}

/// Encode a u64 as varint into the given buffer
///
/// Returns the number of bytes written, or Error::ShortBuffer if insufficient space.
#[inline]
pub fn encode_u64(value: u64, buf: &mut [u8]) -> Result<usize> {
    let mut value = value;
    let mut pos = 0;

    loop {
        if pos >= buf.len() {
            return Err(Error::ShortBuffer);
        }

        if value < 0x80 {
            buf[pos] = value as u8;
            return Ok(pos + 1);
        }

        buf[pos] = (value as u8) | 0x80;
        value >>= 7;
        pos += 1;
    }
}

/// Decode a u64 varint from the given buffer
///
/// Returns (value, bytes_consumed) or an error.
#[inline]
pub fn decode_u64(buf: &[u8]) -> Result<(u64, usize)> {
    let mut result = 0u64;
    let mut shift = 0;
    let mut pos = 0;

    loop {
        if pos >= buf.len() {
            return Err(Error::UnexpectedEof);
        }

        if shift >= 64 {
            return Err(Error::Overflow);
        }

        let byte = buf[pos];
        pos += 1;

        result |= ((byte & 0x7F) as u64) << shift;

        if byte & 0x80 == 0 {
            return Ok((result, pos));
        }

        shift += 7;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u32_roundtrip() {
        let test_values = [0, 1, 127, 128, 16383, 16384, u32::MAX];

        for &val in &test_values {
            let mut buf = [0u8; MAX_VARINT_U32_SIZE];
            let encoded_len = encode_u32(val, &mut buf).unwrap();
            let (decoded_val, decoded_len) = decode_u32(&buf[..encoded_len]).unwrap();

            assert_eq!(val, decoded_val);
            assert_eq!(encoded_len, decoded_len);
        }
    }

    #[test]
    fn test_u64_roundtrip() {
        let test_values = [0, 1, 127, 128, 16383, 16384, u32::MAX as u64, u64::MAX];

        for &val in &test_values {
            let mut buf = [0u8; MAX_VARINT_U64_SIZE];
            let encoded_len = encode_u64(val, &mut buf).unwrap();
            let (decoded_val, decoded_len) = decode_u64(&buf[..encoded_len]).unwrap();

            assert_eq!(val, decoded_val);
            assert_eq!(encoded_len, decoded_len);
        }
    }

    #[test]
    fn test_buffer_too_small() {
        let mut buf = [0u8; 2]; // Too small for large values
        assert_eq!(encode_u32(u32::MAX, &mut buf), Err(Error::ShortBuffer));
    }

    #[test]
    fn test_unexpected_eof() {
        let buf = [0x80]; // Incomplete varint
        assert_eq!(decode_u32(&buf), Err(Error::UnexpectedEof));
    }
}
