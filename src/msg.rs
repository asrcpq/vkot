use std::os::unix::net::UnixStream;

#[derive(Debug)]
pub enum VkotMsg {
	Print(String),
	Getch(char),
	Stream(UnixStream),
}
