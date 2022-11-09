// mod color_table;
mod console;
mod msg;

use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::mpsc::{channel, Receiver, Sender};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};

use msg::VkotMsg;
use triangles::renderer::Renderer;
use triangles::bmtext::FontConfig;
use triangles::model::cmodel::{Face, Model};

fn sender_handler(rx: Receiver<VkotMsg>) {
	let mut stream: Option<UnixStream> = None;
	while let Ok(msg) = rx.recv() {
		match msg {
			VkotMsg::Getch(ch) => {
				let _ = stream.as_mut().unwrap().write(&[ch as u8]);
			},
			VkotMsg::Stream(s) => {
				eprintln!("sender: update stream");
				stream = Some(s);
			}
			_ => panic!(),
		}
	}
}

fn client_handler(proxy: EventLoopProxy<VkotMsg>, tx: Sender<VkotMsg>) {
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
		let stream2 = stream.try_clone().unwrap();
		eprintln!("client: update stream");
		tx.send(VkotMsg::Stream(stream2)).unwrap();
		loop {
			let len = match stream.read(&mut buf) {
				Ok(s) => s,
				Err(_) => break,
			};
			if len == 0 { break }
			let msgs = VkotMsg::from_buf(&buf[..len], &mut 0).unwrap();
			for msg in msgs.into_iter() {
				proxy.send_event(msg).unwrap();
			}
		}
		eprintln!("client: break");
	}
}

fn main() {
	let el = EventLoopBuilder::<VkotMsg>::with_user_event().build();
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

	let tx2 = tx.clone();
	let _ = std::thread::spawn(|| client_handler(proxy, tx2));
	let _ = std::thread::spawn(|| sender_handler(rx));

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
					tx.send(VkotMsg::Getch(ch)).unwrap();
				}
				_ => {}
			}
		}
		Event::RedrawRequested(_window_id) => {
			let [tx, _] = console.get_size();
			let cells = console.get_buffer();
			let cpos = console.get_cpos();
			model.faces = Vec::new();
			for (idx, cell) in cells.iter().enumerate() {
				let idx = idx as u32;
				let offset_x = idx % tx;
				let offset_y = idx / tx;
				model.faces.extend(fc.text2fs(
					[offset_x, offset_y],
					std::iter::once(cell.ch),
					cell.color,
					0,
				));
			}
			let mut modelref = rdr.insert_model(&model);
			modelref.set_z(1);
			_tmhandle = Some(modelref);

			// draw cursor
			let x = (cpos[0] * fsx) as f32;
			let y1 = (cpos[1] * fsy) as f32;
			let y2 = ((cpos[1] + 1) * fsy) as f32;
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
		Event::UserEvent(msg) => {
			console.handle_msg(msg);
			rdr.redraw();
		}
		Event::MainEventsCleared => {
			rdr.redraw();
			*ctrl = ControlFlow::Wait;
		}
		_ => {}
	})
}
