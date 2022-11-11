use crate::msg::VkotMsg;

pub struct Console {
	size: [i16; 2],
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

	pub fn new(size: [i16; 2]) -> Self {
		let mut result = Self {
			size,
			buffer: Vec::new(),
			cpos: [0, 0],
		};
		result.clear();
		result
	}

	pub fn resize(&mut self, size: [i16; 2]) {
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

	fn pos_test(&self, [px, py]: [i16; 2]) -> bool {
		if px < 0 || py < 0 {
			return false
		}
		if px >= self.size[0] {
			return false
		}
		if py >= self.size[1] {
			return false
		}
		true
	}

	fn putchar(&mut self, [px, py]: [i16; 2], ch: u32, color: u32) {
		if !self.pos_test([px, py]) {
			return
		}
		self.buffer[py as usize][px as usize] = Cell {
			ch: char::from_u32(ch).unwrap(),
			color: color_from_u32(color),
		}
	}

	pub fn handle_msg(&mut self, msg: VkotMsg) {
		match msg {
			VkotMsg::Blit(mut region, data) => {
				if region[0] < 0 { region[0] = 0 }
				if region[1] < 0 { region[0] = 0 }
				if region[2] >= self.size[0] { region[2] = self.size[0] - 1 }
				if region[3] >= self.size[1] { region[3] = self.size[1] - 1 }
				if region[2] <= region[0] || region[3] <= region[1] {
					return
				}
				let mut idx = 0;
				for py in region[1]..region[3] {
					for px in region[0]..region[2] {
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

	pub fn get_size(&self) -> [i16; 2] {
		self.size
	}
}
