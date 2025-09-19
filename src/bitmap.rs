//! Presence bitmap utilities for optional fields
//!
//! Supports 8-bit and 16-bit presence bitmaps for tracking which optional
//! fields are present in a message.

use crate::error::{Error, Result};

/// Maximum fields supported by 8-bit bitmap
pub const BITMAP_8_MAX_FIELDS: usize = 8;

/// Maximum fields supported by 16-bit bitmap  
pub const BITMAP_16_MAX_FIELDS: usize = 16;

/// Presence bitmap helper
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresenceBitmap {
    bits: u16,
    size: BitmapSize,
}

/// Bitmap size variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitmapSize {
    /// 8-bit bitmap (1 byte)
    U8,
    /// 16-bit bitmap (2 bytes)
    U16,
}

impl BitmapSize {
    /// Size in bytes
    #[inline]
    pub const fn bytes(&self) -> usize {
        match self {
            BitmapSize::U8 => 1,
            BitmapSize::U16 => 2,
        }
    }

    /// Maximum field index
    #[inline]
    pub const fn max_fields(&self) -> usize {
        match self {
            BitmapSize::U8 => BITMAP_8_MAX_FIELDS,
            BitmapSize::U16 => BITMAP_16_MAX_FIELDS,
        }
    }
}

impl PresenceBitmap {
    /// Create new empty bitmap
    #[inline]
    pub const fn new(size: BitmapSize) -> Self {
        Self { bits: 0, size }
    }

    /// Create from raw bits value
    #[inline]
    pub const fn from_bits(bits: u16, size: BitmapSize) -> Self {
        Self { bits, size }
    }

    /// Get raw bits value
    #[inline]
    pub const fn bits(&self) -> u16 {
        self.bits
    }

    /// Get bitmap size
    #[inline]
    pub const fn size(&self) -> BitmapSize {
        self.size
    }

    /// Set a field as present
    #[inline]
    pub fn set(&mut self, field_idx: usize) -> Result<()> {
        if field_idx >= self.size.max_fields() {
            return Err(Error::Overflow);
        }
        self.bits |= 1 << field_idx;
        Ok(())
    }

    /// Clear a field (mark as absent)
    #[inline]
    pub fn clear(&mut self, field_idx: usize) -> Result<()> {
        if field_idx >= self.size.max_fields() {
            return Err(Error::Overflow);
        }
        self.bits &= !(1 << field_idx);
        Ok(())
    }

    /// Check if a field is present
    #[inline]
    pub fn is_set(&self, field_idx: usize) -> bool {
        if field_idx >= self.size.max_fields() {
            return false;
        }
        (self.bits >> field_idx) & 1 != 0
    }

    /// Count number of set bits
    #[inline]
    pub fn count_set(&self) -> usize {
        self.bits.count_ones() as usize
    }

    /// Check if bitmap is empty (no fields set)
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.bits == 0
    }

    /// Encode bitmap to buffer
    #[inline]
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize> {
        match self.size {
            BitmapSize::U8 => {
                if buf.is_empty() {
                    return Err(Error::ShortBuffer);
                }
                buf[0] = self.bits as u8;
                Ok(1)
            }
            BitmapSize::U16 => {
                if buf.len() < 2 {
                    return Err(Error::ShortBuffer);
                }
                buf[0..2].copy_from_slice(&self.bits.to_le_bytes());
                Ok(2)
            }
        }
    }

    /// Decode bitmap from buffer
    #[inline]
    pub fn decode(buf: &[u8], size: BitmapSize) -> Result<(Self, usize)> {
        match size {
            BitmapSize::U8 => {
                if buf.is_empty() {
                    return Err(Error::UnexpectedEof);
                }
                let bitmap = Self::from_bits(buf[0] as u16, size);
                Ok((bitmap, 1))
            }
            BitmapSize::U16 => {
                if buf.len() < 2 {
                    return Err(Error::UnexpectedEof);
                }
                let bits = u16::from_le_bytes([buf[0], buf[1]]);
                let bitmap = Self::from_bits(bits, size);
                Ok((bitmap, 2))
            }
        }
    }

    /// Iterator over set field indices
    #[inline]
    pub fn iter_set(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.size.max_fields()).filter(move |&i| self.is_set(i))
    }
}

