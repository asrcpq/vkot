mod color_table;
mod console;
mod screen_buffer;

use nix::fcntl::{open, OFlag};
use nix::pty::{grantpt, posix_openpt, ptsname, unlockpt};
use nix::sys::stat::Mode;
use nix::unistd;
use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::path::Path;
use std::sync::mpsc;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoopBuilder};

use console::Console;
use triangles::renderer::Renderer;
use triangles::bmtext::FontConfig;

struct PTY {
	pub master: RawFd,
	pub slave: RawFd,
}

fn openpty() -> Result<PTY, String> {
	// Open a new PTY master
	let master_fd = posix_openpt(OFlag::O_RDWR).unwrap();

	grantpt(&master_fd).unwrap();
	unlockpt(&master_fd).unwrap();

	// Get the name of the slave
	let slave_name = unsafe { ptsname(&master_fd).unwrap() };

	// Try to open the slave
	let slave_fd =
		open(Path::new(&slave_name), OFlag::O_RDWR, Mode::empty()).unwrap();

	use std::os::unix::io::IntoRawFd;
	Ok(PTY {
		master: master_fd.into_raw_fd(),
		slave: slave_fd,
	})
}

#[derive(Debug)]
enum UserEvent {
	Flush,
	Quit,
}

fn main_loop(pty_master: OwnedFd) {
	let el = EventLoopBuilder::<UserEvent>::with_user_event().build();
	let proxy = el.create_proxy();

	let mut rdr = Renderer::new(&el);
	let ssize = rdr.get_size();
	let mut fc = FontConfig::default();
	fc.resize_screen(ssize);
	let img = fc.bitw_loader("../bitw/data/lat15_terminus32x16.txt");
	rdr.upload_tex(img, 0);
	let mut model = fc.generate_model();
	let mut _tmhandle = None; // text model
	let mut _cmhandle = None; // cursor model

	let tsize = fc.get_terminal_size_in_char();
	let ct = crate::color_table::ColorTable::default();
	let mut console = Console::new([tsize[0] as i32, tsize[1] as i32]);
	let pty_master2 = pty_master.try_clone().unwrap();

	let (send, recv) = mpsc::sync_channel(1);
	{
		// spawn a thread which reads bytes from the slave
		// and forwards them to the main thread
		let mut buf = vec![0; 4 * 1024];
		std::thread::spawn(move || loop {
			match unistd::read(pty_master.as_raw_fd(), &mut buf) {
				Ok(nb) => {
					let bytes = buf[..nb].to_vec();
					send.send(bytes).unwrap();
					proxy.send_event(UserEvent::Flush).unwrap();
				}
				Err(e) => {
					eprintln!("{:?}", e);
					proxy.send_event(UserEvent::Quit).unwrap();
					break;
				}
			}
		});
	}

	el.run(move |event, _, ctrl| match event {
		Event::WindowEvent { event: e, .. } => {
			match e {
				WindowEvent::CloseRequested => {
					*ctrl = ControlFlow::Exit;
				}
				WindowEvent::Resized(_) => {
					rdr.damage();
				}
				WindowEvent::ReceivedCharacter(ch) => {
					let mut buf = [0_u8; 4];
					let utf8 = ch.encode_utf8(&mut buf).as_bytes();
					nix::unistd::write(pty_master2.as_raw_fd(), utf8).unwrap();
				}
				_ => {}
			}
		}
		Event::RedrawRequested(_window_id) => {
			if let Ok(bytes) = recv.try_recv() {
				for ch in String::from_utf8_lossy(&bytes).chars() {
					console.put_char(ch);
				}
			}
			let [tx, ty] = console.get_size();
			let [tx, _] = [tx as u32, ty as u32];
			model.faces = Vec::new();
			let (chars, cursor_pos) = console.render_data();
			for (idx, (ch, color)) in chars.iter().enumerate() {
				let idx = idx as u32;
				let offset_x = idx % tx;
				let offset_y = idx / tx;
				model.faces.extend(fc.text2fs(
					[offset_x, offset_y],
					std::iter::once(*ch),
					ct.rgb_from_256color(*color),
					0,
				));
			}
			_tmhandle = Some(rdr.insert_model(&model));
			rdr.render2();
		}
		Event::UserEvent(event) => {
			match event {
				UserEvent::Quit => {
					*ctrl = ControlFlow::Exit;
				}
				UserEvent::Flush => {
					rdr.redraw();
				}
			}
		}
		Event::MainEventsCleared => {
			rdr.redraw();
			*ctrl = ControlFlow::Wait;
		}
		_ => {}
	})
}

fn main() {
	let pty = openpty().unwrap();

	let result = unsafe {unistd::fork()};
	match result {
		Ok(unistd::ForkResult::Parent { .. }) => {
			unistd::close(pty.slave).unwrap();
			let pty_master = unsafe { OwnedFd::from_raw_fd(pty.master) };
			main_loop(pty_master);
		}
		Ok(unistd::ForkResult::Child) => {
			unistd::close(pty.master).unwrap();

			// create process group
			unistd::setsid().unwrap();

			const TIOCSCTTY: usize = 0x540E;
			nix::ioctl_write_int_bad!(tiocsctty, TIOCSCTTY);
			unsafe { tiocsctty(pty.slave, 0).unwrap() };

			unistd::dup2(pty.slave, 0).unwrap(); // stdin
			unistd::dup2(pty.slave, 1).unwrap(); // stdout
			unistd::dup2(pty.slave, 2).unwrap(); // stderr
			unistd::close(pty.slave).unwrap();

			use std::ffi::CString;
			let path = CString::new("/bin/bash").unwrap();
			std::env::set_var("TERM", "vt100");

			unistd::execv::<CString>(&path, &[]).unwrap();
		}
		Err(_) => {}
	}
}
