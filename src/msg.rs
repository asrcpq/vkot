use anyhow::{anyhow, Result};
use byteorder::{ByteOrder, LittleEndian as Ble};
use std::os::unix::net::UnixStream;

const REMAIN_LOOKUP: [usize; 4] = [4, 5, 16, 0];

#[derive(Debug)]
pub enum VkotMsg {
	// client -> server
	Print(char),
	Loc(u8, i32), // 0-4: x_abs, y_abs, x_rel, y_rel
	SetColor([f32; 4]),
	Clear,

	// server -> client
	Getch(u32),
	Resized([u32; 2]),

	// server internal
	Stream(UnixStream),
}

impl VkotMsg {
	pub fn is_s2c(&self) -> bool {
		match self {
			Self::Getch(_) => true,
			Self::Resized(_) => true,
			_ => false,
		}
	}

	pub fn from_buf(buf: &[u8], offset: &mut usize) -> Result<Vec<Self>> {
		let mut result = Vec::new();
		let buflen = buf.len();
		while *offset < buflen {
			let b0 = buf[*offset];
			if b0 < 128 {
				let msg = Self::Print(b0 as char);
				result.push(msg);
				*offset += 1;
				continue
			}
			let b0 = b0 - 128;
			*offset += 1;
			let remain = buflen - *offset;
			if remain < REMAIN_LOOKUP[b0 as usize] {
				break
			}
			let msg = match b0 {
				0 => {
					let ch = Ble::read_u32(&buf[*offset..*offset + 4]);
					let ch = char::from_u32(ch).unwrap();
					*offset += 4;
					Self::Print(ch)
				}
				1 => {
					let i1 = buf[*offset];
					let i2 = Ble::read_i32(&buf[*offset + 1..*offset + 5]);
					*offset += 5;
					// eprintln!("loc {} {}", i1, i2);
					Self::Loc(i1, i2)
				}
				2 => {
					let mut color = [0f32; 4];
					for i in 0..4 {
						color[i] = Ble::read_f32(
							&buf[*offset + i * 4..*offset + 4 + i * 4]
						);
					}
					*offset += 16;
					Self::SetColor(color)
				}
				3 => {
					Self::Clear
				}
				c => return Err(anyhow!("unknown message type {:?}", c as char))
			};
			result.push(msg);
		}
		Ok(result)
	}
}
