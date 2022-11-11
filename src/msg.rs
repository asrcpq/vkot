use anyhow::{anyhow, Result};
use std::convert::TryInto;
use std::os::unix::net::UnixStream;

fn read_u32(bytes: &[u8]) -> u32 {
	u32::from_le_bytes(bytes.try_into().unwrap())
}

fn read_i16(bytes: &[u8]) -> i16 {
	i16::from_le_bytes(bytes.try_into().unwrap())
}

#[derive(Debug)]
pub enum VkotMsg {
	// client -> server
	Blit([i16; 4], Vec<(u32, u32)>), // LTRD
	Put([i16; 2], (u32, u32)),
	Cursor([i16; 2]),

	// server -> client
	Getch(u32),
	Resized([i16; 2]),

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
		loop {
			let b0 = buf[*offset];
			let mut region = [0i16; 4];
			let mut blit_len = 0;
			let test_len = match b0 {
				1 => 1 + 4 + 8,
				2 => 1 + 4,
				0 => {
					if *offset + 9 >= buflen {
						return Ok(result)
					}
					for idx in 0..4 {
						region[idx] = read_i16(&buf[*offset + idx * 2 + 1..*offset + idx * 2 + 3]);
					}
					blit_len = ((region[2] - region[0]) * (region[3] - region[1])) as usize;
					1 + 8 + blit_len * 8
				}
				_ => return Err(anyhow!("ERROR: bad msg {}", b0)),
			};
			if *offset + test_len >= buflen {
				return Ok(result);
			}
			*offset += 1;

			let msg = match b0 {
				1 => {
					eprintln!("msg put");
					let cx = read_i16(&buf[*offset..*offset + 2]);
					let cy = read_i16(&buf[*offset + 2..*offset + 4]);
					let cu = read_u32(&buf[*offset + 4..*offset + 8]);
					let cc = read_u32(&buf[*offset + 8..*offset + 12]);
					*offset += 12;
					VkotMsg::Put([cx, cy], (cu, cc))
				}
				2 => {
					eprintln!("msg cursor");
					let cx = read_i16(&buf[*offset..*offset + 2]);
					let cy = read_i16(&buf[*offset + 2..*offset + 4]);
					*offset += 4;
					VkotMsg::Cursor([cx, cy])
				}
				0 => {
					eprintln!("msg blit {}", blit_len);
					*offset += 8;
					let v = (0..blit_len).map(|idx| {
						let cu = read_u32(&buf[*offset + idx * 8..*offset + idx * 8 + 4]);
						let cc = read_u32(&buf[*offset + idx * 8 + 4..*offset + idx * 8 + 8]);
						(cu, cc)
					}).collect::<Vec<_>>();
					*offset += blit_len * 8;
					VkotMsg::Blit(region, v)
				}
				_ => panic!(),
			};
			result.push(msg);
		}
	}
}
