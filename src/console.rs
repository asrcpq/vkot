use crate::msg::VkotMsg;

pub struct Console {
	size: [u32; 2],
	buffer: Vec<Vec<Cell>>, // row first
	cpos: [i16; 2],
}

#[derive(Clone)]
pub struct Cell {
	pub ch: char,
	pub color: [f32; 4],
}

impl Default for Cell {
	fn default() -> Self {
		Self {
			ch: ' ',
			color: [1.0; 4],
		}
	}
}

fn color_from_u32(uc: u32) -> [f32; 4] {
	let bs = uc.to_le_bytes();
	core::array::from_fn(|idx| bs[idx] as f32 / 255.0)
}

impl Console {
	fn clear(&mut self) {
		self.buffer = vec![
			vec![Cell::default(); self.size[0] as usize];
			self.size[1] as usize
		];
	}

	pub fn new(size: [u32; 2]) -> Self {
		let mut result = Self {
			size,
			buffer: Vec::new(),
			cpos: [0, 0],
		};
		result.clear();
		result
	}

	pub fn resize(&mut self, size: [u32; 2]) {
		// eprintln!("resize to {:?}", size);
		if size[1] > self.size[1] {
			self.buffer.extend(vec![vec![]; (size[1] - self.size[1]) as usize]);
		} else {
			self.buffer.truncate(size[1] as usize);
		}
		for row in self.buffer.iter_mut() {
			row.resize_with(size[0] as usize, Default::default);
		}
		self.size = size;
	}

	fn putchar(&mut self, [px, py]: [i16; 2], ch: u32, color: u32) {
		self.buffer[px as usize][py as usize] = Cell {
			ch: char::from_u32(ch).unwrap(),
			color: color_from_u32(color),
		}
	}

	pub fn handle_msg(&mut self, msg: VkotMsg) {
		match msg {
			VkotMsg::Blit(region, data) => {
				let mut idx = 0;
				for px in region[0]..region[1] {
					for py in region[2]..region[3] {
						let (ch, color) = data[idx];
						self.putchar([px, py], ch, color);
						idx += 1;
					}
				}
			},
			VkotMsg::Cursor(pos) => {
				self.cpos = pos;
			},
			VkotMsg::Put(pos, (ch, color)) => {
				self.putchar(pos, ch, color);
			},
			_ => panic!(),
		}
	}

	pub fn get_buffer(&self) -> &[Vec<Cell>] {
		&self.buffer
	}

	pub fn get_cpos(&self) -> [i16; 2] {
		self.cpos
	}
}
