use anyhow::{anyhow, Result};
use std::convert::TryInto;
use std::os::unix::net::UnixStream;

use vkot_common::cell::Cell;

fn read_i16(bytes: &[u8]) -> i16 {
	i16::from_le_bytes(bytes.try_into().unwrap())
}

#[derive(Debug)]
pub enum VkotMsg {
	// client -> server
	Put([i16; 2], Cell),
	Cursor([i16; 2]),
	Blit([i16; 4], Vec<Cell>), // LTRD
	Fill([i16; 4], Cell),

	// server -> client
	Getch(u32),
	Skey([u8; 3]),
	Resized([i16; 2]),

	// server internal
	Stream(UnixStream),
	ChildExit,
}

impl VkotMsg {
	pub fn is_s2c(&self) -> bool {
		match self {
			Self::Getch(_) => true,
			Self::Skey(_) => true,
			Self::Resized(_) => true,
			_ => false,
		}
	}

	pub fn from_buf(buf: &[u8], offset: &mut usize) -> Result<Vec<Self>> {
		let mut result = Vec::new();
		let buflen = buf.len();
		loop {
			if *offset >= buflen { return Ok(result) }
			let b0 = buf[*offset];
			let mut region = [0i16; 4];
			let mut blit_len = 0;
			let test_len = match b0 {
				0 => 1 + 4,
				1 => 1 + 4 + 16,
				2 | 3 => {
					if *offset + 9 >= buflen {
						return Ok(result)
					}
					for idx in 0..4 {
						region[idx] = read_i16(&buf[*offset + idx * 2 + 1..*offset + idx * 2 + 3]);
					}
					if b0 == 2 {
						blit_len = ((region[2] - region[0]) * (region[3] - region[1])) as usize;
						1 + 8 + blit_len * 16
					} else {
						1 + 8 + 16
					}
				}
				_ => return Err(anyhow!("ERROR: bad msg {}", b0)),
			};
			if *offset + test_len > buflen {
				// eprintln!("{} {}/{}", offset, *offset + test_len, buflen);
				return Ok(result);
			}
			*offset += 1;

			let msg = match b0 {
				0 => {
					let cx = read_i16(&buf[*offset..*offset + 2]);
					let cy = read_i16(&buf[*offset + 2..*offset + 4]);
					*offset += 4;
					VkotMsg::Cursor([cx, cy])
				}
				1 => {
					let cx = read_i16(&buf[*offset..*offset + 2]);
					let cy = read_i16(&buf[*offset + 2..*offset + 4]);
					*offset += 4;
					let cell = Cell::from_le_bytes(&buf[*offset..]);
					*offset += 16;
					VkotMsg::Put([cx, cy], cell)
				}
				2 => {
					*offset += 8;
					let v = (0..blit_len).map(|idx| {
						let cell = Cell::from_le_bytes(&buf[*offset + idx * 16..]);
						cell
					}).collect::<Vec<_>>();
					*offset += blit_len * 16;
					VkotMsg::Blit(region, v)
				}
				3 => {
					*offset += 8;
					let cell = Cell::from_le_bytes(&buf[*offset..]);
					*offset += 16;
					VkotMsg::Fill(region, cell)
				}
				_ => panic!(),
			};
			result.push(msg);
		}
	}
}
