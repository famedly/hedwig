//! Probe of the server's own `/health` endpoint, so a container health check needs no curl.

use std::{
	io::{self, Read, Write},
	net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpStream},
	time::Duration,
};

use crate::settings::Server;

/// How long the probe waits for connecting, sending and the answer each.
const TIMEOUT: Duration = Duration::from_secs(2);

/// Asks the server configured in `server` for `GET /health`; `Ok` if it answers 200. An unspecified
/// bind address (`0.0.0.0`, `::`) is probed on loopback.
pub fn probe(server: &Server) -> io::Result<()> {
	let ip = match server.bind_address {
		IpAddr::V4(ip) if ip.is_unspecified() => IpAddr::V4(Ipv4Addr::LOCALHOST),
		IpAddr::V6(ip) if ip.is_unspecified() => IpAddr::V6(Ipv6Addr::LOCALHOST),
		ip => ip,
	};
	let mut stream = TcpStream::connect_timeout(&SocketAddr::new(ip, server.port), TIMEOUT)?;
	stream.set_read_timeout(Some(TIMEOUT))?;
	stream.set_write_timeout(Some(TIMEOUT))?;
	stream.write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")?;
	// The status line starts with "HTTP/1.1 200".
	let mut status = [0_u8; 12];
	stream.read_exact(&mut status)?;
	if status.starts_with(b"HTTP/1.") && status.ends_with(b" 200") {
		Ok(())
	} else {
		Err(io::Error::other(format!("/health answered {:?}", String::from_utf8_lossy(&status))))
	}
}
