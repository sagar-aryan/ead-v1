//! Bounds-checked little-endian reader for protocol payloads.

use super::{ProtocolError, Result};

pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(n).ok_or(ProtocolError::TooShort)?;
        let slice = self.data.get(self.pos..end).ok_or(ProtocolError::TooShort)?;
        self.pos = end;
        Ok(slice)
    }

    pub fn rest(&mut self) -> &'a [u8] {
        let slice = &self.data[self.pos..];
        self.pos = self.data.len();
        slice
    }

    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.bytes(1)?[0])
    }

    pub fn i8(&mut self) -> Result<i8> {
        Ok(self.u8()? as i8)
    }

    pub fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.array::<2>()?))
    }

    pub fn i16(&mut self) -> Result<i16> {
        Ok(self.u16()? as i16)
    }

    pub fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.array::<4>()?))
    }

    pub fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(self.array::<4>()?))
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    pub fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.array::<8>()?))
    }

    pub fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        Ok(self.bytes(N)?.try_into().expect("slice length checked"))
    }

    pub fn i16_array<const N: usize>(&mut self) -> Result<[i16; N]> {
        let mut out = [0i16; N];
        for slot in out.iter_mut() {
            *slot = self.i16()?;
        }
        Ok(out)
    }

    /// `u8` length prefix + UTF-8 bytes. Invalid UTF-8 is replaced rather than
    /// rejected: a garbled version string must not drop an otherwise good message.
    pub fn str8(&mut self) -> Result<String> {
        let len = self.u8()? as usize;
        Ok(String::from_utf8_lossy(self.bytes(len)?).into_owned())
    }
}
