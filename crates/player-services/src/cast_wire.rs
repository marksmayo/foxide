//! Bounded Cast V2.1 envelopes. This module performs no network I/O or discovery.
//! Wire fields follow Chromium's cast_channel.proto; callers provide a TLS stream.
use serde_json::Value;
use std::io::{self, Read, Write};

pub const MAX_FRAME_BYTES: usize = 64 * 1024;
const MAX_ROUTE_BYTES: usize = 1024;

#[derive(Clone, Debug, PartialEq)]
pub enum Payload {
    Json(Value),
    Binary(Vec<u8>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    source: String,
    destination: String,
    namespace: String,
    payload: Payload,
}

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "Invalid Cast envelope")
}

impl Message {
    pub fn json(
        source: &str,
        destination: &str,
        namespace: &str,
        value: Value,
    ) -> io::Result<Self> {
        Self::new(source, destination, namespace, Payload::Json(value))
    }
    pub fn binary(
        source: &str,
        destination: &str,
        namespace: &str,
        value: Vec<u8>,
    ) -> io::Result<Self> {
        if value.len() > MAX_FRAME_BYTES {
            return Err(invalid());
        }
        Self::new(source, destination, namespace, Payload::Binary(value))
    }
    fn new(source: &str, destination: &str, namespace: &str, payload: Payload) -> io::Result<Self> {
        for route in [source, destination, namespace] {
            if route.is_empty()
                || route.len() > MAX_ROUTE_BYTES
                || route.chars().any(char::is_control)
            {
                return Err(invalid());
            }
        }
        Ok(Self {
            source: source.into(),
            destination: destination.into(),
            namespace: namespace.into(),
            payload,
        })
    }
    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn destination(&self) -> &str {
        &self.destination
    }
    pub fn namespace(&self) -> &str {
        &self.namespace
    }
    pub fn payload(&self) -> &Payload {
        &self.payload
    }

    pub fn write(&self, stream: &mut impl Write) -> io::Result<()> {
        let mut body = vec![8, 0];
        for (key, text) in [
            (18, self.source.as_bytes()),
            (26, self.destination.as_bytes()),
            (34, self.namespace.as_bytes()),
        ] {
            bytes(&mut body, key, text)?;
        }
        match &self.payload {
            Payload::Json(value) => {
                body.extend_from_slice(&[40, 0]);
                // Bound serialization before a second full copy can grow.
                let mut json = Bounded(Vec::new());
                serde_json::to_writer(&mut json, value).map_err(io::Error::other)?;
                bytes(&mut body, 50, &json.0)?;
            }
            Payload::Binary(value) => {
                body.extend_from_slice(&[40, 1]);
                bytes(&mut body, 58, value)?;
            }
        }
        stream.write_all(&(body.len() as u32).to_be_bytes())?;
        stream.write_all(&body)
    }

    pub fn read(stream: &mut impl Read) -> io::Result<Self> {
        let mut header = [0; 4];
        stream.read_exact(&mut header)?;
        let size = u32::from_be_bytes(header) as usize;
        if size == 0 || size > MAX_FRAME_BYTES {
            return Err(invalid());
        }
        let mut body = vec![0; size];
        stream.read_exact(&mut body)?;
        let mut cursor = body.as_slice();
        let mut fields: [Option<Field<'_>>; 7] = [None; 7];
        while !cursor.is_empty() {
            let tag = varint(&mut cursor)?;
            let number = tag >> 3;
            if number == 0 || number > 0x1fff_ffff {
                return Err(invalid());
            }
            let field = match tag & 7 {
                0 => Field::Number(varint(&mut cursor)?),
                1 => {
                    take(&mut cursor, 8)?;
                    Field::Ignored
                }
                2 => {
                    let size = usize::try_from(varint(&mut cursor)?).map_err(|_| invalid())?;
                    Field::Bytes(take(&mut cursor, size)?)
                }
                5 => {
                    take(&mut cursor, 4)?;
                    Field::Ignored
                }
                _ => return Err(invalid()),
            };
            if number <= 7 {
                let slot = &mut fields[number as usize - 1];
                if slot.is_some() {
                    return Err(invalid());
                }
                *slot = Some(field);
            }
        }
        if !matches!(fields[0], Some(Field::Number(0))) {
            return Err(invalid());
        }
        let text = |index: usize| match fields[index] {
            Some(Field::Bytes(data)) => std::str::from_utf8(data).map_err(|_| invalid()),
            _ => Err(invalid()),
        };
        match (fields[4], fields[5], fields[6]) {
            (Some(Field::Number(0)), Some(Field::Bytes(_)), None) => {
                let value = serde_json::from_str(text(5)?).map_err(|_| invalid())?;
                Self::json(text(1)?, text(2)?, text(3)?, value)
            }
            (Some(Field::Number(1)), None, Some(Field::Bytes(data))) => {
                Self::binary(text(1)?, text(2)?, text(3)?, data.to_vec())
            }
            _ => Err(invalid()),
        }
    }
}

#[derive(Clone, Copy)]
enum Field<'a> {
    Number(u64),
    Bytes(&'a [u8]),
    Ignored,
}

fn take<'a>(cursor: &mut &'a [u8], size: usize) -> io::Result<&'a [u8]> {
    let (head, tail) = cursor.split_at_checked(size).ok_or_else(invalid)?;
    *cursor = tail;
    Ok(head)
}
fn varint(cursor: &mut &[u8]) -> io::Result<u64> {
    let mut value = 0;
    for index in 0..10 {
        let byte = take(cursor, 1)?[0];
        if index == 9 && byte > 1 {
            return Err(invalid());
        }
        value |= u64::from(byte & 127) << (index * 7);
        if byte & 128 == 0 {
            return Ok(value);
        }
    }
    Err(invalid())
}
fn bytes(body: &mut Vec<u8>, key: u8, data: &[u8]) -> io::Result<()> {
    if body.len().saturating_add(data.len()).saturating_add(4) > MAX_FRAME_BYTES {
        return Err(invalid());
    }
    body.push(key);
    let mut length = data.len();
    while length >= 128 {
        body.push((length as u8 & 127) | 128);
        length >>= 7;
    }
    body.push(length as u8);
    body.extend_from_slice(data);
    Ok(())
}
struct Bounded(Vec<u8>);
impl Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.0.len().saturating_add(bytes.len()) > MAX_FRAME_BYTES {
            return Err(invalid());
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
