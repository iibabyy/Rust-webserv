use std::{collections::HashMap, io::{self, Error}, net::{IpAddr, Shutdown}};

use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}};
use tokio_util::sync::CancellationToken;

use crate::{request::Request, response::response::{Response, ResponseCode}, server::{config::Config, server::Server}};

pub struct Listener {
	listener: TcpListener,
	servers: Vec<Server>,
	cancel_token: CancellationToken,
}

impl Listener {

	pub async fn init_listeners(addr: IpAddr, servers: Vec<Server>, cancel_token: &CancellationToken) -> io::Result<Vec<Self>> {
		let mut listeners: HashMap<u16, Self> = HashMap::new();

		for serv in servers {
			if serv.port().is_none() {
				eprintln!("Warning: a server don't have port to listen on, he will be ignored");
				continue
			}

			let port = serv.port().unwrap();

			if listeners.contains_key(port) {
				let vec = listeners.get_mut(port).unwrap();
				let listener_servs: &mut Vec<Server> = vec.servers.as_mut();
				listener_servs.push(serv);
			} else {
				listeners.insert(
					port.to_owned(),
					Self::new(addr.clone(), port.to_owned(), vec![serv], cancel_token.clone()).await?
				);
			}
		}

		Ok(listeners.into_values().collect())
	}

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
						let _ = Self::handle_stream(stream, &server_instance).await;
						eprintln!("------[ End of Stream ]------");
					});
				}
				_ = cancel.cancelled() => {
					println!("------[listener ({:#?}): stop listening]------", self.listener.local_addr().unwrap());
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
				Ok(0) => { break },
				Ok(n) => n,
			};

			raw.push_str(std::str::from_utf8(&buffer[..n])?);

			while let Some((header, raw_left)) = raw.split_once("\r\n\r\n") {
				eprintln!("\n\n\nIncoming request");
				raw = match Self::handle_request(header, &mut stream, servers, raw_left).await {
					Some(raw_left) => raw_left,
					None => return Ok(())
				}
			}

		}

		stream.shutdown().await?;

		Ok(())
	}

	async fn handle_request(header: &str, stream: &mut TcpStream, servers: &Vec<Server>, raw_left: &str) -> Option<String> {
		let mut request = match Request::try_from(header) {
			Ok(request) => request,
			Err(err) => {
				Self::send_error_response(stream, err).await;
				return Some(raw_left.to_string())
				// send error response bad request
			}
		};

		let server = Self::choose_server_from(&request, servers);

		match server.parse_request(&mut request) {
			Ok(_) => (),
			Err(err) => {
				eprintln!("Error: {}", err.to_string());
				Self::send_error_response(stream, err).await;
				return if request.keep_connection_alive() == true { Some(raw_left.to_string()) } else { None }
			}
		}

		let raw_left = match server.handle_body_request(&request, stream, raw_left).await {
			Ok(raw_left) => raw_left,
			Err(err) => {
				println!("Error: {}", err.to_string());
				Self::send_error_response(stream, err).await;
				return if request.keep_connection_alive() == true { Some(raw_left.to_string()) } else { None }
			}
		};

		match Self::send_response(server, stream, &request).await {
			Ok(_) => (),
			Err(err) => {
				println!("Error: {err}");
				Self::send_error_response(stream, ResponseCode::from_error(&err)).await;
				return if request.keep_connection_alive() == true { Some(raw_left.to_string()) } else { None }
			}
		}

		Some(raw_left)

	}

	async fn send_error_response(stream: &mut TcpStream, code: ResponseCode) {
		let mut response = Response::new(code);

		let _ = response.send(stream).await;

	}

	async fn send_response(server: &Server, stream: &mut TcpStream, request: &Request) -> Result<(), Error> {
		let mut response =  
		if let Some(location) = server.get_request_location(request) {
			location.build_response(request).await?
		} else {
			server.build_response(request).await?
		};

		response.send(stream).await

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
