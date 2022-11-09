use anyhow::{anyhow, Result};
use std::os::unix::net::UnixStream;

#[derive(Debug)]
pub enum VkotMsg {
	Print(String),
	MoveCursor([u32; 2]),
	Getch(char),
	Stream(UnixStream),
}

fn parse_u32(array: &[u8]) -> u32 {
	((array[0] as u32) <<  0) +
	((array[1] as u32) <<  8) +
	((array[2] as u32) << 16) +
	((array[3] as u32) << 24)
}

impl VkotMsg {
	pub fn from_buf(buf: &[u8]) -> Result<Self> {
		match buf[0] {
			b'p' => {
				let string = String::from_utf8_lossy(&buf[1..]).to_string();
				Ok(Self::Print(string))
			},
			b'c' => {
				let u1 = parse_u32(&buf[1..5]);
				let u2 = parse_u32(&buf[5..9]);
				Ok(Self::MoveCursor([u1, u2]))
			}
			_ => Err(anyhow!("unknown message type"))
		}
	}
}
