use std::{collections::HashMap, io::{self, Error, ErrorKind}, path::PathBuf};

use tokio::fs::File;

use crate::{
    request::request::{Method, Request},
    response::response::{Response, ResponseCode},
    server::location::Location,
};

#[allow(dead_code)]
pub trait Config {
 
	/*------------------------------------------------------------*/
	/*-----------------------[ GETTERS ]--------------------------*/
	/*------------------------------------------------------------*/

    fn auto_index(&self) -> bool;
    fn root(&self) -> Option<&PathBuf>;
    fn alias(&self) -> Option<&PathBuf>;
    fn port(&self) -> Option<&u16>;
    fn index(&self) -> Option<&String>;
    fn max_body_size(&self) -> Option<&u64>;	//-
    fn name(&self) -> Option<&Vec<String>>;
    fn path(&self) -> &PathBuf;
    fn methods(&self) -> Option<&Vec<Method>>;
    fn cgi(&self) -> &HashMap<String, PathBuf>;	//-
    fn error_pages(&self) -> &HashMap<u16, String>;
    fn error_redirect(&self) -> &HashMap<u16, (Option<u16>, String)>;
    fn locations(&self) -> Option<&HashMap<PathBuf, Location>>;
    fn return_(&self) -> Option<&(u16, Option<String>)>;
    fn internal(&self) -> bool;
    fn is_location(&self) -> bool;

	/*------------------------------------------------------------*/
	/*-----------------------[ Response ]-------------------------*/
	/*------------------------------------------------------------*/

	async fn build_response(&self, request: &Request) -> Result<Response, Error> {
		eprintln!("Building response...");
		match request.method() {
			&Method::GET => { return self.build_get_response(request).await },
			_ => return Err(Error::new(ErrorKind::Other, "method not implemented")), // not implemented
		}
	}

	async fn build_get_response(&self, request: &Request) -> Result<Response, Error> {
		eprintln!("Building GET response for '{}'...", request.path().display());
		// todo! list files if no auto_index
		// todo! handle GET on cgi

		let file = self.get_GET_request_file(request).await?;

		eprintln!("GET response build...");
		let mut response = Response::
			new(ResponseCode::default())
			.file(file);

		response.add_header("Content-Type".to_owned(), "text/html".to_owned());

		Ok(response)

	}

	#[allow(non_snake_case)]
	async fn get_GET_request_file(&self, request: &Request) -> io::Result<File> {

		eprintln!("trying to open '{}'...", request.path().display());
		if request.path().is_file() {
			match File::open(request.path()).await {
				Ok(file) => Ok(file),
				Err(err) => Err(err),
			}
		} else if request.path().is_dir() {
			// TODO!: List directory
			todo!("List directory");
		} else {
			Err(Error::new(ErrorKind::NotFound, "file not found"))
		}
	}

	/*------------------------------------------------------------*/
	/*-----------------------[ Parsing ]--------------------------*/
	/*------------------------------------------------------------*/

	fn parse_request(&self, request: &mut Request) -> Result<(), ResponseCode> {
		if let Some(location) = self.get_request_location(request) {
			eprintln!("Location...");
			return location.parse_request(request);
		}
		eprintln!("Parsing request...");
		
		self.parse_method(request)?;

		if request.content_length()  > self.max_body_size() {
			return Err(ResponseCode::from_code(413));
		}

		self.format_path(request)?;

		Ok(())
	}

	fn format_path(&self, request: &mut Request) -> Result<(), ResponseCode> {

		self.add_root_or_alias(request)?;
		self.add_index_if_needed(request)?;
		// let new_path = 
		Ok(())
	}

	fn add_index_if_needed(&self, request: &mut Request) -> Result<(), ResponseCode> {

		if request.path().is_dir() == false { return Ok(()) }	// not a dir -> no index needed

		if self.auto_index() == true {
			if self.index().is_some() {
				let path = request.path().to_str().unwrap();
				let path = format!(
					"{}/{}",
					path,
					self.index().unwrap(),
				);

				let path = PathBuf::from(path);
				request.set_path(path);

			} else {	// auto index on but no index
				return Err(ResponseCode::from_code(404));
			}
		}
		
		// TODO!: later in the code, when building response, check if directory, and list the directory if it's one

		Ok(())
	}

	fn add_root_or_alias(&self, request: &mut Request) -> Result<(), ResponseCode> {
		let path = if self.root().is_some() {
			let mut path = self.root().unwrap().clone();
			path = PathBuf::from(format!(
				"{}/{}", path.to_str().unwrap(),
				request.path().to_str().unwrap(),
			));

			path
		} else if self.alias().is_some() {
			let path = request.path().to_str().unwrap().to_string();
			let path = path.replacen(self.path().to_str().unwrap(), self.alias().unwrap().to_str().unwrap(), 1);
			eprintln!(
				"[\n\tAlias: {} -> {}\n\tpath: {}\n\t result: {}\n]",
				self.path().display(),
				self.alias().unwrap().display(),
				request.path().display(),
				path,
			);
			
			PathBuf::from(path)
		} else { return Err(ResponseCode::from_code(404)) };	// no root nor alias

		request.set_path(path);
		Ok(())
	}

