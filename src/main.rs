// mod color_table;
mod console;
mod msg;

use std::io::Read;
use std::os::unix::net::UnixListener;
use std::sync::mpsc::{channel, Sender};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoopBuilder};

use triangles::renderer::Renderer;
use triangles::bmtext::FontConfig;
use triangles::model::cmodel::{Face, Model};

#[derive(Debug)]
enum UserEvent {
	Flush,
	Quit,
}

fn client_handler(tx: Sender<msg::VkotMsg>) {
	let _ = std::fs::remove_file("./vkot.socket");
	let listener = UnixListener::bind("./vkot.socket").unwrap();
	let mut buf = [0u8; 1024];
	for stream in listener.incoming() {
		let mut stream = match stream {
			Ok(s) => s,
			Err(e) => {
				eprintln!("{:?}", e);
				continue
			}
		};
		loop {
			let len = match stream.read(&mut buf) {
				Ok(s) => s,
				Err(e) => break,
			};
			// FIXME
			let string = String::from_utf8_lossy(&buf[..len]);
			tx.send(msg::VkotMsg::Print(string.to_string())).unwrap();
		}
	}
}

fn main() {
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
	let [fsx, fsy] = fc.get_font_size();
	let (tx, rx) = channel();
	let mut console = console::Console::new([tsize[0], tsize[1]]);

	let _ = std::thread::spawn(|| client_handler(tx));

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
					let _utf8 = ch.encode_utf8(&mut buf).as_bytes();
					// TODO: send
				}
				_ => {}
			}
		}
		Event::RedrawRequested(_window_id) => {
			if let Ok(msg) = rx.try_recv() {
				console.handle_msg(msg);
			}
			let [tx, _] = console.get_size();
			model.faces = Vec::new();
			let (chars, cursor_pos) = console.render_data();
			for (idx, ch) in chars.iter().enumerate() {
				let idx = idx as u32;
				let offset_x = idx % tx;
				let offset_y = idx / tx;
				model.faces.extend(fc.text2fs(
					[offset_x, offset_y],
					std::iter::once(*ch),
					[1.0; 4],
					0,
				));
			}
			let mut modelref = rdr.insert_model(&model);
			modelref.set_z(1);
			_tmhandle = Some(modelref);

			// draw cursor
			let cursor_pos = [cursor_pos[0] as u32, cursor_pos[1] as u32];
			let x = (cursor_pos[0] * fsx) as f32;
			let y1 = (cursor_pos[1] * fsy) as f32;
			let y2 = ((cursor_pos[1] + 1) * fsy) as f32;
			let vs = vec![
				[x, y1, 0.0, 1.0],
				[x, y2, 0.0, 1.0],
				[x + 1.0, y1, 0.0, 1.0],
				[x + 1.0, y2, 0.0, 1.0],
			];
			let faces = vec![
				Face {
					vid: [0, 1, 2],
					color: [1.0; 4],
					uvid: [0; 3],
					layer: -1,
				},
				Face {
					vid: [3, 1, 2],
					color: [1.0; 4],
					uvid: [0; 3],
					layer: -1,
				},
			];
			let model = Model {
				vs,
				uvs: Vec::new(),
				faces,
			};
			let modelref = rdr.insert_model(&model);
			_cmhandle = Some(modelref);

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