/// Builder for constructing presence bitmaps
#[derive(Debug)]
pub struct BitmapBuilder {
    bitmap: PresenceBitmap,
}

impl BitmapBuilder {
    /// Create new builder
    #[inline]
    pub const fn new(size: BitmapSize) -> Self {
        Self {
            bitmap: PresenceBitmap::new(size),
        }
    }

    /// Set a field as present
    #[inline]
    pub fn with_field(mut self, field_idx: usize) -> Result<Self> {
        self.bitmap.set(field_idx)?;
        Ok(self)
    }

    /// Build final bitmap
    #[inline]
    pub const fn build(self) -> PresenceBitmap {
        self.bitmap
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitmap_basic_operations() {
        let mut bitmap = PresenceBitmap::new(BitmapSize::U8);

        assert!(bitmap.is_empty());
        assert_eq!(bitmap.count_set(), 0);
        assert!(!bitmap.is_set(0));

        bitmap.set(0).unwrap();
        bitmap.set(3).unwrap();
        bitmap.set(7).unwrap();

        assert!(!bitmap.is_empty());
        assert_eq!(bitmap.count_set(), 3);
        assert!(bitmap.is_set(0));
        assert!(bitmap.is_set(3));
        assert!(bitmap.is_set(7));
        assert!(!bitmap.is_set(1));

        bitmap.clear(3).unwrap();
        assert!(!bitmap.is_set(3));
        assert_eq!(bitmap.count_set(), 2);
    }

    #[test]
    fn test_bitmap_bounds() {
        let mut bitmap8 = PresenceBitmap::new(BitmapSize::U8);
        let mut bitmap16 = PresenceBitmap::new(BitmapSize::U16);

        // Valid indices
        assert!(bitmap8.set(7).is_ok());
        assert!(bitmap16.set(15).is_ok());

        // Invalid indices
        assert_eq!(bitmap8.set(8), Err(Error::Overflow));
        assert_eq!(bitmap16.set(16), Err(Error::Overflow));
    }

    #[test]
    fn test_bitmap_encode_decode() {
        let mut bitmap = PresenceBitmap::new(BitmapSize::U16);
        bitmap.set(0).unwrap();
        bitmap.set(8).unwrap();
        bitmap.set(15).unwrap();

        let mut buf = [0u8; 4];
        let encoded_len = bitmap.encode(&mut buf).unwrap();
        assert_eq!(encoded_len, 2);

        let (decoded, decoded_len) = PresenceBitmap::decode(&buf, BitmapSize::U16).unwrap();
        assert_eq!(decoded_len, 2);
        assert_eq!(bitmap.bits(), decoded.bits());

        assert!(decoded.is_set(0));
        assert!(decoded.is_set(8));
        assert!(decoded.is_set(15));
        assert!(!decoded.is_set(1));
    }

    #[test]
    fn test_bitmap_iterator() {
        let mut bitmap = PresenceBitmap::new(BitmapSize::U8);
        bitmap.set(1).unwrap();
        bitmap.set(3).unwrap();
        bitmap.set(6).unwrap();

        let set_fields: std::vec::Vec<usize> = bitmap.iter_set().collect();
        assert_eq!(set_fields, std::vec![1, 3, 6]);
    }

    #[test]
    fn test_bitmap_builder() {
        let bitmap = BitmapBuilder::new(BitmapSize::U8)
            .with_field(0)
            .unwrap()
            .with_field(2)
            .unwrap()
            .with_field(7)
            .unwrap()
            .build();

        assert!(bitmap.is_set(0));
        assert!(bitmap.is_set(2));
        assert!(bitmap.is_set(7));
        assert!(!bitmap.is_set(1));
        assert_eq!(bitmap.count_set(), 3);
    }
}
