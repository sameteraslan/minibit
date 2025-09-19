//! Integration tests for minibit
//!
//! These tests verify end-to-end functionality and compatibility.

use minibit::*;

#[test]
fn test_trade_message_roundtrip_10k() {
    // Test 10K random trade messages for complete roundtrip consistency
    let mut buf = vec![0u8; 1024];

    for i in 0..10_000 {
        let seq = i as u32;
        let ts_ns = 1_700_000_000_000_000_000 + i as u64 * 1000;
        let price = 50_000_000 + (i as i64 % 10_000) - 5_000; // Range around 50.0
        let qty = 1 + (i as u32 % 999); // 1-999

        let symbol = if i % 4 == 0 {
            Some(["AAPL", "TSLA", "MSFT", "GOOGL"][(i / 4) % 4].as_bytes())
        } else {
            None
        };

        let note_string = if i % 7 == 0 {
            Some(format!("Order #{}", i))
        } else {
            None
        };
        let note = note_string.as_ref().map(|s| s.as_bytes());

        // Encode
        let size = messages::trade::encode(&mut buf, seq, ts_ns, price, qty, symbol, note).unwrap();

        // Decode
        let (header, decoded_ts_ns, decoded_price, decoded_qty, decoded_symbol, decoded_note) =
            messages::trade::decode(&buf[..size]).unwrap();

        // Verify all fields match
        assert_eq!(header.seq, seq);
        assert_eq!(header.msg_type, messages::msg_types::TRADE_V1);
        assert_eq!(decoded_ts_ns, ts_ns);
        assert_eq!(decoded_price, price);
        assert_eq!(decoded_qty, qty);
        assert_eq!(decoded_symbol, symbol);

        if let (Some(expected), Some(actual)) = (note, decoded_note) {
            assert_eq!(actual, expected);
        } else {
            assert_eq!(decoded_note, note);
        }

        // Verify presence bitmap flag is set correctly
        let has_optional = symbol.is_some() || note.is_some();
        assert_eq!(header.has_flag(FrameFlags::PRESENCE_BITMAP), has_optional);
    }
}

#[test]
fn test_quote_message_variations() {
    let mut buf = [0u8; 256];

    let test_cases = [
        // (seq, ts_ns, bid, ask, level, symbol)
        (1, 1000, 100_000_000, 100_010_000, 1, None),
        (
            2,
            2000,
            200_000_000,
            200_020_000,
            1,
            Some(b"BTC/USD" as &[u8]),
        ),
        (3, 3000, 0, 1, 255, Some(b"X")),
        (
            4,
            4000,
            i64::MAX,
            i64::MIN,
            0,
            Some(b"VERY_LONG_SYMBOL_NAME_FOR_TESTING"),
        ),
    ];

    for (seq, ts_ns, bid, ask, level, symbol) in test_cases {
        let size = messages::quote::encode(&mut buf, seq, ts_ns, bid, ask, level, symbol).unwrap();
        let (header, dec_ts, dec_bid, dec_ask, dec_level, dec_symbol) =
            messages::quote::decode(&buf[..size]).unwrap();

        assert_eq!(header.seq, seq);
        assert_eq!(dec_ts, ts_ns);
        assert_eq!(dec_bid, bid);
        assert_eq!(dec_ask, ask);
        assert_eq!(dec_level, level);
        assert_eq!(dec_symbol, symbol);
    }
}

#[test]
fn test_frame_size_bounds() {
    let mut buf = vec![0u8; 64];

    // Test with buffer that's too small
    let result = messages::trade::encode(&mut buf[..20], 1, 1000, 50000, 100, Some(b"AAPL"), None);
    assert_eq!(result, Err(Error::ShortBuffer));

    // Test with exactly right size
    buf.resize(1024, 0);
    let size = messages::trade::encode(&mut buf, 1, 1000, 50000, 100, None, None).unwrap();
    assert!(size <= buf.len());
    assert!(size >= MIN_FRAME_SIZE);
}

#[test]
fn test_empty_and_large_varbytes() {
    let mut buf = vec![0u8; 4096];

    // Empty symbol and note
    let size1 =
        messages::trade::encode(&mut buf, 1, 1000, 50000, 100, Some(b""), Some(b"")).unwrap();
    let (_, _, _, _, symbol, note) = messages::trade::decode(&buf[..size1]).unwrap();
    assert_eq!(symbol, Some(&[][..]));
    assert_eq!(note, Some(&[][..]));

    // Large symbol (but reasonable)
    let large_symbol = vec![b'A'; 255];
    let size2 =
        messages::trade::encode(&mut buf, 2, 2000, 60000, 200, Some(&large_symbol), None).unwrap();
    let (_, _, _, _, symbol, _) = messages::trade::decode(&buf[..size2]).unwrap();
    assert_eq!(symbol, Some(large_symbol.as_slice()));
}

