use crate::msg::VkotMsg;

pub struct Console {
	size: [u32; 2],
	buffer: Vec<char>,
	cpos: [u32; 2],
}

impl Console {
	pub fn new(size: [u32; 2]) -> Self {
		Self {
			size,
			buffer: vec![' '; (size[0] * size[1]) as usize],
			cpos: [0, 0],
		}
	}

	pub fn p2i(&self, p: [u32; 2]) -> usize {
		(p[0] + p[1] * self.size[0]) as usize
	}

	pub fn handle_msg(&mut self, msg: VkotMsg) {
		match msg {
			VkotMsg::Print(string) => {
				let chars: Vec<char> = string.chars().collect();
				let offset = self.p2i(self.cpos);
				for (idx, ch) in chars.iter().enumerate() {
					self.buffer[idx + offset] = *ch;
				}
			}
			VkotMsg::MoveCursor(pos) => {
				eprintln!("move cursor to {:?}", pos);
				self.cpos = pos;
			}
			_ => panic!(),
		}
	}

	pub fn render_data(&self) -> (Vec<char>, [u32; 2]) {
		(self.buffer.clone(), self.cpos)
	}

	pub fn get_size(&self) -> [u32; 2] {
		self.size
	}
}