	fn is_cgi(&self, request: &Request) -> bool {
		let request_extension = request.path().extension();

		if request_extension.is_none() { return false }

		let request_extension = match request_extension.unwrap().to_str() {
			Some(extension)	=> extension,
			None 					=> return false,
		};

		if self.cgi().contains_key(request_extension) {
			true
		} else {
			false
		}
	}

    fn parse_method(&self, request: &Request) -> Result<(), ResponseCode> {
        let methods = self.methods();
		if methods.is_none() {
			eprintln!("NO METHODS ALLOWED !");
            return Err(ResponseCode::from_code(405));
        } // No method allowed
        if !methods.as_ref().unwrap().contains(request.method()) {
			eprintln!("METHOD NOT ALLOWED !");
            return Err(ResponseCode::from_code(405));
        } // Ok

        return match request.method() {
            // check if implemented (wip)
            &Method::GET	=> Ok(()),

            _ 				=> Err(ResponseCode::from_code(501)), // Not implemented
        };
    }

	fn get_request_location(&self, request: &Request) -> Option<&Location> {
		if self.is_location() == true { return None }
		if self.locations().is_none() { return None }

		let locations = self.locations().unwrap();
		let mut save: Option<&Location> = None;
		let mut save_path = None;

		for (location_path, location) in locations {
			let location_path = match location_path.to_str() {
				Some(location_path) => location_path, None => continue
			};
			
			let request_path = match request.path().to_str() {
				Some(path) => path, None => continue
			};

			if request_path.starts_with(location_path) {
				match location.exact_path() {
					true => {
						if save.is_none() && location_path == request_path { save = Some(location); save_path = Some(location_path) }
					}
					false => {
						if save.is_none() { save = Some(location); save_path = Some(location_path) }
						else if location_path > save_path.unwrap() { save = Some(location) }
					}

				}
			}
		}

		save
	}


}

   // async fn send_response(
    //     &self,
    //     request: &Request,
    //     stream: &mut TcpStream,
    // ) -> Result<(), ResponseCode> {
    //     let _ = match self.parse(&request) {
    //         Ok(()) => (),
    //         Err(err) => return Err(err),
    //     };

    //     match request.method() {
    //         &Method::GET => self.send_get_response(&request, stream).await,
    //         // &Method::POST => {},
    //         // &Method::DELETE => {},
    //         _ => Err(ResponseCode::new(501)), // not implemented
    //     }
    // }

    // async fn send_get_response(
    //     &self,
    //     request: &Request,
    //     stream: &mut TcpStream,
    // ) -> Result<(), ResponseCode> {
    //     let mut path = match self.get_final_path(request.path()) {
	// 		Some(path) => path,
	// 		None => return Err(ResponseCode::new(404)),
	// 	};

    //     if path.is_dir() {
    //         // send_get_response_directory()
    //         eprintln!("index: {}", self.index().as_ref().unwrap());
    //         path = path.join(PathBuf::from(self.index().as_ref().unwrap()));
    //     }

    //     // if self.is_cgi(&request){
    //     // 	// handle CGI GET methods
    //     // 	todo!();
    //     // }

    //     eprintln!("----SENDING RESPONSE----");

    //     // match self.consume_body(stream).await {
    //     // 	Ok(_) => (),
    //     // 	Err(err) => { return Err(ResponseCode::from_error(err.kind())) }
    //     // }

    //     let mut response = match Response::from_file(ResponseCode::new(200), path.as_path()).await {
	// 		Err(err) => return Err(ResponseCode::from_error(err)),
	// 		Ok(response) => response,
	// 	};

    //     match response.send_to(stream).await {
    //         Ok(_) => Ok(()),
    //         Err(err) => Err(ResponseCode::from_error(err.kind())),
    //     }
    // }

    // async fn send_error_response_to(
    //     &self,
    //     stream: &mut TcpStream,
    //     mut code: ResponseCode,
    // ) -> Result<(), ErrorKind> {
    //     let error_page = match self.error_pages().get(&code.code()) {
    //         Some(page) => PathBuf::from(page),
    //         None => {
	// 			code = ResponseCode::new(400);
	// 			PathBuf::from("error_pages/404.html") // TODO: need to have one default page by error (cgi ?)
	// 		}
	// 	};

    //     let response = self.get_final_path(&error_page);

	// 	let mut response = match response {
	// 		Some(path) => {
	// 			Response::from_file(code, path.as_path()).await?
	// 		}
	// 		None => {
	// 			Response::from_file(ResponseCode::new(400), PathBuf::from("error_pages/404.html").as_path()).await?		// final path not found
	// 		}
	// 	};

	// 	match response.send_to(stream).await {
	// 		Ok(_) => Ok(()),
	// 		Err(err) => Err(err.kind()),
	// 	};

	// 	Ok(())

    // }
