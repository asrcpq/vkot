use std::io::{BufWriter, Read, Write};
use std::os::unix::net::UnixStream;

fn main() {
	let mut lines: Vec<Vec<char>> = vec![vec![]];
	let mut buf = [0u8; 1024];
	let mut stream = UnixStream::connect("./vkot.socket").unwrap();
	let mut writer = BufWriter::new(stream.try_clone().unwrap());
	loop {
		let len = match stream.read(&mut buf) {
			Ok(l) => l,
			Err(_) => continue,
		};
		let string = String::from_utf8_lossy(&buf[..len]);
		for ch in string.chars() {
			eprintln!("{:?}", ch);
			if ch == '\r' {
				lines.push(Vec::new());
				continue
			}
			lines.last_mut().unwrap().push(ch);
		}
		writer.write(&[b'C']).unwrap();
		for (ln, line) in lines.iter().enumerate() {
			writer.write(&[b'c']).unwrap();
			if ln % 2 == 0 {
				writer.write(&1f32.to_le_bytes()).unwrap();
				writer.write(&0f32.to_le_bytes()).unwrap();
			} else {
				writer.write(&0f32.to_le_bytes()).unwrap();
				writer.write(&1f32.to_le_bytes()).unwrap();
			}
			writer.write(&1f32.to_le_bytes()).unwrap();
			writer.write(&1f32.to_le_bytes()).unwrap();
			writer.write(&[b'm']).unwrap();
			writer.write(&[0; 4]).unwrap();
			writer.write(&(ln as u32).to_le_bytes()).unwrap();
			writer.write(&[b'p']).unwrap();
			writer.write(&(line.len() as u32).to_le_bytes()).unwrap();
			writer.write(line.iter().collect::<String>().as_bytes()).unwrap();
		}
		writer.flush().unwrap();
	}
}
