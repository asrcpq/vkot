mod console;
mod screen_buffer;

use nix::fcntl::{open, OFlag};
use nix::pty::{grantpt, posix_openpt, ptsname, unlockpt};
use nix::sys::stat::Mode;
use nix::unistd;
use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::path::Path;
use std::sync::mpsc;
use winit::event::{Event, WindowEvent, ElementState};
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

enum UserEvent {
	Flush,
	Quit,
}

fn main() {
	let pty = openpty().unwrap();

	let result = unsafe {unistd::fork()};
	match result {
		Ok(unistd::ForkResult::Parent { child: shell_pid }) => {
			unistd::close(pty.slave).unwrap();

			let mut shift: bool = false;
			let mut ctrl: bool = false;
			let el = EventLoopBuilder::<UserEvent>::with_user_event().build();
			let proxy = el.create_proxy();

			let mut rdr = Renderer::new(&el);
			let ssize = rdr.get_size();
			let mut fc = FontConfig::default();
			fc.resize_screen(ssize);
			let img = fc.bitw_loader("../bitw/data/lat15_terminus32x16.txt");
			rdr.upload_tex(img, 0);
			let mut model = fc.generate_model();
			let mut tmhandle = None;

			let mut tsize = fc.get_terminal_size_in_char();
			let mut console = Console::new((tsize[0] as i32, tsize[1] as i32));

			'main_loop: loop {
				let pty_master = unsafe { OwnedFd::from_raw_fd(pty.master) };
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
								proxy.send_event(UserEvent::Flush);
								// TODO: notify update
							}
							Err(e) => {
								// TODO: quit
								eprintln!("{:?}", e);
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
								let ssize = rdr.get_size();
								rdr.damage();
							}
							WindowEvent::ReceivedCharacter(ch) => {
								let ch = ch as u8;
								nix::unistd::write(pty.master, &[ch]).unwrap();
							}
							_ => {}
						}
					}
					Event::RedrawRequested(_window_id) => {
						rdr.render2();
					}
					Event::MainEventsCleared => {
						let data = console.render_data();
						let string = String::from_utf8_lossy(data);
						model.faces = fc.text2fs(&string, 0);
						tmhandle = Some(rdr.insert_model(&model));
						if let Ok(bytes) = recv.try_recv() {
							for byte in bytes.into_iter() {
								console.put_char(byte);
							}
						}
						rdr.redraw();
						*ctrl = ControlFlow::Wait;
					}
					_ => {}
				})
			}
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
			let path = CString::new("/bin/dash").unwrap();
			std::env::set_var("TERM", "dumb");

			unistd::execv::<CString>(&path, &[]).unwrap();
		}
		Err(_) => {}
	}
}
