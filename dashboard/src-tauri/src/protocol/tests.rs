//! Checks this implementation against the shared golden vectors
//! (`protocol/vectors/`), the same files the firmware's native tests use.

use super::*;

fn vector(name: &str) -> Vec<u8> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../protocol/vectors/").to_string() + name;
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    text.lines()
        .filter(|line| !line.starts_with('#'))
        .flat_map(|line| line.split_whitespace())
        .map(|byte| u8::from_str_radix(byte, 16).expect("hex byte"))
        .collect()
}

#[test]
fn crc32_check_value() {
    assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    assert_eq!(crc32(b""), 0);
}

#[test]
fn cobs_reference_examples() {
    // Cheshire & Baker examples.
    assert_eq!(cobs::encode(&[0x00]), vec![0x01, 0x01]);
    assert_eq!(cobs::encode(&[0x11, 0x22, 0x00, 0x33]), vec![0x03, 0x11, 0x22, 0x02, 0x33]);
    assert_eq!(cobs::encode(&[0x11, 0x00, 0x00, 0x00]), vec![0x02, 0x11, 0x01, 0x01, 0x01]);
    assert_eq!(cobs::decode(&[0x03, 0x11, 0x22, 0x02, 0x33]).unwrap(), vec![0x11, 0x22, 0x00, 0x33]);

    // 254 non-zero bytes: full block, and decoders accept it with or without
    // the trailing empty block.
    let run: Vec<u8> = (1..=254).collect();
    let mut with_trailer = vec![0xFFu8];
    with_trailer.extend_from_slice(&run);
    with_trailer.push(0x01);
    assert_eq!(cobs::encode(&run), with_trailer);
    assert_eq!(cobs::decode(&with_trailer).unwrap(), run);
    assert_eq!(cobs::decode(&with_trailer[..with_trailer.len() - 1]).unwrap(), run);

    assert_eq!(cobs::decode(&[0x03, 0x11, 0x00]), Err(ProtocolError::BadCobs));
    assert_eq!(cobs::decode(&[0x05, 0x11, 0x22]), Err(ProtocolError::BadCobs));
}

#[test]
fn cobs_round_trip_never_emits_zero() {
    let mut lcg: u32 = 12345;
    for size in (0..=1200).step_by(7) {
        let data: Vec<u8> = (0..size)
            .map(|_| {
                lcg = lcg.wrapping_mul(1664525).wrapping_add(1013904223);
                let v = (lcg >> 24) as u8;
                if v < 40 { 0 } else { v }
            })
            .collect();
        let encoded = cobs::encode(&data);
        assert!(!encoded.contains(&0), "size {size}");
        assert_eq!(cobs::decode(&encoded).unwrap(), data, "size {size}");
    }
}

#[test]
fn host_hello_request_matches_vector() {
    let expected = vector("hello_request.hex");
    let (header, payload) = parse(&expected).unwrap();
    assert_eq!(header.msg_type, MsgType::Hello as u8);
    assert_eq!(header.sequence, 7);
    assert_eq!(payload, hello_request());
    assert_eq!(encode(MsgType::Hello, 7, 0, &hello_request()), expected);
}

#[test]
fn device_hello_decodes() {
    let msg = vector("hello_info.hex");
    let (header, payload) = parse(&msg).unwrap();
    assert_eq!(header.sequence, 42);
    assert_eq!(header.time_us, 123_456_789);
    let hello = parse_hello(payload).unwrap();
    assert_eq!(hello.schema, SCHEMA_VERSION);
    assert_eq!(hello.device_state, 4); // READY
    assert_eq!(hello.boot_id, 0xA1B2_C3D4);
    assert_eq!(hello.mac_string(), "44:B1:76:AF:FB:7C");
    assert_eq!((hello.who_foot, hello.who_shank), (0x70, 0x70));
    assert!(!hello.haptics_fitted());
    assert_eq!(hello.capability_names(), vec!["psram_ring"]);
    assert_eq!((hello.oldest_seq, hello.last_seq), (1, 42));
    assert_eq!(hello.fw_version, "0.1.0+test");
}

#[test]
fn device_status_decodes() {
    let msg = vector("status.hex");
    let (_, payload) = parse(&msg).unwrap();
    let status = parse_status(payload).unwrap();
    assert_eq!(DEVICE_STATES[status.device_state as usize], "ready");
    assert_eq!(fault_names(status.faults), vec!["shank_frozen", "acquisition_stalled"]);
    assert_eq!(status.frame_index, 123_456);
    assert_eq!(status.shank_repeated, 17);
    assert_eq!(status.ap_rssi_dbm, -47);
    assert_eq!(status.heap_free_min, 201_000);
    assert_eq!(status.stack_free_wifi, 4200);
    assert_eq!(status.calibration_state, 2, "ready");
    assert_eq!(status.calibration_samples, 500);
    assert_eq!(status.calibration_reject, 0);
    assert_eq!(parse_status(&payload[..10]), Err(ProtocolError::BadPayload("STATUS")));
}

