use crate::msg::VkotMsg;

pub struct Console {
	size: [u32; 2],
	buffer: Vec<Vec<Cell>>, // row first
	current_color: [f32; 4],
	cpos: [i32; 2],
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
			current_color: [0.0; 4],
			cpos: [0, 0],
		};
		result.clear();
		result
	}

	pub fn resize(&mut self, size: [u32; 2]) {
		if size[1] > self.size[1] {
			self.buffer.extend(vec![vec![]; (size[1] - self.size[1]) as usize]);
		} else {
			self.buffer.truncate(size[1] as usize);
		}
		if size[0] > self.size[0] {
			for row in self.buffer.iter_mut() {
				row.extend(vec![Cell::default(); size[0] as usize - row.len()]);
			}
		} else {
			for row in self.buffer.iter_mut() {
				row.truncate(size[0] as usize);
			}
		}
		self.size = size;
	}

	pub fn putchar(&mut self, ch: char) {
		self.buffer[self.cpos[1] as usize][self.cpos[0] as usize] = Cell {
			ch,
			color: self.current_color,
		}
	}

	pub fn limit_cpos(&mut self) {
		for i in 0..2 {
			if self.cpos[i] < 0 {
				self.cpos[i] = 0;
			} else if self.cpos[i] >= self.size[i] as i32 {
				self.cpos[i] = self.size[i] as i32 - 1;
			}
		}
	}

	pub fn handle_msg(&mut self, msg: VkotMsg) {
		match msg {
			VkotMsg::Print(ch) => {
				self.putchar(ch);
			}
			VkotMsg::Loc(ty, pos) => {
				match ty {
					0 => self.cpos[0] = pos,
					1 => self.cpos[1] = pos,
					2 => self.cpos[0] += pos,
					3 => self.cpos[1] += pos,
					_ => panic!(),
				}
				self.limit_cpos();
			}
			VkotMsg::SetColor(color) => {
				self.current_color = color;
			}
			VkotMsg::Clear => {
				eprintln!("cls");
				self.clear();
			}
			_ => panic!(),
		}
	}

	pub fn get_buffer(&self) -> &[Vec<Cell>] {
		&self.buffer
	}

	pub fn get_cpos(&self) -> [i32; 2] {
		self.cpos
	}
}
