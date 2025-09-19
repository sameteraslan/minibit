//! Basic usage example for MiniBit
//!
//! Run with: cargo run --example basic_usage

use minibit::*;

fn main() -> Result<(), Error> {
    println!("MiniBit Basic Usage Example");
    println!("============================");

    // Example 1: Simple trade message without optional fields
    println!("\n1. Simple Trade Message:");
    {
        let mut buf = [0u8; 256];
        let size = messages::trade::encode(
            &mut buf,
            12345,                     // sequence number
            1_700_000_000_000_000_000, // timestamp (ns since epoch)
            50_000_000,                // price (50.00 as 6-decimal fixed point)
            100,                       // quantity
            None,                      // no symbol
            None,                      // no note
        )?;

        println!("  Encoded {} bytes", size);

        // Decode the message
        let (header, ts_ns, price, qty, symbol, note) = messages::trade::decode(&buf[..size])?;
        println!(
            "  Decoded: seq={}, ts={}, price={}, qty={}, symbol={:?}, note={:?}",
            header.seq,
            ts_ns,
            price,
            qty,
            symbol.map(|s| std::str::from_utf8(s).unwrap_or("?")),
            note.map(|s| std::str::from_utf8(s).unwrap_or("?"))
        );
    }

    // Example 2: Trade message with optional fields
    println!("\n2. Trade Message with Optional Fields:");
    {
        let mut buf = [0u8; 256];
        let size = messages::trade::encode(
            &mut buf,
            67890,
            1_700_000_001_000_000_000,
            -25_500_000, // negative price (short position)
            200,
            Some(b"AAPL"),            // Apple stock symbol
            Some(b"Stop loss order"), // order note
        )?;

        println!("  Encoded {} bytes", size);

        let (header, ts_ns, price, qty, symbol, note) = messages::trade::decode(&buf[..size])?;
        println!(
            "  Decoded: seq={}, ts={}, price={}, qty={}, symbol={:?}, note={:?}",
            header.seq,
            ts_ns,
            price,
            qty,
            symbol.map(|s| std::str::from_utf8(s).unwrap_or("?")),
            note.map(|s| std::str::from_utf8(s).unwrap_or("?"))
        );

        // Show that presence bitmap flag is set
        println!(
            "  Presence bitmap flag: {}",
            header.has_flag(FrameFlags::PRESENCE_BITMAP)
        );
    }

    // Example 3: Quote message
    println!("\n3. Quote Message:");
    {
        let mut buf = [0u8; 256];
        let size = messages::quote::encode(
            &mut buf,
            11111,
            1_700_000_002_000_000_000,
            99_950_000,  // bid
            100_050_000, // ask
            1,           // level 1 quote
            Some(b"BTC/USD"),
        )?;

        println!("  Encoded {} bytes", size);

        let (header, ts_ns, bid, ask, level, symbol) = messages::quote::decode(&buf[..size])?;
        println!(
            "  Decoded: seq={}, ts={}, bid={}, ask={}, level={}, symbol={:?}",
            header.seq,
            ts_ns,
            bid,
            ask,
            level,
            symbol.map(|s| std::str::from_utf8(s).unwrap_or("?"))
        );
    }

    // Example 4: Manual encoding with low-level API
    println!("\n4. Manual Encoding with Low-level API:");
    {
        let mut buf = [0u8; 256];
        let mut encoder = FrameEncoder::new(&mut buf);

        let mut header = FrameHeader::new(999, 42, 0); // Custom message type
        header.set_flag(FrameFlags::PRESENCE_BITMAP);

        encoder.begin(&header)?;

        // Fixed fields
        encoder.put_u64(1_700_000_003_000_000_000)?; // timestamp
        encoder.put_i32(12345)?; // some value

        // Presence bitmap (indicating field 0 and field 2 are present)
        encoder.put_bitmap(0b101)?;

        // Optional field 0
        encoder.put_varbytes(b"Custom message")?;

        // Optional field 2 (skipping field 1)
        encoder.put_u16(0xCAFE)?;

        let size = encoder.finish_crc32c()?;
        println!("  Encoded custom message: {} bytes", size);

        // Manual decoding
        let decoder = FrameDecoder::new(&buf[..size]);
        let decoded_header = decoder.header()?;
        decoder.verify_crc32c()?;

        let mut body = decoder.body()?;
        let timestamp = body.get_u64()?;
        let value = body.get_i32()?;
        let bitmap = body.get_bitmap()?;

        println!(
            "  Decoded custom: msg_type={}, ts={}, value={}, bitmap=0b{:b}",
            decoded_header.msg_type, timestamp, value, bitmap
        );

        if bitmap & 1 != 0 {
            let text = body.get_varbytes()?;
            println!(
                "    Field 0: {:?}",
                std::str::from_utf8(text).unwrap_or("?")
            );
        }

        if bitmap & 4 != 0 {
            let val = body.get_u16()?;
            println!("    Field 2: 0x{:X}", val);
        }
    }

    // Example 5: Performance test
    println!("\n5. Performance Test:");
    {
        const N: usize = 10_000;
        let mut buf = vec![0u8; 1024];

        let start = std::time::Instant::now();

        for i in 0..N {
            let size = messages::trade::encode(
                &mut buf,
                i as u32,
                1_700_000_000_000_000_000 + i as u64,
                50_000_000 + (i as i64 % 1000),
                100 + (i as u32 % 900),
                if i % 3 == 0 { Some(b"AAPL") } else { None },
                if i % 5 == 0 { Some(b"test") } else { None },
            )?;

            let _result = messages::trade::decode(&buf[..size])?;
            std::hint::black_box(_result);
        }

        let elapsed = start.elapsed();
        let ns_per_op = elapsed.as_nanos() as u64 / N as u64;
        let ops_per_sec = N as f64 / elapsed.as_secs_f64();

        println!(
            "  {} roundtrips in {:.2}ms",
            N,
            elapsed.as_secs_f64() * 1000.0
        );
        println!("  {} ns/op, {:.0} ops/sec", ns_per_op, ops_per_sec);
    }

    // Example 6: Frame size analysis
    println!("\n6. Frame Size Analysis:");
    {
        let mut buf = [0u8; 256];

        let sizes = [
            (
                "Minimal trade",
                messages::trade::encode(&mut buf, 1, 1000, 50000, 100, None, None)?,
            ),
            (
                "Trade + symbol",
                messages::trade::encode(&mut buf, 1, 1000, 50000, 100, Some(b"AAPL"), None)?,
            ),
            (
                "Trade + symbol + note",
                messages::trade::encode(
                    &mut buf,
                    1,
                    1000,
                    50000,
                    100,
                    Some(b"AAPL"),
                    Some(b"Buy order"),
                )?,
            ),
            (
                "Quote minimal",
                messages::quote::encode(&mut buf, 1, 1000, 99000, 101000, 1, None)?,
            ),
            (
                "Quote + symbol",
                messages::quote::encode(&mut buf, 1, 1000, 99000, 101000, 1, Some(b"BTC/USD"))?,
            ),
        ];

        for (name, size) in sizes {
            println!("  {}: {} bytes", name, size);
        }

        println!("  Header overhead: {} bytes", FrameHeader::SIZE);
        println!("  CRC32C overhead: 4 bytes");
        println!("  Minimum frame: {} bytes", MIN_FRAME_SIZE);
    }

    println!("\nAll examples completed successfully!");
    Ok(())
}
