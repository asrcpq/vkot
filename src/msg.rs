use anyhow::{anyhow, Result};
use byteorder::{ByteOrder, LittleEndian as Ble};
use std::os::unix::net::UnixStream;

#[derive(Debug)]
pub enum VkotMsg {
	Print(String),
	MoveCursor([u32; 2]),
	SetColor([f32; 4]),
	Clear,

	Getch(char),
	Stream(UnixStream),
}

impl VkotMsg {
	pub fn from_buf(buf: &[u8], offset: &mut usize) -> Result<Vec<Self>> {
		let mut result = Vec::new();
		while *offset < buf.len() {
			let b0 = buf[*offset];
			*offset += 1;
			let msg = match b0 {
				b'p' => {
					let len = Ble::read_u32(&buf[*offset..*offset + 4]) as usize;
					*offset += 4;
					let string = String::from_utf8_lossy(&buf[*offset..*offset + len]).to_string();
					*offset += len;
					Self::Print(string)
				}
				b'm' => {
					let u1 = Ble::read_u32(&buf[*offset..*offset + 4]);
					let u2 = Ble::read_u32(&buf[*offset + 4..*offset + 8]);
					*offset += 8;
					Self::MoveCursor([u1, u2])
				}
				b'c' => {
					let mut color = [0f32; 4];
					for i in 0..4 {
						color[i] = Ble::read_f32(
							&buf[*offset + i * 4..*offset + 4 + i * 4]
						);
					}
					*offset += 16;
					Self::SetColor(color)
				}
				b'C' => {
					Self::Clear
				}
				c => return Err(anyhow!("unknown message type {:?}", c as char))
			};
			result.push(msg);
		}
		if *offset != buf.len() {
			eprintln!("bad msg: {:?}", String::from_utf8_lossy(buf))
		}
		Ok(result)
	}
}
