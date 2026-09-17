//! Consistent Overhead Byte Stuffing (Cheshire & Baker), used to delimit
//! protocol messages on the USB byte stream.

use super::{ProtocolError, Result};

pub fn encode(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + data.len() / 254 + 1);
    let mut code_index = 0;
    out.push(0); // placeholder for the first block's code
    let mut code: u8 = 1;
    for &byte in data {
        if byte == 0 {
            out[code_index] = code;
            code_index = out.len();
            out.push(0);
            code = 1;
            continue;
        }
        out.push(byte);
        code += 1;
        if code == 0xFF {
            // A full block of 254 data bytes carries no implied zero.
            out[code_index] = code;
            code_index = out.len();
            out.push(0);
            code = 1;
        }
    }
    out[code_index] = code;
    out
}

pub fn decode(data: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        let code = data[i];
        if code == 0 {
            return Err(ProtocolError::BadCobs);
        }
        let block_end = i + code as usize;
        if block_end > data.len() {
            return Err(ProtocolError::BadCobs);
        }
        let block = &data[i + 1..block_end];
        if block.contains(&0) {
            return Err(ProtocolError::BadCobs);
        }
        out.extend_from_slice(block);
        i = block_end;
        if code != 0xFF && i < data.len() {
            out.push(0);
        }
    }
    Ok(out)
}
