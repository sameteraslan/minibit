//! High-performance zero-copy frame decoder
//!
//! The decoder operates on borrowed slices and provides zero-copy access
//! to frame contents where possible.

use crate::crc32c;
use crate::error::{Error, Result};
use crate::frame::FrameHeader;
use crate::varint;

/// Zero-copy frame decoder
#[derive(Debug)]
pub struct FrameDecoder<'a> {
    buf: &'a [u8],
}

/// Cursor for reading body content with position tracking
#[derive(Debug)]
pub struct BodyCursor<'a> {
    /// Buffer containing the body data
    pub buf: &'a [u8],
    /// Current read position
    pub pos: usize,
}

impl<'a> FrameDecoder<'a> {
    /// Create new decoder for the given buffer
    #[inline]
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf }
    }

    /// Decode and validate the frame header
    #[inline]
    pub fn header(&self) -> Result<FrameHeader> {
        FrameHeader::decode(self.buf)
    }

    /// Verify CRC32C checksum of the frame
    #[inline]
    pub fn verify_crc32c(&self) -> Result<()> {
        // Try to get header, but if it fails due to corruption,
        // we still want to attempt CRC validation if we have enough bytes
        let header_result = self.header();

        let total_size = match header_result {
            Ok(header) => header.total_size(),
            Err(_) => {
                // If header is corrupted, try to extract length from raw bytes
                // to still perform CRC check (header might be corrupted but CRC can still detect it)
                if self.buf.len() < FrameHeader::SIZE {
                    return Err(Error::UnexpectedEof);
                }

                // Extract length field from raw bytes (at offset 10)
                let len_bytes = [self.buf[10], self.buf[11], self.buf[12], self.buf[13]];
                let len = u32::from_le_bytes(len_bytes) as usize;

                // Validate reasonable bounds
                if len > crate::MAX_FRAME_SIZE - FrameHeader::SIZE - 4 {
                    return header_result.map(|_| ()); // Return original header error
                }

                FrameHeader::SIZE + len + 4 // header + body + crc
            }
        };

        if self.buf.len() < total_size {
            return Err(Error::UnexpectedEof);
        }

        // CRC covers header + body (not including the CRC itself)
        let crc_start = total_size - 4;
        let frame_data = &self.buf[..crc_start];
        let stored_crc = u32::from_le_bytes([
            self.buf[crc_start],
            self.buf[crc_start + 1],
            self.buf[crc_start + 2],
            self.buf[crc_start + 3],
        ]);

        if !crc32c::verify_crc32c(frame_data, stored_crc) {
            return Err(Error::CrcMismatch);
        }

        // If header was invalid but CRC was valid, return the header error
        // This maintains the priority: CRC mismatch is more specific than invalid header
        match header_result {
            Err(e) => Err(e),
            Ok(_) => Ok(()),
        }
    }

    /// Get body cursor for reading body content
    #[inline]
    pub fn body(&self) -> Result<BodyCursor<'a>> {
        let header = self.header()?;
        let body_start = FrameHeader::SIZE;
        let body_end = body_start + header.len as usize;

        if self.buf.len() < body_end {
            return Err(Error::UnexpectedEof);
        }

        Ok(BodyCursor {
            buf: &self.buf[body_start..body_end],
            pos: 0,
        })
    }

    /// Get entire frame buffer including header and CRC
    #[inline]
    pub fn frame_buffer(&self) -> Result<&'a [u8]> {
        let header = self.header()?;
        let total_size = header.total_size();

        if self.buf.len() < total_size {
            return Err(Error::UnexpectedEof);
        }

        Ok(&self.buf[..total_size])
    }
}

impl<'a> BodyCursor<'a> {
    /// Get remaining bytes in cursor
    #[inline]
    pub fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    /// Check if cursor is at end
    #[inline]
    pub fn is_at_end(&self) -> bool {
        self.pos >= self.buf.len()
    }

    /// Skip bytes in the cursor
    #[inline]
    pub fn skip(&mut self, n: usize) -> Result<()> {
        if self.pos + n > self.buf.len() {
            return Err(Error::UnexpectedEof);
        }
        self.pos += n;
        Ok(())
    }

    /// Read a u8 value
    #[inline]
    pub fn get_u8(&mut self) -> Result<u8> {
        if self.pos >= self.buf.len() {
            return Err(Error::UnexpectedEof);
        }
        let value = self.buf[self.pos];
        self.pos += 1;
        Ok(value)
    }