#[test]
fn raw_batch_decodes_signed_counts() {
    let msg = vector("raw_batch.hex");
    let (header, payload) = parse(&msg).unwrap();
    assert!(MsgType::from_u8(header.msg_type).unwrap().is_durable());
    let frames = parse_raw_batch(payload).unwrap();
    assert_eq!(frames.len(), 2);

    assert_eq!(frames[0].timestamp_us, 1_000_000);
    assert_eq!(frames[0].frame_index, 100);
    // Doc 09 declares these fields u16; they are signed (DEC-007).
    assert_eq!(frames[0].foot, [8192, -8192, 32767, -32768, 1, -1]);
    assert_eq!(frames[0].shank, [0, 16, -16, 655, -655, 32767]);
    assert_eq!(
        names_for(frames[0].status, &RAW_STATUS_NAMES),
        vec!["foot_read_fail", "foot_gyro_saturated", "shank_accel_saturated"]
    );

    assert_eq!(frames[1].frame_index, 101);
    assert_eq!(frames[1].foot, [-1, -2, -3, -4, -5, -6]);
    assert_eq!(frames[1].q_shank, [0, -32767, 12345, -12345]);
    assert_eq!(names_for(frames[1].status, &RAW_STATUS_NAMES), vec!["shank_repeated"]);

    assert!(parse_raw_batch(&payload[..payload.len() - 1]).is_err());
}

#[test]
fn device_error_decodes() {
    let msg = vector("error.hex");
    let (_, payload) = parse(&msg).unwrap();
    let error = parse_device_error(payload).unwrap();
    assert_eq!(error.cmd_seq, 9);
    assert_eq!(error.cmd_type, MsgType::SessionStart as u8);
    assert_eq!(error.code, 3);
    assert_eq!(error.detail, "SESSION_START not supported");
    assert!(error.to_string().contains("NotSupported"));
}

#[test]
fn config_response_hash_matches_section() {
    let payload = vector("config_response.hex");
    let config = parse_config(&payload).unwrap();
    assert_eq!(config.format, 1);
    assert_eq!(config.section, vector("config_section.hex"));
    // The device reports this hash in HELLO; recompute it the same way.
    use sha2::{Digest, Sha256};
    assert_eq!(config.sha256[..], Sha256::digest(&config.section)[..]);
}

#[test]
fn config_section_decodes_every_documented_field() {
    let section = vector("config_section.hex");
    let config = config::parse_section(&section).unwrap();

    // Contract values (CONFIG_V1.json).
    assert_eq!(config.imu.foot_address, 0x68);
    assert_eq!(config.imu.shank_address, 0x69);
    assert_eq!(config.imu.sample_hz, 100);
    assert_eq!(config.imu.accel_range_g, 4);
    assert_eq!(config.imu.gyro_range_dps, 500);
    assert_eq!(config.imu.accel_lsb_per_g, 8192.0);
    assert_eq!(config.imu.gyro_lsb_per_dps, 65.5);
    assert_eq!(config.pins.motors, [1, 2, 4, 9, 43, 44]);
    assert_eq!(config.mahony_kp, 2.0);
    assert_eq!(config.gait.min_cycle_s, 0.45);
    assert_eq!(config.zupt.gyro_threshold_dps, 25.0);
    assert_eq!(config.error.weights[0], 0.25);
    assert_eq!(config.ap_ip, [192, 168, 4, 1]);
    assert_eq!(config.ws_port, 8080);
    assert_eq!(config.reference_min_cycles, 30);

    // As-built values (docs/hardware.md, DEC-006/009).
    assert!(!config.haptics.fitted);
    assert_eq!(config.imu.foot_mount, [[1, 0, 0], [0, 1, 0], [0, 0, 1]]);
    assert_eq!(config.imu.shank_mount, [[0, 0, -1], [0, 1, 0], [1, 0, 0]]);

    // Trailing bytes mean a layout this build does not know.
    let mut longer = section.clone();
    longer.push(0);
    assert!(config::parse_section(&longer).is_err());
    assert!(config::parse_section(&section[..section.len() - 1]).is_err());
}

#[test]
fn mount_maps_produce_anatomical_units() {
    let config = config::parse_section(&vector("config_section.hex")).unwrap();
    // One g on chip +Z with the foot mount (identity) is one g anatomical up.
    let (accel, gyro) = config.foot_anatomical(&[0, 0, 8192, 0, 0, 655]);
    assert_eq!(accel, [0.0, 0.0, 1.0]);
    assert_eq!(gyro, [0.0, 0.0, 10.0]);

    // Shank: anatomical X = -chipZ, Y = +chipY, Z = +chipX, measured on the leg
    // (TEST-027, PROB-002).
    let (accel, _) = config.shank_anatomical(&[8192, 0, 0, 0, 0, 0]);
    assert_eq!(accel, [0.0, 0.0, 1.0], "gravity on chip +X reads as anatomical +Z");
    let (accel, _) = config.shank_anatomical(&[0, 8192, 0, 0, 0, 0]);
    assert_eq!(accel, [0.0, 1.0, 0.0], "chip +Y is anatomical +Y (medial)");
    let (accel, _) = config.shank_anatomical(&[0, 0, 8192, 0, 0, 0]);
    assert_eq!(accel, [-1.0, 0.0, 0.0], "chip +Z points posteriorly");
}

