//! High-performance frame encoder with zero-allocation design
//!
//! The encoder writes directly into a user-provided buffer with careful
//! bounds checking and optimal memory layout.

use crate::crc32c;
use crate::error::{Error, Result};
use crate::frame::FrameHeader;
use crate::varint;

/// Frame encoder that writes into a user-provided buffer
pub struct FrameEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    header_start: usize,
    body_start: usize,
}

impl<'a> FrameEncoder<'a> {
    /// Create new encoder with the given buffer
    #[inline]
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self {
            buf,
            pos: 0,
            header_start: 0,
            body_start: 0,
        }
    }

    /// Begin encoding a frame with the given header
    #[inline]
    pub fn begin(&mut self, header: &FrameHeader) -> Result<()> {
        if self.buf.len() < FrameHeader::SIZE {
            return Err(Error::ShortBuffer);
        }

        self.header_start = self.pos;
        header.encode(&mut self.buf[self.pos..self.pos + FrameHeader::SIZE])?;
        self.pos += FrameHeader::SIZE;
        self.body_start = self.pos;

        Ok(())
    }

    /// Write a u8 value
    #[inline]
    pub fn put_u8(&mut self, value: u8) -> Result<()> {
        if self.pos >= self.buf.len() {
            return Err(Error::ShortBuffer);
        }
        self.buf[self.pos] = value;
        self.pos += 1;
        Ok(())
    }

    /// Write a u16 value (little-endian)
    #[inline]
    pub fn put_u16(&mut self, value: u16) -> Result<()> {
        if self.pos + 2 > self.buf.len() {
            return Err(Error::ShortBuffer);
        }
        self.buf[self.pos..self.pos + 2].copy_from_slice(&value.to_le_bytes());
        self.pos += 2;
        Ok(())
    }

    /// Write a u32 value (little-endian)
    #[inline]
    pub fn put_u32(&mut self, value: u32) -> Result<()> {
        if self.pos + 4 > self.buf.len() {
            return Err(Error::ShortBuffer);
        }
        self.buf[self.pos..self.pos + 4].copy_from_slice(&value.to_le_bytes());
        self.pos += 4;
        Ok(())
    }

    /// Write a u64 value (little-endian)
    #[inline]
    pub fn put_u64(&mut self, value: u64) -> Result<()> {
        if self.pos + 8 > self.buf.len() {
            return Err(Error::ShortBuffer);
        }
        self.buf[self.pos..self.pos + 8].copy_from_slice(&value.to_le_bytes());
        self.pos += 8;
        Ok(())
    }

    /// Write an i32 value (little-endian)
    #[inline]
    pub fn put_i32(&mut self, value: i32) -> Result<()> {
        self.put_u32(value as u32)
    }

    /// Write an i64 value (little-endian)
    #[inline]
    pub fn put_i64(&mut self, value: i64) -> Result<()> {
        self.put_u64(value as u64)
    }

    /// Write a presence bitmap (8-bit or 16-bit)
    #[inline]
    pub fn put_bitmap(&mut self, bitmap: u16) -> Result<()> {
        // For simplicity, always use 16-bit format
        self.put_u16(bitmap)
    }

    /// Write variable-length bytes with length prefix
    #[inline]
    pub fn put_varbytes(&mut self, bytes: &[u8]) -> Result<()> {
        // Write length as varint
        let remaining = &mut self.buf[self.pos..];
        let varint_len = varint::encode_u32(bytes.len() as u32, remaining)?;
        self.pos += varint_len;

        // Write bytes
        if self.pos + bytes.len() > self.buf.len() {
            return Err(Error::ShortBuffer);
        }
        self.buf[self.pos..self.pos + bytes.len()].copy_from_slice(bytes);
        self.pos += bytes.len();

        Ok(())
    }

    /// Write raw bytes without length prefix
    #[inline]
    pub fn put_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        if self.pos + bytes.len() > self.buf.len() {
            return Err(Error::ShortBuffer);
        }
        self.buf[self.pos..self.pos + bytes.len()].copy_from_slice(bytes);
        self.pos += bytes.len();
        Ok(())
    }

    /// Write a varint-encoded u32
    #[inline]
    pub fn put_varint_u32(&mut self, value: u32) -> Result<()> {
        let remaining = &mut self.buf[self.pos..];
        let varint_len = varint::encode_u32(value, remaining)?;
        self.pos += varint_len;
        Ok(())
    }

    /// Write a varint-encoded u64  
    #[inline]
    pub fn put_varint_u64(&mut self, value: u64) -> Result<()> {
        let remaining = &mut self.buf[self.pos..];
        let varint_len = varint::encode_u64(value, remaining)?;
        self.pos += varint_len;
        Ok(())
    }

    /// Get current position in buffer
    #[inline]
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Get remaining buffer capacity
    #[inline]
    pub fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    /// Finish encoding and compute CRC32C
    ///
    /// Updates the length field in the header and appends the CRC32C checksum.
    /// Returns the total frame size.
    #[inline]
    pub fn finish_crc32c(&mut self) -> Result<usize> {
        // Calculate body length
        let body_len = self.pos - self.body_start;

        // Update length field in header
        let len_offset = self.header_start + 10; // offset of len field
        self.buf[len_offset..len_offset + 4].copy_from_slice(&(body_len as u32).to_le_bytes());

        // Calculate CRC32C over header + body
        let frame_data = &self.buf[self.header_start..self.pos];
        let crc = crc32c::crc32c(frame_data);

        // Append CRC32C
        if self.pos + 4 > self.buf.len() {
            return Err(Error::ShortBuffer);
        }
        self.buf[self.pos..self.pos + 4].copy_from_slice(&crc.to_le_bytes());
        self.pos += 4;

        Ok(self.pos - self.header_start)
    }

    /// Reset encoder for reuse with the same buffer
    #[inline]
    pub fn reset(&mut self) {
        self.pos = 0;
        self.header_start = 0;
        self.body_start = 0;
    }

    /// Get a slice of the encoded data
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        &self.buf[self.header_start..self.pos]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{decoder::FrameDecoder, FRAME_MAGIC};

    #[test]
    fn test_encoder_basic() {
        let mut buf = [0u8; 256];
        let mut encoder = FrameEncoder::new(&mut buf);

        let header = FrameHeader::new(1, 12345, 0); // len will be updated
        encoder.begin(&header).unwrap();

        encoder.put_u64(1_000_000_000).unwrap(); // timestamp
        encoder.put_i64(50_000_000).unwrap(); // price
        encoder.put_u32(100).unwrap(); // quantity

        let frame_size = encoder.finish_crc32c().unwrap();

        // Verify the frame can be decoded
        let decoder = FrameDecoder::new(&buf[..frame_size]);
        let decoded_header = decoder.header().unwrap();

        assert_eq!(decoded_header.magic, FRAME_MAGIC);
        assert_eq!(decoded_header.msg_type, 1);
        assert_eq!(decoded_header.seq, 12345);
        assert_eq!(decoded_header.len, 20); // 8+8+4 bytes

        decoder.verify_crc32c().unwrap();
    }

    #[test]
    fn test_encoder_varbytes() {
        let mut buf = [0u8; 256];
        let mut encoder = FrameEncoder::new(&mut buf);

        let header = FrameHeader::new(1, 1, 0);
        encoder.begin(&header).unwrap();

        encoder.put_varbytes(b"AAPL").unwrap();
        encoder.put_varbytes(b"").unwrap(); // empty string
        encoder.put_varbytes(b"Hello, World!").unwrap();

        let frame_size = encoder.finish_crc32c().unwrap();

        let decoder = FrameDecoder::new(&buf[..frame_size]);
        decoder.verify_crc32c().unwrap();
    }

    #[test]
    fn test_encoder_bitmap() {
        let mut buf = [0u8; 128];
        let mut encoder = FrameEncoder::new(&mut buf);

        let header = FrameHeader::new(2, 456, 0);
        encoder.begin(&header).unwrap();

        encoder.put_bitmap(0b1010_0001).unwrap(); // fields 0, 5, 7 present
        encoder.put_u32(42).unwrap();

        let frame_size = encoder.finish_crc32c().unwrap();

        let decoder = FrameDecoder::new(&buf[..frame_size]);
        decoder.verify_crc32c().unwrap();

        let decoded_header = decoder.header().unwrap();
        assert_eq!(decoded_header.len, 6); // 2 bytes bitmap + 4 bytes u32
    }

    #[test]
    fn test_encoder_buffer_too_small() {
        let mut buf = [0u8; 10]; // Too small
        let mut encoder = FrameEncoder::new(&mut buf);

        let header = FrameHeader::new(1, 1, 100);
        assert_eq!(encoder.begin(&header), Err(Error::ShortBuffer));
    }

    #[test]
    fn test_encoder_reset() {
        let mut buf = [0u8; 256];
        let mut encoder = FrameEncoder::new(&mut buf);

        // First frame
        let header1 = FrameHeader::new(1, 1, 0);
        encoder.begin(&header1).unwrap();
        encoder.put_u32(123).unwrap();
        let _size1 = encoder.finish_crc32c().unwrap();

        let first_frame = encoder.as_slice().to_vec();

        // Reset and encode second frame
        encoder.reset();
        let header2 = FrameHeader::new(2, 2, 0);
        encoder.begin(&header2).unwrap();
        encoder.put_u64(456).unwrap();
        let _size2 = encoder.finish_crc32c().unwrap();

        // Verify first frame is still valid
        let decoder1 = FrameDecoder::new(&first_frame);
        assert_eq!(decoder1.header().unwrap().msg_type, 1);
    }
}