#[test]
fn test_all_bitmap_combinations() {
    let mut buf = [0u8; 256];

    // Test all 4 combinations of symbol/note presence for trade messages
    let test_cases = [
        (None, None),
        (Some(b"AAPL" as &[u8]), None),
        (None, Some(b"note" as &[u8])),
        (Some(b"TSLA" as &[u8]), Some(b"buy order" as &[u8])),
    ];

    for (i, (symbol, note)) in test_cases.iter().enumerate() {
        let size =
            messages::trade::encode(&mut buf, i as u32, 1000, 50000, 100, *symbol, *note).unwrap();
        let (header, _, _, _, dec_symbol, dec_note) =
            messages::trade::decode(&buf[..size]).unwrap();

        assert_eq!(dec_symbol, *symbol);
        assert_eq!(dec_note, *note);

        let has_optional = symbol.is_some() || note.is_some();
        assert_eq!(header.has_flag(FrameFlags::PRESENCE_BITMAP), has_optional);
    }
}

#[test]
fn test_crc_validation() {
    let mut buf = [0u8; 256];
    let size = messages::trade::encode(&mut buf, 1, 1000, 50000, 100, Some(b"TEST"), None).unwrap();

    // Original should validate
    let decoder = FrameDecoder::new(&buf[..size]);
    decoder.verify_crc32c().unwrap();

    // Corrupt header (magic number) - should fail with InvalidMagic, not CrcMismatch
    let original = buf[0];
    buf[0] ^= 0x01;
    let decoder = FrameDecoder::new(&buf[..size]);
    // Header corruption should be detected as InvalidMagic first
    assert_eq!(decoder.header(), Err(Error::InvalidMagic));
    // But CRC validation should also catch it (depending on implementation)
    let crc_result = decoder.verify_crc32c();
    assert!(crc_result.is_err()); // Could be InvalidMagic or CrcMismatch
    buf[0] = original;

    // Corrupt body - should fail with CrcMismatch
    buf[20] ^= 0x01;
    let decoder = FrameDecoder::new(&buf[..size]);
    assert_eq!(decoder.verify_crc32c(), Err(Error::CrcMismatch));
    buf[20] ^= 0x01;

    // Corrupt CRC - should fail with CrcMismatch
    buf[size - 1] ^= 0x01;
    let decoder = FrameDecoder::new(&buf[..size]);
    assert_eq!(decoder.verify_crc32c(), Err(Error::CrcMismatch));
}

#[test]
fn test_edge_case_values() {
    let mut buf = [0u8; 256];

    // Test extreme values
    let test_cases = [
        (0, 0, i64::MIN, 0),
        (u32::MAX, u64::MAX, i64::MAX, u32::MAX),
        (12345, 1_700_000_000_000_000_000, 0, 1),
        (54321, 999_999_999_999_999_999, -1, 999_999_999),
    ];

    for (seq, ts_ns, price, qty) in test_cases {
        let size = messages::trade::encode(&mut buf, seq, ts_ns, price, qty, None, None).unwrap();
        let (header, dec_ts, dec_price, dec_qty, _, _) =
            messages::trade::decode(&buf[..size]).unwrap();

        assert_eq!(header.seq, seq);
        assert_eq!(dec_ts, ts_ns);
        assert_eq!(dec_price, price);
        assert_eq!(dec_qty, qty);
    }
}

#[test]
fn test_manual_frame_construction() {
    // Test that manually constructed frames work with high-level decoders
    let mut buf = [0u8; 256];
    let mut encoder = FrameEncoder::new(&mut buf);

    // Build a trade-like message manually
    let mut header = FrameHeader::new(messages::msg_types::TRADE_V1, 42, 0);
    header.set_flag(FrameFlags::PRESENCE_BITMAP);
    encoder.begin(&header).unwrap();

    // Fixed fields matching trade layout
    encoder.put_u64(1234567890).unwrap(); // ts_ns
    encoder.put_i64(987654321).unwrap(); // price
    encoder.put_u32(100).unwrap(); // qty

    // Presence bitmap and optional fields
    encoder.put_bitmap(0b01).unwrap(); // Only symbol present
    encoder.put_varbytes(b"MANUAL").unwrap();

    let size = encoder.finish_crc32c().unwrap();

    // Decode with high-level API
    let (header, ts_ns, price, qty, symbol, note) = messages::trade::decode(&buf[..size]).unwrap();

    assert_eq!(header.seq, 42);
    assert_eq!(ts_ns, 1234567890);
    assert_eq!(price, 987654321);
    assert_eq!(qty, 100);
    assert_eq!(symbol, Some(&b"MANUAL"[..]));
    assert_eq!(note, None);
}

