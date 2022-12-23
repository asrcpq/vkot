use std::io::{Read, Write, BufWriter};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::mpsc::{channel, Receiver};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy};

use crate::cursor;
use crate::msg::VkotMsg;
use crate::console::Console;
use skey::Skey;
use skey::winit::{WinitConversion, WinitModifier};
use skey::modtrack::ModifierTracker;
use ttri::renderer::Renderer;
use ttri::model::model_ref::ModelRef;
use ttri::teximg::Teximg;
use ttri_mono::bmtext::FontConfig;

type Swriter = BufWriter<UnixStream>;
pub type Elp = EventLoopProxy<VkotMsg>;

fn uc(uc: u32) -> [f32; 4] {
	let bs = uc.to_be_bytes();
	core::array::from_fn(|idx| bs[idx] as f32 / 255.0)
}

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
		VkotMsg::Skey(ch) => {
			let _ = sw.write(&[2])?;
			let _ = sw.write(&ch)?;
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

fn client_handler(listener: UnixListener, proxy: Elp) {
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
			// TODO: avoid heap allocation of msgs, return an iterator
			let msgs = match VkotMsg::from_buf(&bufv, &mut offset) {
				Ok(msgs) => msgs,
				Err(e) => {
					eprintln!("{:?}", e);
					Vec::new()
				},
			};
			bufv.drain(..offset);
			for msg in msgs.into_iter() {
				proxy.send_event(msg).unwrap();
			}
		}
		eprintln!("client: break");
	}
}

pub struct Vkot {
	pub elp: EventLoopProxy<VkotMsg>,
	el: Option<EventLoop<VkotMsg>>,
	listener: Option<UnixListener>,
	rdr: Renderer,
	fc: FontConfig,

	text_model: Option<ModelRef>,
	textbg_model: Option<ModelRef>,
	cursor_model: Option<ModelRef>,
}

impl Vkot {
	pub fn new() -> Self {
		let el = EventLoopBuilder::<VkotMsg>::with_user_event().build();
		let mut rdr = Renderer::new(&el);
		let elp = el.create_proxy();
		// TODO: multiple sockets
		let socket_path = std::path::Path::new("./vkot.socket");
		let _ = std::fs::remove_file(socket_path);
		std::env::set_var("VKOT_SOCKET", socket_path);
		let listener = UnixListener::bind(socket_path).unwrap();
		let (fc, img) = {
			let ssize = rdr.get_size();
			//let img = Teximg::load("../fontdata/v1/unifont2_terminus.png", false);
			//let fc = FontConfig::new(ssize, img.dim, [16, 16]).with_scaler(2);
			let img = Teximg::load("../fontdata/v1/unifont3_terminus_32.png", false);
			let fc = FontConfig::new(ssize, img.dim, [32, 32]);
			(fc, img)
		};
		rdr.upload_tex(img, 0);
		Self {
			elp,
			el: Some(el),
			listener: Some(listener),
			rdr,
			fc,

			text_model: None,
			textbg_model: None,
			cursor_model: None,
		}
	}

	pub fn run(mut self) {
		let [mut model, mut bmodel] = self.fc.generate_models();
	
		let tsize = self.fc.get_terminal_size_in_char();
		let [fsx, fsy] = self.fc.get_scaled_font_size();
		let [fsx, fsy] = [fsx as i16, fsy as i16];
		let (tx, rx) = channel();
		let mut console = Console::new(
			[tsize[0] as i16, tsize[1] as i16]
		);
	
		let listener = self.listener.take().unwrap();
		let elp = self.elp.clone();
		let _ = std::thread::spawn(move || client_handler(listener, elp));
		let _ = std::thread::spawn(move || sender_handler(rx));
	
		let mut modtrack = ModifierTracker::default();
		self.el
			.take()
			.unwrap()
			.run(move |event, _, ctrl| match event
		{
			Event::WindowEvent { event: e, .. } => {
				match e {
					WindowEvent::CloseRequested => {
						*ctrl = ControlFlow::Exit;
					}
					WindowEvent::Resized(_) => {
						self.rdr.damage();
						let ssize = self.rdr.get_size();
						self.fc.resize_screen(ssize);
						[model, bmodel] = self.fc.generate_models();
						let tsize = self.fc.get_terminal_size_in_char();
						let tsize = [tsize[0] as i16, tsize[1] as i16];
						console.resize(tsize);
						self.rdr.redraw();
						tx.send(VkotMsg::Resized(tsize)).unwrap();
					}
					WindowEvent::ModifiersChanged(modifiers) => {
						let ks = modtrack.update_state(modifiers);
						for k in ks.into_iter() {
							let bytes = k.ser();
							tx.send(VkotMsg::Skey(bytes)).unwrap();
						}
					}
					WindowEvent::KeyboardInput {
						input,
						..
					} => {
						if let Some(k) = Skey::from_wki(input) {
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
				let buffer = console.get_buffer();
				let tsize = console.get_size();
				let cpos = console.get_cpos();
				model.faces = Vec::new();
				bmodel.faces = Vec::new();
				for py in 0..tsize[1] as usize {
					for px in 0..tsize[0] as usize {
						match self.fc.char(
							[px as u32, py as u32],
							buffer[py][px].ch,
							uc(buffer[py][px].fg),
							uc(buffer[py][px].bg),
							0,
						) {
							Ok([face0, face1, face2, face3]) => {
								model.faces.push(face0);
								model.faces.push(face1);
								bmodel.faces.push(face2);
								bmodel.faces.push(face3);
							}
							Err(e) => eprintln!("ERROR {:?}", e),
						}
					}
				}
				let mut modelref = self.rdr.insert_model(&model);
				modelref.set_z(2);
				self.text_model= Some(modelref);
				let mut modelref = self.rdr.insert_model(&bmodel);
				modelref.set_z(1);
				self.textbg_model = Some(modelref);
	
				let x1 = (cpos[0] * fsx) as f32;
				let x2 = (cpos[0] * fsx) as f32;
				let y1 = (cpos[1] * fsy) as f32;
				let y2 = ((cpos[1] + 1) * fsy) as f32;
				let ssize = self.rdr.get_size();
				let model = cursor::draw1([x1, y1, x2, y2], ssize);
				let mut modelref = self.rdr.insert_model(&model);
				modelref.set_z(0);
				self.cursor_model = Some(modelref);
	
				self.rdr.render_s();
			}
			Event::UserEvent(msg) => {
				match msg {
					VkotMsg::Stream(_) => {
						tx.send(msg).unwrap();
						let tsize = console.get_size();
						tx.send(VkotMsg::Resized(tsize)).unwrap();
					}
					VkotMsg::ChildExit => {
						*ctrl = ControlFlow::Exit
					}
					_ => {
						console.handle_msg(msg);
						self.rdr.redraw();
					}
				}
			}
			Event::MainEventsCleared => {
				self.rdr.redraw();
				*ctrl = ControlFlow::Wait;
			}
			_ => {}
		})
	}
}
