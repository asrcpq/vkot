use std::sync::mpsc::channel;
use std::ffi::CString;

use vkot::term::{Elp, Vkot};

fn child_handler(proxy: Elp) {
	let (tx, rx) = channel();
	let args = std::env::args()
		.skip(1)
		.map(|x| CString::new(x).unwrap())
		.collect::<Vec<CString>>();
	let args = if args.is_empty() {
		let shell = std::env::var("SHELL").unwrap();
		vec![CString::new(shell).unwrap()]
	} else {
		args
	};
	let _ = std::thread::spawn(|| vkot_client::apaterm::start(
		tx, args,
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