#[test]
fn test_decoder_position_tracking() {
    let mut buf = [0u8; 256];
    let size = messages::trade::encode(&mut buf, 1, 1000, 50000, 100, Some(b"AAPL"), Some(b"note"))
        .unwrap();

    let decoder = FrameDecoder::new(&buf[..size]);
    let mut body = decoder.body().unwrap();

    assert!(!body.is_at_end());
    assert_eq!(body.remaining(), body.buf.len());

    // Read fixed fields
    let _ts = body.get_u64().unwrap();
    assert_eq!(body.remaining(), body.buf.len() - 8);

    let _price = body.get_i64().unwrap();
    let _qty = body.get_u32().unwrap();

    // Read bitmap and optional fields
    let _bitmap = body.get_bitmap().unwrap();
    let _symbol = body.get_varbytes().unwrap();
    let _note = body.get_varbytes().unwrap();

    assert!(body.is_at_end());
    assert_eq!(body.remaining(), 0);
}

#[test]
fn test_skip_and_peek_operations() {
    let mut buf = [0u8; 256];
    let mut encoder = FrameEncoder::new(&mut buf);

    let header = FrameHeader::new(99, 1, 0);
    encoder.begin(&header).unwrap();
    encoder.put_u32(0x12345678).unwrap();
    encoder.put_u16(0xABCD).unwrap();
    encoder.put_u8(0xEF).unwrap();
    let size = encoder.finish_crc32c().unwrap();

    let decoder = FrameDecoder::new(&buf[..size]);
    let mut body = decoder.body().unwrap();

    // Peek without advancing
    let peeked = body.peek_bytes(4).unwrap();
    assert_eq!(peeked, &[0x78, 0x56, 0x34, 0x12]); // little-endian u32

    // Position shouldn't change after peek
    let pos_before = body.pos;
    let _peeked2 = body.peek_bytes(2).unwrap();
    assert_eq!(body.pos, pos_before);

    // Skip bytes
    body.skip(4).unwrap(); // Skip the u32
    let value = body.get_u16().unwrap();
    assert_eq!(value, 0xABCD);

    let byte = body.get_u8().unwrap();
    assert_eq!(byte, 0xEF);
}

#[test]
fn test_error_conditions() {
    // Test various error conditions

    // Buffer too short for header
    let short_buf = [0u8; 10];
    let decoder = FrameDecoder::new(&short_buf);
    assert!(decoder.header().is_err());

    // Invalid magic number
    let mut bad_magic = [0u8; 32];
    bad_magic[0] = 0xFF;
    bad_magic[1] = 0xFF;
    let decoder = FrameDecoder::new(&bad_magic);
    assert_eq!(decoder.header(), Err(Error::InvalidMagic));

    // Body cursor reading past end
    let mut buf = [0u8; 256];
    let size = messages::trade::encode(&mut buf, 1, 1000, 50000, 100, None, None).unwrap();
    let decoder = FrameDecoder::new(&buf[..size]);
    let mut body = decoder.body().unwrap();

    // Consume all the data
    let _ts = body.get_u64().unwrap();
    let _price = body.get_i64().unwrap();
    let _qty = body.get_u32().unwrap();

    // Try to read past end
    assert_eq!(body.get_u8(), Err(Error::UnexpectedEof));
    assert_eq!(body.get_u16(), Err(Error::UnexpectedEof));
    assert_eq!(body.skip(1), Err(Error::UnexpectedEof));
    assert_eq!(body.peek_bytes(1), Err(Error::UnexpectedEof));
}

