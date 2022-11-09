use crate::msg::VkotMsg;

pub struct Console {
	size: [u32; 2],
	buffer: Vec<Cell>,
	current_color: [f32; 4],
	cpos: [u32; 2],
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
			Cell::default();
			(self.size[0] * self.size[1]) as usize
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

	pub fn p2i(&self, p: [u32; 2]) -> usize {
		(p[0] + p[1] * self.size[0]) as usize
	}

	pub fn handle_msg(&mut self, msg: VkotMsg) {
		match msg {
			VkotMsg::Print(string) => {
				eprintln!("print {}", string.len());
				let chars: Vec<char> = string.chars().collect();
				let offset = self.p2i(self.cpos);
				for (idx, ch) in chars.iter().enumerate() {
					if idx + offset >= self.buffer.len() {
						eprintln!("overflow");
						break
					}
					self.buffer[idx + offset] = Cell {
						ch: *ch,
						color: self.current_color,
					}
				}
			}
			VkotMsg::MoveCursor(pos) => {
				eprintln!("move cursor to {:?}", pos);
				self.cpos = pos;
			}
			VkotMsg::SetColor(color) => {
				eprintln!("set color to {:?}", color);
				self.current_color = color;
			}
			VkotMsg::Clear => {
				eprintln!("cls");
				self.clear();
			}
			_ => panic!(),
		}
	}

	pub fn get_buffer(&self) -> &[Cell] {
		&self.buffer
	}

	pub fn get_cpos(&self) -> [u32; 2] {
		self.cpos
	}

	pub fn get_size(&self) -> [u32; 2] {
		self.size
	}
}
