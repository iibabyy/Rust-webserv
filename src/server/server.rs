/* ************************************************************************** */
/*                                                                            */
/*                                                        :::      ::::::::   */
/*   server.rs                                          :+:      :+:    :+:   */
/*                                                    +:+ +:+         +:+     */
/*   By: ibaby <ibaby@student.42.fr>                +#+  +:+       +#+        */
/*                                                +#+#+#+#+#+   +#+           */
/*   Created: 2024/12/15 05:34:36 by ibaby             #+#    #+#             */
/*   Updated: 2024/12/17 17:05:36 by ibaby            ###   ########.fr       */
/*                                                                            */
/* ************************************************************************** */

use std::{
    borrow::{BorrowMut, Cow},
    collections::HashMap,
    fmt::Debug,
    io::Error,
    net::{IpAddr, SocketAddr},
    os::fd::AsFd,
    path::{Path, PathBuf},
    sync::Arc,
};

use tokio::{
    fs::File,
    io::{self, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufStream},
    net::{TcpListener, TcpStream},
    stream,
    sync::Mutex,
};
use tokio_util::sync::CancellationToken;

use crate::{
    client::client::Client,
    request::request::{Method, Request, State},
    response::response::{Response, ResponseCode},
    traits::config::Config,
    Parsing::*,
};

use super::location::Location;

/*--------------------------[ SERVER ]--------------------------*/

#[derive(Clone, Debug)]
pub struct Server {
    internal: bool,
    default: bool,
    auto_index: bool,
    port: Option<u16>,
    path: PathBuf,
    socket: Option<SocketAddr>,
    max_body_size: Option<u64>,
    root: Option<PathBuf>,
    alias: Option<PathBuf>,
    index: Option<String>,
    return_: Option<(u16, Option<String>)>,
    name: Option<Vec<String>>,
    methods: Option<Vec<Method>>,
    error_pages: HashMap<u16, String>,
    error_redirect: HashMap<u16, (Option<u16>, String)>,
    infos: HashMap<String, Vec<String>>,
    cgi: HashMap<String, PathBuf>,
    locations: HashMap<PathBuf, Location>,
}

impl Config for Server {
    fn path(&self) -> &PathBuf/*----------------------------------*/{ &self.path }
	fn internal(&self) -> bool/*----------------------------------*/{ self.internal }
    fn auto_index(&self) -> bool/*--------------------------------*/{ self.auto_index }
    fn is_location(&self) -> bool/*-------------------------------*/{ false }
    fn port(&self) -> Option<&u16>/*------------------------------*/{ self.port.as_ref() }
    fn index(&self) -> Option<&String>/*--------------------------*/{ self.index.as_ref() }
    fn root(&self) -> Option<&PathBuf>/*--------------------------*/{ self.root.as_ref() }
	fn alias(&self) -> Option<&PathBuf>/*-------------------------*/{ self.alias.as_ref() }
    fn name(&self) -> Option<&Vec<String>>/*----------------------*/{ self.name.as_ref() }
    fn max_body_size(&self) -> Option<&u64>/*---------------------*/{ self.max_body_size.as_ref() }
    fn methods(&self) -> Option<&Vec<Method>>/*-------------------*/{ self.methods.as_ref() }
    fn cgi(&self) -> &HashMap<String, PathBuf>/*------------------*/{ &self.cgi }
    fn error_pages(&self) -> &HashMap<u16, String>/*--------------*/{ &self.error_pages }
    fn return_(&self) -> Option<&(u16, Option<String>)>/*---------*/{ self.return_.as_ref() }
    fn locations(&self) -> Option<&HashMap<PathBuf, Location>>/*--*/{ Some(&self.locations) }
    fn error_redirect(&self) -> &HashMap<u16, (Option<u16>, String)>{ &self.error_redirect }
}

/*------------------------------------------------------------------------------------------------------*/
/*												SERVER													*/
/*------------------------------------------------------------------------------------------------------*/

impl Server {
    // pub async fn run(mut self, ip: IpAddr, cancel_token: CancellationToken) -> Result<(), ()> {
    //     if self.port.is_none() {
    //         println!("------[No port to listen -> no bind]------");
    //         return Ok(());
    //     }

    //     self.socket = Some(SocketAddr::new(ip, self.port.unwrap()));
    //     let listener = match TcpListener::bind(self.socket.unwrap().clone()).await {
    //         Ok(listener) => listener,
    //         Err(e) => {
    //             eprintln!("Server ({}): failed to bind: {e}", self.socket.unwrap());
    //             return Err(());
    //         }
    //     };

    //     println!(
    //         "------[Server listening on {ip}::{}]------",
    //         self.port.unwrap()
    //     );
    //     let server = Arc::new(self);

    //     loop {
    //         let cancel = cancel_token.clone();
    //         tokio::select! {
    //             Ok((stream, addr)) = listener.accept() => {
    //                 println!("------[Connection accepted: {addr}]------");
    //                 let server_instance = Arc::clone(&server);
    //                 tokio::spawn( async move {
    //                     server_instance.handle_client(stream).await
    //                 });
    //             }
    //             _ = cancel.cancelled() => {
    //                 println!("------[Server ({}): stop listening]------", server.socket.unwrap());
    //                 return Ok(());
    //             }
    //         }
    //     }
    // }