    /// Read a u16 value (little-endian)
    #[inline]
    pub fn get_u16(&mut self) -> Result<u16> {
        if self.pos + 2 > self.buf.len() {
            return Err(Error::UnexpectedEof);
        }
        let bytes = [self.buf[self.pos], self.buf[self.pos + 1]];
        let value = u16::from_le_bytes(bytes);
        self.pos += 2;
        Ok(value)
    }

    /// Read a u32 value (little-endian)
    #[inline]
    pub fn get_u32(&mut self) -> Result<u32> {
        if self.pos + 4 > self.buf.len() {
            return Err(Error::UnexpectedEof);
        }
        let bytes = [
            self.buf[self.pos],
            self.buf[self.pos + 1],
            self.buf[self.pos + 2],
            self.buf[self.pos + 3],
        ];
        let value = u32::from_le_bytes(bytes);
        self.pos += 4;
        Ok(value)
    }

    /// Read a u64 value (little-endian)
    #[inline]
    pub fn get_u64(&mut self) -> Result<u64> {
        if self.pos + 8 > self.buf.len() {
            return Err(Error::UnexpectedEof);
        }
        let bytes = [
            self.buf[self.pos],
            self.buf[self.pos + 1],
            self.buf[self.pos + 2],
            self.buf[self.pos + 3],
            self.buf[self.pos + 4],
            self.buf[self.pos + 5],
            self.buf[self.pos + 6],
            self.buf[self.pos + 7],
        ];
        let value = u64::from_le_bytes(bytes);
        self.pos += 8;
        Ok(value)
    }

    /// Read an i32 value (little-endian)
    #[inline]
    pub fn get_i32(&mut self) -> Result<i32> {
        Ok(self.get_u32()? as i32)
    }

    /// Read an i64 value (little-endian)
    #[inline]
    pub fn get_i64(&mut self) -> Result<i64> {
        Ok(self.get_u64()? as i64)
    }

    /// Read a presence bitmap (16-bit)
    #[inline]
    pub fn get_bitmap(&mut self) -> Result<u16> {
        self.get_u16()
    }

    /// Read variable-length bytes with length prefix
    ///
    /// Returns a zero-copy slice into the original buffer
    #[inline]
    pub fn get_varbytes(&mut self) -> Result<&'a [u8]> {
        let remaining_buf = &self.buf[self.pos..];
        let (len, varint_size) = varint::decode_u32(remaining_buf)?;
        self.pos += varint_size;

        let len = len as usize;
        if self.pos + len > self.buf.len() {
            return Err(Error::UnexpectedEof);
        }

