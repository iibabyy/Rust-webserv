use std::{
    collections::HashMap,
    fmt::format,
    io::{self, SeekFrom},
    net::IpAddr,
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{unix::SocketAddr, TcpListener, TcpStream},
};
use tokio_util::sync::CancellationToken;

use crate::{
    request::Request,
    response::response::{Response, ResponseCode},
    server::{
        server::Server,
        traits::{config::Config, handler::Handler},
    },
};

pub struct Listener {
    listener: TcpListener,
    servers: Vec<Server>,
    cancel_token: CancellationToken,
}

impl Listener {
    pub async fn init_listeners(
        addr: IpAddr,
        servers: HashMap<u16, Vec<Server>>,
        cancel_token: &CancellationToken,
    ) -> io::Result<Vec<Self>> {
        let mut listeners: Vec<Self> = Vec::new();

        for (port, vec) in servers {
            let listener = Self::new(addr, port, vec, cancel_token.clone()).await?;
            listeners.push(listener);
        }

        Ok(listeners)
    }

    pub async fn new(
        addr: IpAddr,
        port: u16,
        servers: Vec<Server>,
        cancel_token: CancellationToken,
    ) -> io::Result<Self> {
        let socket = format!("{}:{}", addr, port);

        let listener = match TcpListener::bind(socket).await {
            Ok(listener) => listener,
            Err(err) => return Err(err),
        };

        Ok(Listener {
            servers,
            listener,
            cancel_token,
        })
    }

    pub async fn listen(self) -> io::Result<()> {
        println!(
            "------[listener ({}): start listening]------",
            self.listener.local_addr().unwrap()
        );
        loop {
            let cancel = self.cancel_token.clone();
            tokio::select! {
                Ok((stream, addr)) = self.listener.accept() => {
                    println!("------[Connection accepted: {addr}]------");
                    let server_instance = self.servers.clone();
                    tokio::spawn( async move {
                        let _ = Self::handle_stream(stream, &server_instance).await;
                        // eprintln!("------[ End of Stream ]------");
                    });
                }
                _ = cancel.cancelled() => {
                    println!("------[listener ({}): stop listening]------", self.listener.local_addr().unwrap());
                    return Ok(());
                }
            }
        }
    }

    async fn handle_stream(mut stream: TcpStream, servers: &Vec<Server>) -> anyhow::Result<()> {
        let mut raw = String::new();
        let mut buffer = [0; 65536];

        loop {
            let n = match stream.read(&mut buffer).await {
                Err(err) => {
                    return Ok(eprintln!("Error: {} -> closing conection", err));
                }
                Ok(0) => break,
                Ok(n) => n,
            };

            raw.push_str(&String::from_utf8_lossy(&buffer[..n]));

            while let Some((header, raw_left)) = raw.split_once("\r\n\r\n") {
                eprintln!("\n\n\n----[ Incoming request ]----");
                raw = match Self::handle_request(
                    header,
                    &mut stream,
                    servers,
                    &mut raw_left.to_owned(),
                )
                .await
                {
                    Some(raw_left) => raw_left,
                    None => return Ok(()),
                }
            }
        }

        stream.shutdown().await?;

        Ok(())
    }

    async fn handle_request(
        header: &str,
        stream: &mut TcpStream,
        servers: &Vec<Server>,
        raw_left: &mut str,
    ) -> Option<String> {
        let request = match Request::try_from(header) {
            Ok(request) => request,
            Err(err) => {
                eprintln!("Error: deserializing header: {}", err.to_string());
                send_error_response(stream, err).await;
                return Some(raw_left.to_string());
                // send error response bad request
            }
        };

        let server = Self::choose_server_from(&request, servers);

        let raw_left = if let Some(location) = server.get_request_location(&request) {
            location.handle_request(request, stream, raw_left).await
        } else {
            server.handle_request(request, stream, raw_left).await
        };

        return raw_left;
    }

    fn choose_server_from<'a>(request: &Request, servers: &'a Vec<Server>) -> &'a Server {
        let mut default = None;

        // eprintln!("request:\n{request:#?}");

        if request.host().is_some() {
            let hostname = request.host().unwrap();
            for serv in servers {
                if serv.is_default() {
                    default = Some(serv)
                }
                if serv.name().is_none() {
                    continue;
                }

                let names = serv.name().unwrap();

                if names.iter().any(|name| name == hostname) {
                    return serv;
                }
            }
        }

        if default.is_some() {
            return default.unwrap();
        }

        return servers.first().unwrap();
    }
}

pub async fn send_error_response(stream: &mut TcpStream, code: ResponseCode) {
    let mut response = Response::new(code);

    let _ = response.send(stream).await;
}