#[test]
fn backfill_round_trip() {
    let request = vector("backfill_request.hex");
    let (_, payload) = parse(&request).unwrap();
    assert_eq!(payload, backfill_request(100, 250));

    let msg = vector("backfill_data.hex");
    let (_, payload) = parse(&msg).unwrap();
    let chunk = parse_backfill_data(payload).unwrap();
    assert_eq!((chunk.cmd_seq, chunk.first_seq, chunk.last_seq, chunk.more), (11, 3, 4, false));
    assert_eq!(chunk.messages.len(), 2);
    // The inner messages are intact originals, decodable on their own.
    assert_eq!(chunk.messages[0], vector("raw_batch.hex"));
    let sequences: Vec<u32> =
        chunk.messages.iter().map(|m| parse(m).unwrap().0.sequence).collect();
    assert_eq!(sequences, vec![3, 4]);
}

#[test]
fn usb_frames_match_vectors() {
    for (frame_name, msg_name) in [
        ("usb_frame_hello_request.hex", "hello_request.hex"),
        ("usb_frame_long.hex", "long_message.hex"),
    ] {
        let msg = vector(msg_name);
        assert_eq!(usb_frame(&msg), vector(frame_name), "{frame_name}");
    }
}

#[test]
fn usb_decoder_skips_boot_text_and_corruption() {
    let hello_frame = vector("usb_frame_hello_request.hex");
    let long_frame = vector("usb_frame_long.hex");
    let mut corrupted = hello_frame.clone();
    corrupted[5] ^= 0x40; // breaks the CRC

    let mut stream = b"ESP-ROM:esp32s3-20210327\r\nentry 0x403c98d0\r\n".to_vec();
    stream.extend_from_slice(&hello_frame);
    stream.extend_from_slice(&corrupted);
    stream.extend_from_slice(&long_frame);

    let mut decoder = UsbDecoder::new();
    // Feed in odd-sized chunks: framing must not depend on read boundaries.
    let mut messages = Vec::new();
    for chunk in stream.chunks(7) {
        messages.extend(decoder.feed(chunk));
    }
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0], vector("hello_request.hex"));
    assert_eq!(messages[1], vector("long_message.hex"));
    assert_eq!(decoder.rejected(), 2); // ROM text + corrupted frame
}

#[test]
fn parse_rejects_bad_headers() {
    let mut msg = vector("hello_request.hex");
    assert_eq!(parse(&msg[..10]), Err(ProtocolError::TooShort));
    assert!(matches!(
        parse(&msg[..msg.len() - 1]),
        Err(ProtocolError::LengthMismatch { declared: 2, actual: 1 })
    ));
    msg[0] = 2;
    assert_eq!(parse(&msg), Err(ProtocolError::Version(2)));
}

#[test]
fn session_start_matches_the_vector() {
    let msg = vector("session_start.hex");
    let (header, payload) = parse(&msg).unwrap();
    assert_eq!(header.msg_type, MsgType::SessionStart as u8);
    assert_eq!(payload, session_start(SESSION_KIND_CALIBRATION, 5000));
}

#[test]
fn calibration_record_decodes() {
    let msg = vector("calibration_record.hex");
    let (header, payload) = parse(&msg).unwrap();
    assert_eq!(header.msg_type, MsgType::SessionStop as u8);
    let record = parse_calibration(payload).unwrap();
    assert!(record.usable());
    assert_eq!(record.samples, 500);
    assert_eq!(record.foot.gyro_bias_dps, [1.5, -0.5, 0.25]);
    assert_eq!(record.shank.gyro_bias_dps, [-0.75, 0.5, 0.125]);
    // The foot board sits tilted on the instep; the shank board nearly upright.
    assert_eq!(record.foot.tilt_deg, 33.0);
    assert_eq!(record.shank.tilt_deg, 1.3);
    assert_eq!(parse_calibration(&payload[..10]), Err(ProtocolError::BadPayload("SESSION_STOP")));
}

#[test]
fn rejection_bits_name_every_reason() {
    assert!(calibration_rejections(0).is_empty());
    assert_eq!(calibration_rejections(0b1010), vec!["the sensor moved", "gravity was not upward"]);
    assert_eq!(calibration_rejections(0b1111).len(), 4);
}