        let bytes = &self.buf[self.pos..self.pos + len];
        self.pos += len;
        Ok(bytes)
    }

    /// Read raw bytes without length prefix
    #[inline]
    pub fn get_bytes(&mut self, len: usize) -> Result<&'a [u8]> {
        if self.pos + len > self.buf.len() {
            return Err(Error::UnexpectedEof);
        }
        let bytes = &self.buf[self.pos..self.pos + len];
        self.pos += len;
        Ok(bytes)
    }

    /// Read a varint-encoded u32
    #[inline]
    pub fn get_varint_u32(&mut self) -> Result<u32> {
        let remaining_buf = &self.buf[self.pos..];
        let (value, varint_size) = varint::decode_u32(remaining_buf)?;
        self.pos += varint_size;
        Ok(value)
    }

    /// Read a varint-encoded u64
    #[inline]
    pub fn get_varint_u64(&mut self) -> Result<u64> {
        let remaining_buf = &self.buf[self.pos..];
        let (value, varint_size) = varint::decode_u64(remaining_buf)?;
        self.pos += varint_size;
        Ok(value)
    }

    /// Peek at bytes without advancing cursor
    #[inline]
    pub fn peek_bytes(&self, len: usize) -> Result<&'a [u8]> {
        if self.pos + len > self.buf.len() {
            return Err(Error::UnexpectedEof);
        }
        Ok(&self.buf[self.pos..self.pos + len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoder::FrameEncoder;
    use crate::frame::FrameFlags;
    use crate::FRAME_MAGIC;

    #[test]
    fn test_decoder_basic() {
        let mut buf = [0u8; 256];
        let mut encoder = FrameEncoder::new(&mut buf);

        let header = FrameHeader::new(1, 12345, 0);
        encoder.begin(&header).unwrap();

        encoder.put_u64(1_000_000_000).unwrap(); // timestamp
        encoder.put_i64(50_000_000).unwrap(); // price
        encoder.put_u32(100).unwrap(); // quantity

        let frame_size = encoder.finish_crc32c().unwrap();

        // Test decoder
        let decoder = FrameDecoder::new(&buf[..frame_size]);
        let decoded_header = decoder.header().unwrap();

        assert_eq!(decoded_header.magic, FRAME_MAGIC);
        assert_eq!(decoded_header.msg_type, 1);
        assert_eq!(decoded_header.seq, 12345);

        decoder.verify_crc32c().unwrap();

        let mut body = decoder.body().unwrap();
        assert_eq!(body.get_u64().unwrap(), 1_000_000_000);
        assert_eq!(body.get_i64().unwrap(), 50_000_000);
        assert_eq!(body.get_u32().unwrap(), 100);
        assert!(body.is_at_end());
    }

    #[test]
    fn test_decoder_varbytes() {
        let mut buf = [0u8; 256];
        let mut encoder = FrameEncoder::new(&mut buf);

        let header = FrameHeader::new(2, 1, 0);
        encoder.begin(&header).unwrap();

        encoder.put_varbytes(b"AAPL").unwrap();
        encoder.put_varbytes(b"").unwrap();
        encoder.put_varbytes(b"Hello, World!").unwrap();

        let frame_size = encoder.finish_crc32c().unwrap();

        let decoder = FrameDecoder::new(&buf[..frame_size]);
        decoder.verify_crc32c().unwrap();

        let mut body = decoder.body().unwrap();
        assert_eq!(body.get_varbytes().unwrap(), b"AAPL");
        assert_eq!(body.get_varbytes().unwrap(), b"");
        assert_eq!(body.get_varbytes().unwrap(), b"Hello, World!");
        assert!(body.is_at_end());
    }

    #[test]
    fn test_decoder_bitmap() {
        let mut buf = [0u8; 128];
        let mut encoder = FrameEncoder::new(&mut buf);

        let mut header = FrameHeader::new(3, 456, 0);
        header.set_flag(FrameFlags::PRESENCE_BITMAP);
        encoder.begin(&header).unwrap();

        encoder.put_bitmap(0b1010_0001).unwrap();
        encoder.put_u32(42).unwrap();
        encoder.put_u64(999).unwrap();

        let frame_size = encoder.finish_crc32c().unwrap();

        let decoder = FrameDecoder::new(&buf[..frame_size]);
        let decoded_header = decoder.header().unwrap();
        assert!(decoded_header.has_flag(FrameFlags::PRESENCE_BITMAP));

        decoder.verify_crc32c().unwrap();

        let mut body = decoder.body().unwrap();
        let bitmap = body.get_bitmap().unwrap();
        assert_eq!(bitmap, 0b1010_0001);
        assert_eq!(body.get_u32().unwrap(), 42);
        assert_eq!(body.get_u64().unwrap(), 999);
    }

    #[test]
    fn test_decoder_error_cases() {
        // Test with invalid frame
        let buf = [0u8; 10];
        let decoder = FrameDecoder::new(&buf);
        assert_eq!(decoder.header(), Err(Error::UnexpectedEof));

        // Test with bad magic
        let mut buf = [0u8; 32];
        buf[0] = 0xFF;
        buf[1] = 0xFF;
        let decoder = FrameDecoder::new(&buf);
        assert_eq!(decoder.header(), Err(Error::InvalidMagic));
    }

    #[test]
    fn test_decoder_crc_mismatch() {
        let mut buf = [0u8; 256];
        let mut encoder = FrameEncoder::new(&mut buf);

        let header = FrameHeader::new(1, 1, 0);
        encoder.begin(&header).unwrap();
        encoder.put_u32(123).unwrap();
        let frame_size = encoder.finish_crc32c().unwrap();

        // Corrupt the CRC
        buf[frame_size - 1] ^= 0x01;

        let decoder = FrameDecoder::new(&buf[..frame_size]);
        assert_eq!(decoder.verify_crc32c(), Err(Error::CrcMismatch));
    }

    #[test]
    fn test_body_cursor_operations() {
        let data = [1, 2, 3, 4, 5, 6, 7, 8];
        let mut cursor = BodyCursor { buf: &data, pos: 0 };

        assert_eq!(cursor.remaining(), 8);
        assert!(!cursor.is_at_end());

        assert_eq!(cursor.get_u16().unwrap(), 0x0201); // little-endian
        assert_eq!(cursor.remaining(), 6);

        cursor.skip(2).unwrap();
        assert_eq!(cursor.remaining(), 4);

        let peeked = cursor.peek_bytes(2).unwrap();
        assert_eq!(peeked, &[5, 6]);
        assert_eq!(cursor.remaining(), 4); // peek doesn't advance

        assert_eq!(cursor.get_u32().unwrap(), 0x08070605); // little-endian
        assert!(cursor.is_at_end());
    }
}
