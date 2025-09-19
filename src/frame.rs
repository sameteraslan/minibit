//! Frame header structures and utilities

use crate::error::{Error, Result};
use crate::{FRAME_MAGIC, MAX_FRAME_SIZE, MIN_FRAME_SIZE};

/// Frame header structure (16 bytes, little-endian)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHeader {
    /// Magic number (0xFEED)
    pub magic: u16,
    /// Protocol version
    pub ver: u8,
    /// Frame flags
    pub flags: u8,
    /// Message type identifier
    pub msg_type: u16,
    /// Sequence number
    pub seq: u32,
    /// Body length in bytes
    pub len: u32,
}

/// Frame flags bit definitions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameFlags;

impl FrameFlags {
    /// Presence bitmap is included (bit 0)
    pub const PRESENCE_BITMAP: u8 = 0x01;

    /// Body is LZ4 compressed (bit 1)
    pub const COMPRESSED: u8 = 0x02;

    /// Body is AEAD encrypted (bit 2)
    pub const ENCRYPTED: u8 = 0x04;

    /// Reserved flags mask
    pub const RESERVED: u8 = 0xF8;
}

impl Default for FrameHeader {
    fn default() -> Self {
        Self {
            magic: FRAME_MAGIC,
            ver: 1,
            flags: 0,
            msg_type: 0,
            seq: 0,
            len: 0,
        }
    }
}

impl FrameHeader {
    /// Header size in bytes (fixed)
    pub const SIZE: usize = 16;

    /// Create a new frame header
    #[inline]
    pub fn new(msg_type: u16, seq: u32, len: u32) -> Self {
        Self {
            magic: FRAME_MAGIC,
            ver: 1,
            flags: 0,
            msg_type,
            seq,
            len,
        }
    }

    /// Set a flag bit
    #[inline]
    pub fn set_flag(&mut self, flag: u8) {
        self.flags |= flag;
    }

    /// Clear a flag bit
    #[inline]
    pub fn clear_flag(&mut self, flag: u8) {
        self.flags &= !flag;
    }

    /// Check if a flag bit is set
    #[inline]
    pub fn has_flag(&self, flag: u8) -> bool {
        self.flags & flag != 0
    }

    /// Validate frame header
    #[inline]
    pub fn validate(&self) -> Result<()> {
        if self.magic != FRAME_MAGIC {
            return Err(Error::InvalidMagic);
        }

        if self.ver != 1 {
            return Err(Error::UnsupportedVersion);
        }

        // Check for reserved flags
        if self.flags & FrameFlags::RESERVED != 0 {
            return Err(Error::FlagConflict);
        }

        // Validate frame size bounds
        let total_size = (self.len as usize)
            .checked_add(Self::SIZE)
            .and_then(|s| s.checked_add(4)) // CRC32C
            .ok_or(Error::Overflow)?;

        if total_size < MIN_FRAME_SIZE || total_size > MAX_FRAME_SIZE {
            return Err(Error::Overflow);
        }

        Ok(())
    }

    /// Encode header to bytes (little-endian)
    #[inline]
    pub fn encode(&self, buf: &mut [u8]) -> Result<()> {
        if buf.len() < Self::SIZE {
            return Err(Error::ShortBuffer);
        }

        buf[0..2].copy_from_slice(&self.magic.to_le_bytes());
        buf[2] = self.ver;
        buf[3] = self.flags;
        buf[4..6].copy_from_slice(&self.msg_type.to_le_bytes());
        buf[6..10].copy_from_slice(&self.seq.to_le_bytes());
        buf[10..14].copy_from_slice(&self.len.to_le_bytes());
        buf[14..16].fill(0); // Reserved bytes

        Ok(())
    }

    /// Decode header from bytes (little-endian)
    #[inline]
    pub fn decode(buf: &[u8]) -> Result<Self> {
        if buf.len() < Self::SIZE {
            return Err(Error::UnexpectedEof);
        }

        let header = Self {
            magic: u16::from_le_bytes([buf[0], buf[1]]),
            ver: buf[2],
            flags: buf[3],
            msg_type: u16::from_le_bytes([buf[4], buf[5]]),
            seq: u32::from_le_bytes([buf[6], buf[7], buf[8], buf[9]]),
            len: u32::from_le_bytes([buf[10], buf[11], buf[12], buf[13]]),
            // Skip reserved bytes [14..16]
        };

        header.validate()?;
        Ok(header)
    }

    /// Calculate total frame size including header and CRC
    #[inline]
    pub fn total_size(&self) -> usize {
        Self::SIZE + self.len as usize + 4 // header + body + crc32c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_encode_decode() {
        let header = FrameHeader {
            magic: FRAME_MAGIC,
            ver: 1,
            flags: FrameFlags::PRESENCE_BITMAP | FrameFlags::COMPRESSED,
            msg_type: 42,
            seq: 0x12345678,
            len: 100,
        };

        let mut buf = [0u8; FrameHeader::SIZE];
        header.encode(&mut buf).unwrap();

        let decoded = FrameHeader::decode(&buf).unwrap();
        assert_eq!(header, decoded);
    }

    #[test]
    fn test_header_validation() {
        let mut header = FrameHeader::default();
        assert!(header.validate().is_ok());

        // Invalid magic
        header.magic = 0x1234;
        assert_eq!(header.validate(), Err(Error::InvalidMagic));
        header.magic = FRAME_MAGIC;

        // Invalid version
        header.ver = 99;
        assert_eq!(header.validate(), Err(Error::UnsupportedVersion));
        header.ver = 1;

        // Reserved flags
        header.flags = 0x80;
        assert_eq!(header.validate(), Err(Error::FlagConflict));
        header.flags = 0;

        // Frame too large
        header.len = MAX_FRAME_SIZE as u32;
        assert_eq!(header.validate(), Err(Error::Overflow));
    }

    #[test]
    fn test_flag_operations() {
        let mut header = FrameHeader::default();

        assert!(!header.has_flag(FrameFlags::PRESENCE_BITMAP));

        header.set_flag(FrameFlags::PRESENCE_BITMAP);
        assert!(header.has_flag(FrameFlags::PRESENCE_BITMAP));

        header.clear_flag(FrameFlags::PRESENCE_BITMAP);
        assert!(!header.has_flag(FrameFlags::PRESENCE_BITMAP));
    }
}
