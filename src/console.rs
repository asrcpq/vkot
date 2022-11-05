use crate::screen_buffer::ScreenBuffer;

pub struct Console {
	csi_buf: Vec<u8>,
	screen: Vec<ScreenBuffer>,
	sid: usize,
	color: u8,
}

impl Console {
	pub fn new(size: [i32; 2]) -> Console {
		Console {
			csi_buf: Vec::new(),
			screen: vec![ScreenBuffer::new(size), ScreenBuffer::new(size)],
			sid: 0,
			color: 231,
		}
	}

	fn proc_csi(&mut self) -> Option<Vec<u8>> {
		println!("{:?}", String::from_utf8(self.csi_buf.clone()).unwrap());
		if self.csi_buf.is_empty() {
			return None;
		}
		if self.csi_buf[0] != 27 {
			println!("csi_buf error");
			return None;
		}
		let mut param = Vec::new();
		let mut final_byte = None;
		for ch in self.csi_buf[1..].iter() {
			match ch {
				0x30..=0x3F => {
					param.push(*ch);
				}
				0x40..=0x7F => {
					final_byte = Some(ch);
				}
				_ => {}
			}
		}
		let mut report = None;
		match final_byte {
			Some(b'D') => {
				self.screen[self.sid].move_cursor(
					-String::from_utf8(param)
						.unwrap()
						.parse::<i32>()
						.unwrap_or(1),
					0,
					false,
				);
			}
			Some(b'C') => {
				self.screen[self.sid].move_cursor(
					String::from_utf8(param)
						.unwrap()
						.parse::<i32>()
						.unwrap_or(1),
					0,
					false,
				);
			}
			Some(b'A') => {
				self.screen[self.sid].move_cursor(
					0,
					-String::from_utf8(param)
						.unwrap()
						.parse::<i32>()
						.unwrap_or(1),
					false,
				);
			}
			Some(b'B') => {
				self.screen[self.sid].move_cursor(
					0,
					String::from_utf8(param)
						.unwrap()
						.parse::<i32>()
						.unwrap_or(1),
					false,
				);
			}
			Some(b'H') => {
				// ansi coodinate is 1..=n, not 0..n
				let params = String::from_utf8(param)
					.unwrap()
					.split(';')
					.map(|x| x.parse::<i32>().unwrap_or(1) - 1)
					.collect::<Vec<i32>>();
				if params.len() == 1 {
					self.screen[self.sid].move_cursor(1, 1, true);
				} else {
					self.screen[self.sid]
						.move_cursor(params[1], params[0], true);
				}
			}
			Some(b'J') => {
				self.screen[self.sid].erase_display(
					String::from_utf8(param)
						.unwrap()
						.parse::<i32>()
						.unwrap_or(0),
				);
			}
			Some(b'K') => {
				self.screen[self.sid].erase_line(
					String::from_utf8(param)
						.unwrap()
						.parse::<i32>()
						.unwrap_or(0),
				);
			}
			Some(b'm') => {
				let params = String::from_utf8(param)
					.unwrap()
					.split(';')
					.map(|x| x.parse::<i32>().unwrap_or(0))
					.collect::<Vec<i32>>();
				for param in params.iter().cloned() {
					if param == 0 {
						self.color = 231;
					} else if (30..=37).contains(&param) {
						self.color = param as u8 - 30;
					} else if (90..=97).contains(&param) {
						self.color = param as u8 - 90 + 8;
					} else if (40..=47).contains(&param) {
						// skip
					} else if (100..=107).contains(&param) {
						// skip
					} else {
						eprintln!("Unimplemented sgr sequence {:?}", params);
					}
				}
			}
			Some(b'n') => {
				report = self.screen[self.sid].report_cursor(
					String::from_utf8(param)
						.unwrap()
						.parse::<i32>()
						.unwrap_or(0),
				);
			}
			Some(b'h') => {
				if std::str::from_utf8(&self.csi_buf).unwrap() == "\x1b[?1049h"
				{
					self.sid = 1;
				} else {
					println!(
						"Unimplemented csi sequence {:?}",
						String::from_utf8(self.csi_buf.clone()).unwrap()
					);
				}
			}
			Some(b'l') => {
				if std::str::from_utf8(&self.csi_buf).unwrap() == "\x1b[?1049l"
				{
					self.sid = 0;
				} else {
					println!(
						"Unimplemented csi sequence {:?}",
						String::from_utf8(self.csi_buf.clone()).unwrap()
					);
				}
			}
			Some(_) => {
				println!(
					"Unimplemented final byte {:?}",
					String::from_utf8(self.csi_buf.clone()).unwrap()
				);
			}
			_ => {}
		}
		self.csi_buf.clear();
		report
	}

	pub fn put_char(&mut self, ch: char) -> Option<Vec<u8>> {
		let byte = ch as u8;
		if byte == 0 {
			// TODO: investigate the reason why erase write zero
			return None
		}

		if byte == 27 {
			//self.proc_csi();
			self.csi_buf = vec![27];
			return None;
		}

		if !self.csi_buf.is_empty() {
			if self.csi_buf.len() == 1 && byte == b'[' {
				self.csi_buf.push(byte);
				return None;
			}
			if byte >= 0x40 && byte < 0x80 {
				self.csi_buf.push(byte);
				return self.proc_csi();
			}
			self.csi_buf.push(byte);
			return None;
		}

		if byte == 13 { // FIXME: ???
			self.screen[self.sid].set_char(ch, self.color, false);
			return None;
		}
		self.screen[self.sid].set_char(ch, self.color, true);
		None
	}

	pub fn get_size(&self) -> [i32; 2] {
		self.screen[0].get_size()
	}

	pub fn render_data(&mut self) -> (&Vec<(char, u8)>, [i32; 2]) {
		self.screen[self.sid].get_render_data()
	}
}
