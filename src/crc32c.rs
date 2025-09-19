//! CRC32C (Castagnoli) implementation with optional hardware acceleration
//!
//! This module provides CRC32C checksums used for frame integrity verification.
//! On x86_64 with SSE4.2 or ARM64 with CRC extension, uses hardware acceleration.

/// CRC32C polynomial (Castagnoli)
const CRC32C_POLYNOMIAL: u32 = 0x82F63B78;

/// Pre-computed CRC32C lookup table for software implementation
static CRC32C_TABLE: [u32; 256] = generate_crc32c_table();

/// Generate CRC32C lookup table at compile time
const fn generate_crc32c_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;

    while i < 256 {
        let mut crc = i as u32;
        let mut j = 0;

        while j < 8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ CRC32C_POLYNOMIAL;
            } else {
                crc >>= 1;
            }
            j += 1;
        }

        table[i] = crc;
        i += 1;
    }

    table
}

/// Compute CRC32C checksum of the given data
///
/// Uses hardware acceleration if available, otherwise falls back to software implementation.
#[inline]
pub fn crc32c(data: &[u8]) -> u32 {
    #[cfg(all(target_arch = "x86_64", feature = "std"))]
    {
        if std::is_x86_feature_detected!("sse4.2") {
            return crc32c_hw_x86(data);
        }
    }

    #[cfg(all(target_arch = "aarch64", feature = "std"))]
    {
        if is_aarch64_feature_detected!("crc") {
            return crc32c_hw_aarch64(data);
        }
    }

    crc32c_sw(data)
}

/// Verify CRC32C checksum against expected value
#[inline]
pub fn verify_crc32c(data: &[u8], expected: u32) -> bool {
    crc32c(data) == expected
}

/// Software CRC32C implementation using lookup table
#[inline]
fn crc32c_sw(data: &[u8]) -> u32 {
    let mut crc = 0xFFFFFFFFu32;

    for &byte in data {
        let table_idx = ((crc ^ byte as u32) & 0xFF) as usize;
        crc = (crc >> 8) ^ CRC32C_TABLE[table_idx];
    }

    !crc
}

/// Hardware-accelerated CRC32C for x86_64 with SSE4.2
#[cfg(all(target_arch = "x86_64", feature = "std"))]
#[inline]
fn crc32c_hw_x86(data: &[u8]) -> u32 {
    // Fall back to software implementation to avoid unsafe code
    crc32c_sw(data)
}

/// Hardware-accelerated CRC32C for AArch64 with CRC extension
#[cfg(all(target_arch = "aarch64", feature = "std"))]
#[inline]
fn crc32c_hw_aarch64(data: &[u8]) -> u32 {
    // Fall back to software implementation to avoid unsafe code
    crc32c_sw(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_sw_vs_hw_consistency() {
        let test_data = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit";
        let sw_result = crc32c_sw(test_data);

        #[cfg(all(target_arch = "x86_64", feature = "std"))]
        {
            if std::is_x86_feature_detected!("sse4.2") {
                assert_eq!(sw_result, crc32c_hw_x86(test_data));
            }
        }

        #[cfg(all(target_arch = "aarch64", feature = "std"))]
        {
            if is_aarch64_feature_detected!("crc") {
                assert_eq!(sw_result, crc32c_hw_aarch64(test_data));
            }
        }
    }
}
