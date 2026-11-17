//! Tests for the health probe's failures; its success is in `api.rs`, against the real server.

use std::{io::Write, net::TcpListener, thread};

use matrix_hedwig::{health::probe, settings::Server};

fn server(port: u16) -> Server {
	Server { port, bind_address: [127, 0, 0, 1].into() }
}

#[test]
fn probe_fails_on_a_non_200_answer() -> Result<(), Box<dyn std::error::Error>> {
	let listener = TcpListener::bind("127.0.0.1:0")?;
	let port = listener.local_addr()?.port();
	let answer = thread::spawn(move || -> std::io::Result<()> {
		let (mut stream, _) = listener.accept()?;
		stream.write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n")
	});

	assert!(probe(&server(port)).is_err());
	answer.join().map_err(|_| "answer thread panicked")??;
	Ok(())
}

#[test]
fn probe_fails_without_a_server() -> Result<(), Box<dyn std::error::Error>> {
	let port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();
	assert!(probe(&server(port)).is_err());
	Ok(())
}
