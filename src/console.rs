use crate::msg::VkotMsg;

use vkot_common::cell::Cell;

pub struct Console {
	size: [i16; 2],
	buffer: Vec<Vec<Cell>>, // row first
	cpos: [i16; 2],
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

	fn setchar_checked(&mut self, [px, py]: [i16; 2], cell: Cell) {
		if !self.pos_test([px, py]) {
			return
		}
		self.buffer[py as usize][px as usize] = cell;
	}

	fn fit_region(&mut self, region: &mut [i16; 4]) -> bool {
		if region[0] < 0 { region[0] = 0 }
		if region[1] < 0 { region[0] = 0 }
		if region[2] > self.size[0] { region[2] = self.size[0] }
		if region[3] > self.size[1] { region[3] = self.size[1] }
		if region[2] <= region[0] || region[3] <= region[1] {
			return false
		}
		true
	}

	pub fn handle_msg(&mut self, msg: VkotMsg) {
		match msg {
			VkotMsg::Cursor(pos) => {
				self.cpos = pos;
			},
			VkotMsg::Put(pos, cell) => {
				self.setchar_checked(pos, cell);
			},
			VkotMsg::Blit(mut region, cells) => {
				let mut idx = 0;
				if !self.fit_region(&mut region) {
					return
				}
				for py in region[1]..region[3] {
					for px in region[0]..region[2] {
						self.setchar_checked([px, py], cells[idx]);
						idx += 1;
					}
				}
			},
			VkotMsg::Fill(mut region, cell) => {
				if !self.fit_region(&mut region) {
					return
				}
				for py in region[1]..region[3] {
					for px in region[0]..region[2] {
						self.setchar_checked([px, py], cell);
					}
				}
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
