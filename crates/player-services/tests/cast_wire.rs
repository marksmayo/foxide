use player_services::cast_wire::{Message, Payload};
use std::io::Cursor;

// Independent proto2 fixture: fields 1=0, 2=s, 3=r, 4=urn:x, 5=0, 6={}.
const JSON_FRAME: &[u8] = b"\0\0\0\x15\x08\x00\x12\x01s\x1a\x01r\x22\x05urn:x\x28\x00\x32\x02{}";

#[test]
fn exchanges_a_standard_cast_envelope_without_changing_its_routing() {
    let message = Message::read(&mut Cursor::new(JSON_FRAME)).unwrap();
    assert_eq!(message.source(), "s");
    assert_eq!(message.destination(), "r");
    assert_eq!(message.namespace(), "urn:x");
    assert_eq!(message.payload(), &Payload::Json(serde_json::json!({})));
    let outgoing = Message::json("s", "r", "urn:x", serde_json::json!({})).unwrap();
    let mut wire = Vec::new();
    outgoing.write(&mut wire).unwrap();
    assert_eq!(wire, JSON_FRAME);
}

#[test]
fn carries_binary_device_auth_challenges_without_utf8_conversion() {
    let outgoing = Message::binary("s", "r", "urn:x", vec![0, 255, 0]).unwrap();
    let expected = b"\0\0\0\x16\x08\0\x12\x01s\x1a\x01r\x22\x05urn:x\x28\x01\x3a\x03\0\xff\0";
    let mut wire = Vec::new();
    outgoing.write(&mut wire).unwrap();
    assert_eq!(wire, expected);
    assert_eq!(
        Message::read(&mut Cursor::new(expected)).unwrap().payload(),
        &Payload::Binary(vec![0, 255, 0])
    );
}

fn framed(body: &[u8]) -> Vec<u8> {
    let mut bytes = (body.len() as u32).to_be_bytes().to_vec();
    bytes.extend_from_slice(body);
    bytes
}
fn fails(body: &[u8]) {
    assert!(
        Message::read(&mut Cursor::new(framed(body))).is_err(),
        "{body:?}"
    );
}

#[test]
fn rejects_declared_oversize_before_reading_any_body() {
    use std::io::{self, Read};
    struct HeaderOnly {
        header: Cursor<[u8; 4]>,
    }
    impl Read for HeaderOnly {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            assert!(self.header.position() < 4, "oversize body was read");
            self.header.read(bytes)
        }
    }
    for length in [0u32, 65537, u32::MAX] {
        assert!(
            Message::read(&mut HeaderOnly {
                header: Cursor::new(length.to_be_bytes())
            })
            .is_err()
        );
    }
}

#[test]
fn handles_fragmented_reads_and_leaves_the_next_frame_untouched() {
    use std::io::{self, Read};
    struct Fragmented(Cursor<Vec<u8>>);
    impl Read for Fragmented {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            let length = output.len().min(1);
            self.0.read(&mut output[..length])
        }
    }
    let mut stream = Fragmented(Cursor::new([JSON_FRAME, JSON_FRAME].concat()));
    for _ in 0..2 {
        assert_eq!(Message::read(&mut stream).unwrap().source(), "s");
    }
    assert_eq!(stream.0.position(), (JSON_FRAME.len() * 2) as u64);
}

#[test]
fn skips_forward_compatible_unknown_fields_of_each_supported_wire_type() {
    let mut body = JSON_FRAME[4..].to_vec();
    body.extend_from_slice(&[
        0x40, 0xac, 2, 0x49, 0, 0, 0, 0, 0, 0, 0, 0, 0x52, 3, 1, 2, 3, 0x5d, 0, 0, 0, 0,
    ]);
    let message = Message::read(&mut Cursor::new(framed(&body))).unwrap();
    assert_eq!(message.payload(), &Payload::Json(serde_json::json!({})));
}

#[test]
fn refuses_truncated_frames_and_every_missing_required_field() {
    for end in 0..JSON_FRAME.len() {
        assert!(
            Message::read(&mut Cursor::new(&JSON_FRAME[..end])).is_err(),
            "end={end}"
        );
    }
    let body = &JSON_FRAME[4..];
    for (start, end) in [(0, 2), (2, 5), (5, 8), (8, 15), (15, 17), (17, 21)] {
        fails(&[&body[..start], &body[end..]].concat());
    }
}

#[test]
fn rejects_ambiguous_payloads_duplicate_routes_and_unsupported_versions() {
    let body = &JSON_FRAME[4..];
    for extra in [&[0x12, 1, b'z'][..], &[0x3a, 0][..]] {
        fails(&[body, extra].concat());
    }
    for (offset, value) in [(1, 1), (16, 2), (0, 9), (2, 16), (7, 0xff), (20, b'x')] {
        let mut changed = body.to_vec();
        changed[offset] = value;
        fails(&changed);
    }
    for extra in [
        &[0][..],
        &[0x40, 0x80][..],
        &[0x52, 0xff, 0xff, 0xff, 0xff, 0x7f][..],
        &[0x43][..],
        &[0x46][..],
        &[0x49, 1][..],
        &[0x5d, 1][..],
    ] {
        fails(&[body, extra].concat());
    }
    fails(&[body, &[0x40], &[0xff; 10]].concat());
    fails(&[body, &[0x80, 0x80, 0x80, 0x80, 0x10, 0]].concat());
}

#[test]
fn validates_routes_and_bounds_outbound_data_before_writing_a_header() {
    for route in ["", "bad\nroute", "\0", &"x".repeat(1025)] {
        for message in [
            Message::json(route, "r", "urn:x", serde_json::json!({})),
            Message::json("s", route, "urn:x", serde_json::json!({})),
            Message::json("s", "r", route, serde_json::json!({})),
        ] {
            assert!(message.is_err());
        }
    }
    for payload in [
        Payload::Json(serde_json::json!("x".repeat(65536))),
        Payload::Binary(vec![0; 65536]),
    ] {
        let message = match payload {
            Payload::Json(value) => Message::json("s", "r", "urn:x", value).unwrap(),
            Payload::Binary(value) => Message::binary("s", "r", "urn:x", value).unwrap(),
        };
        let mut wire = Vec::new();
        assert!(message.write(&mut wire).is_err());
        assert!(wire.is_empty(), "invalid message emitted a header");
    }
    assert!(Message::binary("s", "r", "urn:x", vec![0; 65537]).is_err());
    let message = Message::binary("s", "r", "urn:x", vec![0; 65515]).unwrap();
    let mut wire = Vec::new();
    message.write(&mut wire).unwrap();
    assert_eq!(wire.len(), 65540);
    assert_eq!(Message::read(&mut Cursor::new(&wire)).unwrap(), message);
}

#[test]
fn preserves_unicode_and_multi_byte_lengths() {
    let value =
        serde_json::json!({"type":"MEDIA_STATUS", "title":"🎬 café", "details":"x".repeat(300)});
    let message = Message::json(
        "sender-1",
        "receiver-0",
        "urn:x-cast:com.google.cast.media",
        value,
    )
    .unwrap();
    let mut wire = Vec::new();
    message.write(&mut wire).unwrap();
    assert_eq!(Message::read(&mut Cursor::new(wire)).unwrap(), message);
}

#[test]
fn propagates_io_failures_instead_of_claiming_a_message_was_sent() {
    use std::io::{self, Write};
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let message = Message::json("s", "r", "urn:x", serde_json::json!({})).unwrap();
    assert_eq!(
        message.write(&mut Broken).unwrap_err().kind(),
        io::ErrorKind::BrokenPipe
    );
}
