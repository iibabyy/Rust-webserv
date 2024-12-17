mod connection {
    use std::sync::{Arc, Mutex};

    use tokio::{io::BufReader, net::TcpStream};

	pub struct Connection {
		stream: Arc<Mutex<TcpStream>>,
	}

	impl Connection {
		pub fn new(stream: TcpStream) -> Self {
			Connection {
				stream: Arc::new(Mutex::new(stream)),
			}
		}

	}

}

mod listener {
    use std::{fmt::Debug, io, net::IpAddr, sync::{Arc, Mutex}};

    use nom::FindToken;
    use tokio::{io::{AsyncBufReadExt, AsyncReadExt}, net::{TcpListener, TcpStream}};
    use tokio_util::sync::CancellationToken;

    use crate::{request::{self, request::Request}, response::response::{Response, ResponseCode}, server::server::Server, traits::config::Config};

	pub struct Listener {
		listener: TcpListener,
		servers: Vec<Server>,
		cancel_token: CancellationToken,
	}

	impl Listener {
		pub async fn new(addr: IpAddr, port: u16, servers: Vec<Server>, cancel_token: CancellationToken) -> io::Result<Self> {
			let socket = format!("{}:{}", addr, port);

			let listener = match TcpListener::bind(socket).await {
				Ok(listener) => listener,
				Err(err) => return Err(err),
			};

			Ok(
				Listener {
					servers,
					listener,
					cancel_token,
				}
			)
		}

		pub async fn listen(self) -> io::Result<()> {

			loop {
				let cancel = self.cancel_token.clone();
				tokio::select! {
					Ok((stream, addr)) = self.listener.accept() => {
						println!("------[Connection accepted: {addr}]------");
						let server_instance = self.servers.clone();
						tokio::spawn( async move {
							Self::hande_connection(stream, server_instance);
						});
					}
					_ = cancel.cancelled() => {
						println!("------[listener ({:#?}): stop listening]------", self.listener.local_addr());
						return Ok(());
					}
				}
			}
		}

		async fn handle_stream(mut stream: TcpStream, servers: &Vec<Server>) -> anyhow::Result<()> {
			let mut buffer = [0; 65536];
			let mut raw = String::new();

			loop {
				let n = match stream.read(&mut buffer).await {
					Err(err) => {
						return Ok(eprintln!("Error: {} -> closing conection", err));
					}
					Ok(n) => n,
				};
				
				if n == 0 {
					return Ok(eprintln!("End of stream: closing conection"));
				}	// end of connection

				raw.push_str(std::str::from_utf8(&buffer[..n])?);

				let mut temp = None;
				while let Some((header, raw_left)) = raw.split_once("\r\n\r\n") {
					Self::handle_request(header, &mut stream, servers).await;
					temp = Some(raw_left);
				}
				
				if temp.is_some() { raw = temp.unwrap().to_owned() }

			}
		}

		async fn handle_request(header: &str, mut stream: &mut TcpStream, servers: &Vec<Server>) {
			let request = match Request::try_from(header) {
				Ok(request) => request,
				Err(err) => {
					eprintln!("Bad request: {err}");
					// send error response bad request
					todo!()
				}
			};

			let server = Self::choose_server_from(&request, servers);

			match server.parse_request(&request) {
				Ok(_) => (),
				Err(err) => {
					eprintln!("Error: {}", err.to_string());
					// TODO: send error response
					todo!()
				}
			}

			let response = server.final_path():

		}

		fn send_response(server: &Server, request: &Request) {
			let response = match server.build_response(request) {
				_ => todo!()
			};
			todo!()
		}

		fn choose_server_from<'a>(request: &Request, servers: &'a Vec<Server>) -> &'a Server {
			let mut default = None;

			if request.host().is_some() {
				let hostname = request.host().unwrap();
				for serv in servers {
					if serv.is_default() { default = Some(serv) }
					if serv.name().is_none() { continue }
					
					let names = serv.name().unwrap();
					
					if names.iter().any(|name| name == hostname) { return serv }
					
				}
			}

			if default.is_some() { return default.unwrap() }

			return servers.first().unwrap()

		}

	}
}
