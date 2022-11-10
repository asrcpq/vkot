// mod color_table;
mod console;
mod msg;

use std::io::{Read, Write, BufWriter};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::mpsc::{channel, Receiver, Sender};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};

use msg::VkotMsg;
use triangles::renderer::Renderer;
use triangles::bmtext::FontConfig;
use triangles::model::cmodel::{Face, Model};

type Swriter = BufWriter<UnixStream>;

fn sender_handler2(sw: &mut Swriter, msg: VkotMsg) -> std::io::Result<()> {
	match msg {
		VkotMsg::Getch(ch) => {
			let _ = sw.write(&[b'g'])?;
			let _ = sw.write(&ch.to_le_bytes())?;
			sw.flush()?;
		},
		VkotMsg::Resized([tx, ty]) => {
			let _ = sw.write(&[b'r'])?;
			let _ = sw.write(&tx.to_le_bytes())?;
			let _ = sw.write(&ty.to_le_bytes())?;
			sw.flush()?;
		},
		_ => panic!()
	}
	Ok(())
}

fn sender_handler(rx: Receiver<VkotMsg>) {
	let mut stream: Option<Swriter> = None;
	while let Ok(msg) = rx.recv() {
		if msg.is_s2c() {
			if let Some(stream) = stream.as_mut() {
				let _ = sender_handler2(stream, msg);
			}
			continue
		}
		match msg {
			VkotMsg::Stream(s) => {
				eprintln!("sender: update stream");
				let s = BufWriter::new(s);
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
	let mut bufv = Vec::new();
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
			bufv.extend(buf[..len].to_vec());
			let mut offset = 0;
			let msgs = VkotMsg::from_buf(&bufv, &mut offset).unwrap();
			bufv.drain(..offset);
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
	let [fsx, fsy] = [fsx as i32, fsy as i32];
	let (tx, rx) = channel();
	let mut console = console::Console::new(tsize);

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
					let ssize = rdr.get_size();
					fc.resize_screen(ssize);
					model = fc.generate_model();
					let tsize = fc.get_terminal_size_in_char();
					console.resize(tsize);
					rdr.redraw();
					tx.send(VkotMsg::Resized(ssize)).unwrap();
				}
				WindowEvent::ReceivedCharacter(ch) => {
					let mut buf = [0_u8; 4];
					let _utf8 = ch.encode_utf8(&mut buf).as_bytes();
					tx.send(VkotMsg::Getch(ch as u32)).unwrap();
				}
				_ => {}
			}
		}
		Event::RedrawRequested(_window_id) => {
			let cells = console.get_buffer();
			let cpos = console.get_cpos();
			model.faces = Vec::new();
			for (py, line) in cells.iter().enumerate() {
				for (px, cell) in line.iter().enumerate() {
					model.faces.extend(fc.text2fs(
						[px as u32, py as u32],
						std::iter::once(cell.ch),
						cell.color,
						0,
					));
				}
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