    // async fn handle_client(&self, mut stream: TcpStream) -> Result<(), Error> {
    //     //	getting request
    //     loop {
    //         let request = match self.read_until_header_complete(&mut stream).await {
    //             Ok(request) => request,
    //             Err(err) => {
    //                 if err.is_none() {
    //                     // Request Parsing Error
    //                     eprintln!("invalid header !");
    //                     self.send_error_response_to(&mut stream, ResponseCode::new(400)).await?;
    //                 } else {
    //                     // i/o Error
    //                     let err = err.unwrap();
    //                     eprintln!("failed to read header !");
    //                     self.send_error_response_to(
    //                         &mut stream,
    //                         ResponseCode::from_error(err.kind()),
    //                     )
    //                     .await?;
    //                 }
    //                 continue;
    //             }
    //         };

	// 		eprintln!("---[ {:#?}", request);

    //         match if let Some(location) = self.get_location(request.path()) {
    //             eprintln!("---[ LOCATION");
    //             location.send_response(&request, &mut stream).await
    //         } else {
    //             eprintln!("---[ SERVER");
	// 			// eprintln!("---[ {:#?}", request);
    //             self.send_response(&request, &mut stream).await
    //         } {
    //             Ok(_) => (),
    //             Err(err) => {
    //                 eprintln!("failed to send response: {} !", err.to_string());
    //                 self.send_error_response_to(&mut stream, err).await?;
    //                 continue;
    //             }
    //         }

    //         if request.keep_connection_alive() == false {
    //             break;
    //         }
    //     }

    //     Ok(())
    // }

    fn get_location(&self, to_find: &PathBuf) -> Option<&Location> {
        let mut save: Option<(&PathBuf, &Location)> = None;

        for (path, location) in &self.locations {
            if path.is_absolute() {
                if to_find
                    .to_str()
                    .unwrap()
                    .starts_with(path.to_str().unwrap())
                    == false
                {
                    continue;
                }
            } else {
                if to_find.iter().any(|filename| filename.eq(path)) == false {
                    continue;
                }
            }

            if save.is_none() {
                save = Some((path, location))
            } else if save.unwrap().0 < path {
                save = Some((path, location))
            }
        }

        if save.is_none() {
            None
        } else {
            Some(save.unwrap().1)
        }
    }

    // async fn read_until_header_complete(
    //     &self,
    //     mut stream: &mut TcpStream,
    // ) -> Result<Request, Option<Error>> {
    //     let buffer = match Self::read_header(&mut stream).await {
	// 		Ok(headers) => headers,
	// 		Err(err) => return Err(Some(err)),
	// 	};
    //     let mut request = match Request::try_from(buffer) {
    //         Ok(request) => request,
    //         Err(_) => {
    //             println!("Error: bad request");
    //             // self.send_error_response_to(&mut stream);
    //             return Err(None);
    //         }
    //     };

    //     while request.state().is(State::OnHeader) {
    //         let buffer = Self::read_from(&mut stream).await?;

    //         match request.push(buffer) {
    //             Ok(_) => (),
    //             Err(_) => {
    //                 println!("Error: bad request");
    //                 return Err(None);
    //             }
    //         }
    //     }

    //     Ok(request)
    // }

	async fn read_header(stream: &mut TcpStream) -> io::Result<Vec<String>> {
        let reader = BufReader::new(stream);
		let mut lines = reader.lines();
		let mut header = Vec::new();

		while let Some(line) = lines.next_line().await? {
			if line.is_empty() { return Ok(header) }
			header.push(line);
		}

		Err(io::Error::new(std::io::ErrorKind::FileTooLarge, "header too large"))
	}

    async fn consume_body(&self, stream: &mut TcpStream) -> Result<(), Error> {
        let mut buffer = [0; 65536];

        loop {
            match stream.try_read(&mut buffer) {
                Ok(0) => break,
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    println!("would block !");
                    continue;
                }
                Err(e) => {
                    println!("failed to read: {e}");
                    return Err(e);
                }
                _ => (),
            }
        }
        Ok(())
    }
}

/*---------------------------------------------------------------*/
/*							  UTILS								 */
/*---------------------------------------------------------------*/

#[allow(dead_code)]
impl Server {
    pub fn new(config: ServerBlock) -> Result<Self, String> {
        let mut serv = Server {
            port: None,
            socket: None,
            root: None,
            alias: None,
            path: PathBuf::from("/"),
            max_body_size: None,
            index: None,
            methods: None,
            return_: None,
            auto_index: false,
            error_pages: HashMap::new(),
            error_redirect: HashMap::new(),
            infos: HashMap::new(),
            locations: HashMap::new(),
            cgi: config.cgi,
            default: false,
            name: None,
            internal: false,
        };

        for directive in config.directives {
            serv.add_directive(directive.0, directive.1)?;
        }

        for location in config.locations {
            serv.add_location(location.1)?;
        }

        Ok(serv)
    }

