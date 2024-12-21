/* ************************************************************************** */
/*																			*/
/*														:::	  ::::::::   */
/*   server.rs										  :+:	  :+:	:+:   */
/*													+:+ +:+		 +:+	 */
/*   By: ibaby <ibaby@student.42.fr>				+#+  +:+	   +#+		*/
/*												+#+#+#+#+#+   +#+		   */
/*   Created: 2024/12/15 05:34:36 by ibaby			 #+#	#+#			 */
/*   Updated: 2024/12/20 11:43:12 by ibaby			###   ########.fr	   */
/*																			*/
/* ************************************************************************** */

use std::{collections::HashMap, net::SocketAddr, path::PathBuf};

use crate::{request::Method, LocationBlock, ServerBlock};

use super::{config::Config, handler::Handler, location::Location, parsing};

/*---------------------------------------------------------------*/
/*-------------------------[ SERVER ]----------------------------*/
/*---------------------------------------------------------------*/

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Server {
    internal: bool,
    default: bool,
    auto_index: bool,
    port: Option<u16>,
    path: PathBuf,
    socket: Option<SocketAddr>,
    max_body_size: Option<usize>,
    root: Option<PathBuf>,
    upload_folder: Option<PathBuf>,
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

impl Handler for Server {}

impl Config for Server {
    fn path(&self) -> &PathBuf /*----------------------------------*/ {
        &self.path
    }
    fn internal(&self) -> bool /*----------------------------------*/ {
        self.internal
    }
    fn auto_index(&self) -> bool /*--------------------------------*/ {
        self.auto_index
    }
    fn is_location(&self) -> bool /*-------------------------------*/ {
        false
    }
    fn port(&self) -> Option<&u16> /*------------------------------*/ {
        self.port.as_ref()
    }
    fn index(&self) -> Option<&String> /*--------------------------*/ {
        self.index.as_ref()
    }
    fn root(&self) -> Option<&PathBuf> /*--------------------------*/ {
        self.root.as_ref()
    }
    fn alias(&self) -> Option<&PathBuf> /*-------------------------*/ {
        None
    }
    fn name(&self) -> Option<&Vec<String>> /*----------------------*/ {
        self.name.as_ref()
    }
    fn methods(&self) -> Option<&Vec<Method>> /*-------------------*/ {
        self.methods.as_ref()
    }
    fn max_body_size(&self) -> Option<&usize> /*---------------------*/ {
        self.max_body_size.as_ref()
    }
    fn cgi(&self) -> &HashMap<String, PathBuf> /*------------------*/ {
        &self.cgi
    }
    fn upload_folder(&self) -> Option<&PathBuf> /*-----------------*/ {
        self.upload_folder.as_ref()
    }
    fn error_pages(&self) -> &HashMap<u16, String> /*--------------*/ {
        &self.error_pages
    }
    fn return_(&self) -> Option<&(u16, Option<String>)> /*---------*/ {
        self.return_.as_ref()
    }
    fn locations(&self) -> Option<&HashMap<PathBuf, Location>> /*--*/ {
        Some(&self.locations)
    }
    fn error_redirect(&self) -> &HashMap<u16, (Option<u16>, String)> {
        &self.error_redirect
    }
}

/*---------------------------------------------------------------*/
/*--------------------------[ UTILS ]----------------------------*/
/*---------------------------------------------------------------*/

#[allow(dead_code)]
impl Server {
    pub fn new(config: ServerBlock) -> Result<Self, String> {
        let mut serv = Server {
            port: None,
            socket: None,
            root: None,
            upload_folder: None,
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
    //	 let mut buffer = [0; 65536];

    //	 match stream.read(&mut buffer).await {
    //		 Ok(n) => Ok(String::from_utf8_lossy(&buffer[..n]).into_owned()),
    //		 Err(e) => Err(e),
    //	 }
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
                if self.root.is_some() {
                    println!("Warning: root: duplicated value")
                };
                self.root = Some(parsing::extract_root(infos)?)
            }
            "alias" => {
                return Err(format!(
                    "invalid field: alias: alias can only be set in locations"
                ));
            }
            "upload_folder" => self.upload_folder = Some(parsing::extract_upload_folder(infos)?),
            "listen" => {
                (self.port, self.default) = parsing::extract_listen(infos)?;
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
                self.index = Some(parsing::extract_index(infos)?);
            }
            "auto_index" => {
                self.auto_index = parsing::extract_auto_index(infos)?;
            }
            "client_max_body_size" => {
                self.max_body_size = Some(parsing::extract_max_body_size(infos)?);
            }
            "cgi" => {
                let (extension, path) = parsing::extract_cgi(infos)?;
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
            }
            "return" => {
                self.return_ = Some(parsing::extract_return(infos)?);
            }
            "error_page" => {
                let (pages, redirect) = parsing::extract_error_page(infos)?;
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
