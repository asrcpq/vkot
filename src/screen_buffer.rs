const CLEAR: (char, u8) = (' ', 0);

pub struct ScreenBuffer {
	size: [i32; 2],
	cursor: [i32; 2],
	pub buffer: Vec<(char, u8)>,
}

impl ScreenBuffer {
	pub fn new(size: [i32; 2]) -> ScreenBuffer {
		ScreenBuffer {
			size,
			cursor: [0; 2],
			buffer: vec![CLEAR; (size[0] * size[1]) as usize],
		}
	}

	fn cursor_inc(&mut self) {
		if self.cursor[0] < self.size[0] - 1 {
			self.cursor[0] += 1;
		} else {
			self.cursor_newline();
		}
	}

	fn cursor_newline(&mut self) {
		self.cursor[0] = 0;
		if self.cursor[1] < self.size[1] - 1 {
			self.cursor[1] += 1;
		} else {
			self.scroll_up();
			self.clear_line();
		}
	}

	fn clear_line(&mut self) {
		for x in 0..self.size[0] {
			self.buffer[(x + self.cursor[1] * self.size[0]) as usize] = CLEAR;
		}
	}

	// does not move cursor
	fn scroll_up(&mut self) {
		for x in 0..self.size[0] {
			for y in 0..self.size[1] - 1 {
				self.buffer[(x + y * self.size[0]) as usize] =
					self.buffer[(x + (y + 1) * self.size[0]) as usize];
			}
		}
	}

	// not set char
	fn backspace(&mut self) {
		if self.cursor[0] > 0 {
			self.cursor[0] -= 1;
		}
	}

	pub fn set_char(&mut self, ch: char, color: u8, cursor_inc: bool) {
		if ch == '\r' {
			return
		}
		if ch == '\n' {
			self.cursor_newline();
			return;
		}
		if ch == 7 as char {
			println!("beep!");
			return;
		}
		if ch == 8 as char {
			// override cursor_inc
			self.backspace();
			return;
		}
		self.buffer[(self.cursor[0] + self.cursor[1] * self.size[0]) as usize] =
			(ch, color);
		if cursor_inc {
			self.cursor_inc();
		}
	}

	pub fn move_cursor(&mut self, x: i32, y: i32, abs: bool) {
		if abs {
			self.cursor[0] = x;
			self.cursor[1] = y;
		} else {
			self.cursor[0] += x;
			self.cursor[1] += y;
		}
		self.cursor[0] = self.cursor[0].min(self.size[0] - 1).max(0);
		self.cursor[1] = self.cursor[1].min(self.size[1] - 1).max(0);
	}

	// match csi definition
	pub fn erase_display(&mut self, param: i32) {
		if param == 0 {
			for x in 0..self.size[0] {
				for y in self.cursor[1]..self.size[1] {
					self.buffer[(x + y * self.size[0]) as usize] = CLEAR;
				}
			}
		} else if param == 1 {
			for x in 0..self.size[0] {
				for y in 0..=self.cursor[1] {
					self.buffer[(x + y * self.size[0]) as usize] = CLEAR;
				}
			}
		} else if param == 2 {
			for x in 0..self.size[0] {
				for y in 0..self.size[1] {
					self.buffer[(x + y * self.size[0]) as usize] = CLEAR;
				}
			}
		} else {
			println!("Unsupported EL Param!")
		}
	}

	// match csi definition
	pub fn erase_line(&mut self, param: i32) {
		if param == 0 {
			for i in self.cursor[0]..self.size[0] {
				self.buffer[(i + self.cursor[1] * self.size[0]) as usize] = CLEAR;
			}
		} else if param == 1 {
			for i in 0..=self.cursor[0] {
				self.buffer[(i + self.cursor[1] * self.size[0]) as usize] = CLEAR;
			}
		} else if param == 2 {
			for i in 0..self.size[0] {
				self.buffer[(i + self.cursor[1] * self.size[0]) as usize] = CLEAR;
			}
		} else {
			println!("Unsupported EL Param!")
		}
	}

	pub fn report_cursor(&self, param: i32) -> Option<Vec<u8>> {
		if param != 6 {
			println!("Error: only implemented report_cursor for final byte: n");
			return None;
		}
		let mut report = vec![27, b'['];
		report.extend(self.cursor[1].to_string().into_bytes());
		report.push(b';');
		report.extend(self.cursor[0].to_string().into_bytes());
		report.push(b'R');
		Some(report)
	}

	pub fn get_size(&self) -> [i32; 2] {
		self.size
	}

	pub fn get_render_data(&self) -> (&Vec<(char, u8)>, [i32; 2]) {
		(&self.buffer, self.cursor)
	}
}