    // async fn read_from(mut stream: impl AsyncRead + Unpin) -> Result<String, Error> {
    //     let mut buffer = [0; 65536];

    //     match stream.read(&mut buffer).await {
    //         Ok(n) => Ok(String::from_utf8_lossy(&buffer[..n]).into_owned()),
    //         Err(e) => Err(e),
    //     }
    // }

    // async fn create_request_from(&mut self, stream: &mut TcpStream) -> Result<Request, ()> {
    // 	let mut buffer = [0;65536];

    // 	let buffer = match stream.read(&mut buffer).await {
    // 		Ok(n) => String::from_utf8_lossy(&buffer[..n]).into_owned(),
    // 		Err(_) => return Err(()),
    // 	};

    // 	match Request::try_from(buffer) {
    // 		Ok(request) => Ok(request),
    // 		Err(_) => Err(())
    // 	}

    // }
}

/*---------------------------------------------------------------*/
/*----------------------[ CONFIG PARSING ]-----------------------*/
/*---------------------------------------------------------------*/

impl Server {

    pub fn init_servers(configs: Vec<ServerBlock>) -> Result<Vec<Self>, String> {
        let mut servers = Vec::new();

        for server_config in configs {
            servers.push(Self::new(server_config)?);
        }

        Ok(servers)
    }

    fn add_directive(&mut self, name: String, infos: Vec<String>) -> Result<(), String> {
        match name.as_str() {
            "root" => {
                if self.alias.is_some() {
                    return Err(format!(
                        "invalid field: root: root cannot be set with alias"
                    ));
                } else {
                    self.root = Some(Self::extract_root(infos)?)
                }
            }
            "alias" => {
                if self.root.is_some() {
                    return Err(format!(
                        "invalid field: alias: alias cannot be set with root"
                    ));
                } else {
                    self.alias = Some(Self::extract_alias(infos)?)
                }
            }
            "listen" => {
                (self.port, self.default) = Self::extract_listen(infos)?;
            }
            "server_name" | "server_names" => {
                if infos.len() < 1 {
                    return Err("invalid field: server_name".to_owned());
                } else {
                    if self.name.is_none() {
                        self.name = Some(Vec::new())
                    }

                    self.name.as_mut().unwrap().append(&mut infos.clone());
                }
            }
            "index" => {
                self.index = Some(Self::extract_index(infos)?);
            }
            "auto_index" => {
                self.auto_index = Self::extract_auto_index(infos)?;
            }
            "client_max_body_size" => {
                self.max_body_size = Some(Self::extract_max_body_size(infos)?);
            }
            "cgi" => {
                let (extension, path) = Self::extract_cgi(infos)?;
                self.cgi.insert(extension, path);
            }
            "allowed_methods" => {
                if infos.len() < 1 {
                    return Err("invalid field: allowed_methods".to_owned());
                }

                let methods: Vec<Result<Method, String>> = infos
                    .clone()
                    .iter()
                    .map(|method| Method::try_from_str(&method[..]))
                    .collect();

                if methods.iter().any(|res| res.is_err()) {
                    return Err(format!("invalid field: allowed_methods: unknown method"));
                }

                if self.methods.is_none() {
                    self.methods = Some(Vec::new())
                }
				
				let mut method = methods
					.iter()
					.map(|method| method.as_ref().unwrap().to_owned())
					.collect::<Vec<Method>>();
				self.methods.as_mut().unwrap().append(&mut method);

				println!("METHOD ADDED: {:#?}", self.methods);

            }
            "return" => {
                self.return_ = Some(Self::extract_return(infos)?);
            }
            "error_page" => {
                let (pages, redirect) = Self::extract_error_page(infos)?;
                let hash = &mut self.error_pages;
                if pages.is_some() {
                    pages
                        .unwrap()
                        .iter()
                        .map(|(code, url)| hash.insert(code.to_owned(), url.to_owned()))
                        .last();
                }
                let hash = &mut self.error_redirect;
                if redirect.is_some() {
                    redirect
                        .unwrap()
                        .iter()
                        .map(|(code, url)| hash.insert(code.to_owned(), url.to_owned()))
                        .last();
                }
            }
            "internal" => {
                self.internal = true;
            }
            _ => {
                self.infos.insert(name, infos);
            }
        }
        Ok(())
    }

    fn add_location(&mut self, location: LocationBlock) -> Result<(), String> {
        let new_location = Location::new(location, &self)?;

        self.locations
            .insert(new_location.path().clone(), new_location);

        Ok(())
    }
}

/*---------------------------------------------------------------*/
/*----------------------[ GETTER / SETTER ]----------------------*/
/*---------------------------------------------------------------*/

#[allow(dead_code)]
impl Server {
    pub fn is_default(&self) -> bool {
        self.default
    }

    pub fn get(&self, info: String) -> Option<String> {
        Some(self.infos.get(&info)?.join(" "))
    }
}

/*-------------------------------------------------------------------------------------------------------*/
