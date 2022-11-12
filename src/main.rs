mod console;
mod msg;
mod cursor;

use std::io::{Read, Write, BufWriter};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::mpsc::{channel, Receiver};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use winit::event::{KeyboardInput, ElementState};

use msg::VkotMsg;
use skey::Skey;
use skey::winit::WinitConversion;
use triangles::renderer::Renderer;
use triangles::bmtext::FontConfig;
use triangles::teximg::Teximg;

type Swriter = BufWriter<UnixStream>;
type Elp = EventLoopProxy<VkotMsg>;

fn sender_handler2(sw: &mut Swriter, msg: VkotMsg) -> std::io::Result<()> {
	match msg {
		VkotMsg::Getch(ch) => {
			let _ = sw.write(&[0])?;
			let _ = sw.write(&ch.to_le_bytes())?;
			sw.flush()?;
		},
		VkotMsg::Resized([tx, ty]) => {
			let _ = sw.write(&[1])?;
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

fn client_handler(proxy: Elp) {
	let _ = std::fs::remove_file("./vkot.socket");
	let listener = UnixListener::bind("./vkot.socket").unwrap();
	let mut buf = [0u8; 32768];
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
		proxy.send_event(VkotMsg::Stream(stream2)).unwrap();
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
	let (mut fc, img) = {
		let ssize = rdr.get_size();
		let img = Teximg::load("../fontdata/v1/unifont2_terminus.png");
		let fc = FontConfig::new(ssize, img.dim, [16, 16]).with_scaler(2);
		(fc, img)
	};
	rdr.upload_tex(img, 0);
	let mut model = fc.generate_model();
	let mut _tmhandle = None; // text model
	let mut _cmhandle = None; // cursor model

	let tsize = fc.get_terminal_size_in_char();
	let [fsx, fsy] = fc.get_scaled_font_size();
	let [fsx, fsy] = [fsx as i16, fsy as i16];
	let (tx, rx) = channel();
	let mut console = console::Console::new(
		[tsize[0] as i16, tsize[1] as i16]
	);

	let _ = std::thread::spawn(|| client_handler(proxy));
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
					let tsize = [tsize[0] as i16, tsize[1] as i16];
					console.resize(tsize);
					rdr.redraw();
					tx.send(VkotMsg::Resized(tsize)).unwrap();
				}
				WindowEvent::KeyboardInput {
					input: KeyboardInput {
						state: ElementState::Pressed,
						virtual_keycode: Some(vkc),
						..
					},
					..
				} => {
					if let Some(k) = Skey::from_wk(vkc) {
						let bytes = k.ser();
						tx.send(VkotMsg::Skey(bytes)).unwrap();
					}
				}
				WindowEvent::ReceivedCharacter(ch) => {
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

			let x1 = (cpos[0] * fsx) as f32;
			let x2 = (cpos[0] * fsx) as f32;
			let y1 = (cpos[1] * fsy) as f32;
			let y2 = ((cpos[1] + 1) * fsy) as f32;
			let ssize = rdr.get_size();
			let model = cursor::draw1([x1, y1, x2, y2], ssize);
			let modelref = rdr.insert_model(&model);
			_cmhandle = Some(modelref);

			rdr.render2();
		}
		Event::UserEvent(msg) => {
			match msg {
				VkotMsg::Stream(_) => {
					tx.send(msg).unwrap();
					let tsize = console.get_size();
					tx.send(VkotMsg::Resized(tsize)).unwrap();
				}
				_ => {
					console.handle_msg(msg);
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