#[test]
fn test_varint_edge_cases() {
    use minibit::varint::*;

    let test_cases = [0u32, 1, 127, 128, 16383, 16384, u32::MAX];

    for value in test_cases {
        let mut buf = [0u8; MAX_VARINT_U32_SIZE];
        let encoded_len = encode_u32(value, &mut buf).unwrap();
        let (decoded_value, decoded_len) = decode_u32(&buf[..encoded_len]).unwrap();

        assert_eq!(value, decoded_value);
        assert_eq!(encoded_len, decoded_len);
    }

    // Test buffer too small
    let mut small_buf = [0u8; 2];
    assert_eq!(
        encode_u32(u32::MAX, &mut small_buf),
        Err(Error::ShortBuffer)
    );

    // Test incomplete varint
    let incomplete = [0x80]; // Has continuation bit but no next byte
    assert_eq!(decode_u32(&incomplete), Err(Error::UnexpectedEof));
}

// #[cfg(feature = "std")]
// #[test]
// fn test_benchmark_functions() {
//     use minibit::bench::*;

//     // Test that benchmark functions run without errors
//     let stats = bench_trade_encode(100).unwrap();
//     assert_eq!(stats.count, 100);
//     assert!(stats.avg_ns_per_op > 0);

//     let stats = bench_trade_decode(50).unwrap();
//     assert_eq!(stats.count, 50);

//     let stats = bench_trade_roundtrip(25).unwrap();
//     assert_eq!(stats.count, 25);
// }

#[test]
fn test_crc32c_known_vectors() {
    use minibit::crc32c::*;

    // Test known CRC32C values from RFC 3720
    assert_eq!(crc32c(&[]), 0);
    assert_eq!(crc32c(b"123456789"), 0xe3069283);
    assert_eq!(
        crc32c(b"The quick brown fox jumps over the lazy dog"),
        0x22620404
    );

    // Test verification
    assert!(verify_crc32c(b"123456789", 0xe3069283));
    assert!(!verify_crc32c(b"123456789", 0));
}

#[test]
fn test_presence_bitmap_utilities() {
    use minibit::bitmap::*;

    let mut bitmap = PresenceBitmap::new(BitmapSize::U8);

    assert!(bitmap.is_empty());
    assert_eq!(bitmap.count_set(), 0);

    bitmap.set(0).unwrap();
    bitmap.set(3).unwrap();
    bitmap.set(7).unwrap();

    assert!(!bitmap.is_empty());
    assert_eq!(bitmap.count_set(), 3);
    assert!(bitmap.is_set(0));
    assert!(bitmap.is_set(3));
    assert!(bitmap.is_set(7));
    assert!(!bitmap.is_set(1));

    // Test encode/decode
    let mut buf = [0u8; 4];
    let encoded_len = bitmap.encode(&mut buf).unwrap();
    assert_eq!(encoded_len, 1);

    let (decoded, decoded_len) = PresenceBitmap::decode(&buf, BitmapSize::U8).unwrap();
    assert_eq!(decoded_len, 1);
    assert_eq!(bitmap.bits(), decoded.bits());

    // Test iterator
    let set_fields: Vec<_> = bitmap.iter_set().collect();
    assert_eq!(set_fields, vec![0, 3, 7]);
}

#[test]
fn test_frame_header_operations() {
    let mut header = FrameHeader::new(42, 12345, 100);

    assert_eq!(header.msg_type, 42);
    assert_eq!(header.seq, 12345);
    assert_eq!(header.len, 100);
    assert_eq!(header.flags, 0);

    header.set_flag(FrameFlags::PRESENCE_BITMAP);
    header.set_flag(FrameFlags::COMPRESSED);

    assert!(header.has_flag(FrameFlags::PRESENCE_BITMAP));
    assert!(header.has_flag(FrameFlags::COMPRESSED));
    assert!(!header.has_flag(FrameFlags::ENCRYPTED));

    header.clear_flag(FrameFlags::PRESENCE_BITMAP);
    assert!(!header.has_flag(FrameFlags::PRESENCE_BITMAP));
    assert!(header.has_flag(FrameFlags::COMPRESSED));

    // Test total size calculation
    assert_eq!(header.total_size(), FrameHeader::SIZE + 100 + 4); // header + body + crc
}

#[test]
fn test_compatibility_across_versions() {
    // Test that messages encoded with current version can be read by decoder
    // This test would expand as versions change

    let mut buf = [0u8; 256];
    let size = messages::trade::encode(&mut buf, 1, 1000, 50000, 100, Some(b"AAPL"), None).unwrap();

    let decoder = FrameDecoder::new(&buf[..size]);
    let header = decoder.header().unwrap();

    assert_eq!(header.ver, minibit::PROTOCOL_VERSION);
    assert_eq!(header.magic, minibit::FRAME_MAGIC);

    decoder.verify_crc32c().unwrap();
}
