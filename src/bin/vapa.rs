use std::sync::mpsc::channel;

use vkot::term::{Elp, Vkot};

fn child_handler(proxy: Elp) {
	let (tx, rx) = channel();
	let shell = std::env::var("SHELL").unwrap();
	let _ = std::thread::spawn(|| vkot_client::apaterm::start(
		tx,
		vec![std::ffi::CString::new(shell).unwrap()],
	));
	let _ = rx.recv().unwrap();
	proxy.send_event(vkot::msg::VkotMsg::ChildExit).unwrap();
}

fn main() {
	let vkot = Vkot::new();
	let elp = vkot.elp.clone();
	let _ = std::thread::spawn(move || child_handler(elp));
	vkot.run();
}
