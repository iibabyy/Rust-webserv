use std::{
    collections::HashMap,
    io::{self, ErrorKind},
    path::PathBuf,
    process::{Output, Stdio},
};

use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    process::Command,
};

use crate::{
    listener::send_error_response,
    request::{Method, Request},
    response::response::{Response, ResponseCode},
};

use super::config::{utils, Config};


pub trait Handler: Config {
    async fn handle_request(
        &self,
        mut request: Request,
        stream: &mut TcpStream,
        raw_left: &mut str,
    ) -> Option<String> {
        match self.parse_request(&mut request) {
            Ok(location) => location,
            Err(err) => {
                eprintln!("Error: parsing request: {}", err.to_string());
                send_error_response(stream, err).await;
                return if request.keep_connection_alive() == true {
                    Some(raw_left.to_string())
                } else {
                    None
                };
            }
        }

        let raw_left = if self.is_cgi(&request) {
            self.handle_cgi(&request, stream, raw_left).await
        } else {
            self.handle_non_cgi(&request, stream, raw_left).await
        };

        return raw_left;
    }

    async fn handle_non_cgi(
        &self,
        request: &Request,
        stream: &mut TcpStream,
        raw_left: &mut str,
    ) -> Option<String> {
        let raw_left = match self.handle_body_request(&request, stream, raw_left).await {
            Ok(raw_left) => raw_left,
            Err(err) => {
                match err.kind() {
                    ErrorKind::UnexpectedEof => return None, // end of stream

                    _ => {
                        println!("Error: handling body: {}", err.to_string());
                        send_error_response(stream, ResponseCode::from_error(&err)).await;

                        if request.keep_connection_alive() == true {
                            return Some(raw_left.to_string()); // keep stream alive
                        } else {
                            return None; // kill stream
                        };
                    }
                }
            }
        };

        match self.send_response(stream, &request).await {
            Ok(_) => (),
            Err(err) => {
                println!("Error: sending response: {err}");
                send_error_response(stream, ResponseCode::from_error(&err)).await;
                return if request.keep_connection_alive() == true
                    && err.kind() != ErrorKind::UnexpectedEof
                {
                    Some(raw_left)
                } else {
                    None
                };
            }
        }

        Some(raw_left)
    }

    async fn handle_cgi(
        &self,
        request: &Request,
        stream: &mut TcpStream,
        raw_left: &mut str,
    ) -> Option<String> {
        eprintln!("executing CGI");

        let (output, raw_left) = match self.execute_cgi(request, stream, raw_left).await {
            Ok(res) => res,
            Err(err) => {
                eprintln!(
                    "Error : {}: sending response: {err}",
                    request.path().display()
                );
                send_error_response(stream, ResponseCode::from_error(&err)).await;
                if request.keep_connection_alive() == true && err.kind() != ErrorKind::UnexpectedEof
                {
                    return Some(raw_left.to_owned());
                } else {
                    return None;
                };
            }
        };

        eprintln!("sending CGI response");
        match Self::send_cgi_response(output, stream).await {
            Ok(_) => (),
            Err(err) => {
                println!("Error: sending response: {err}");
                send_error_response(stream, ResponseCode::from_error(&err)).await;
                if request.keep_connection_alive() == true && err.kind() != ErrorKind::UnexpectedEof
                {
                    return Some(raw_left.to_owned());
                } else {
                    return None;
                };
            }
        }

        eprintln!("CGI response send !");

        Some(raw_left)
    }

    async fn send_cgi_response(cgi_output: Output, stream: &mut TcpStream) -> io::Result<()> {
        stream.write_all("HTTP/1.1 200 OK\r\n".as_bytes()).await?;

        // eprintln!("CGI response: \n{:#?}", String::from_utf8_lossy(cgi_output.stdout.as_bytes()).to_string());
        stream.write_all(&cgi_output.stdout).await
    }

    async fn handle_body_request(
        &self,
        request: &Request,
        stream: &mut TcpStream,
        raw_left: &mut str,
    ) -> Result<String, io::Error> {
        if request.content_length().is_none() {
            return Ok(raw_left.to_string());
        }

        match request.method() {
            &Method::POST => self.upload_body(request, stream, raw_left).await,
            _ => utils::consume_body(request, stream, raw_left).await,
        }
    }

    /*------------------------------------------------------------*/
    /*-----------------------[ Upload ]---------------------------*/
    /*------------------------------------------------------------*/

    async fn upload_body(
        &self,
        request: &Request,
        stream: &mut TcpStream,
        raw_left: &str,
    ) -> Result<String, io::Error> {
        if self.upload_folder().is_none() {
            return Err(io::Error::new(ErrorKind::NotFound, "No upload folder"));
        }

        let upload_folder = self.upload_folder().unwrap();

        if upload_folder.exists() == false {
            return Err(io::Error::new(
                ErrorKind::NotFound,
                "Upload folder not found",
            ));
        }

        match utils::choose_upload_type(request) {
            // UploadType::Multipart => todo!(),
            _ => {
                self.upload_default_content(request, stream, raw_left, upload_folder)
                    .await
            }
        }
    }

    /*------------------------------------------------------------*/
    /*------------------[ Multipart Upload ]----------------------*/
    /*------------------------------------------------------------*/

    // async fn upload_mutipart_content(
    //	 &self,
    //	 request: &Request,
    //	 stream: &mut TcpStream,
    //	 raw_left: &str,
    //	 upload_folder: &PathBuf,
    // ) -> Result<String, io::Error> {
    // 	let boundary = match utils::extract_boundary(request.content_type()) {
    // 		Some(boundary) => boundary,
    // 		None => return Err(io::Error::new(ErrorKind::InvalidInput, "boundary not found")),
    // 	};

    // }

    /*------------------------------------------------------------*/
    /*-------------------[ Default Upload ]-----------------------*/
    /*------------------------------------------------------------*/

    async fn upload_default_content(
        &self,
        request: &Request,
        stream: &mut TcpStream,
        raw_left: &str,
        upload_folder: &PathBuf,
    ) -> Result<String, io::Error> {
        let file = format!("{}/test", upload_folder.to_str().unwrap());

        let mut file = match OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(file)
            .await
        {
            Ok(file) => file,
            Err(err) => return Err(err),
        };

        self.default_upload(&mut file, stream, request, raw_left)
            .await
    }

    async fn default_upload(
        &self,
        file: &mut File,
        stream: &mut TcpStream,
        request: &Request,
        raw_left: &str,
    ) -> Result<String, io::Error> {
        let mut buffer = [0; 65536];
        let mut read = 0;
        let mut n = 0;
        let body_len = request.content_length().unwrap().clone() as usize;

        if body_len <= raw_left.len() {
            file.write(&raw_left.as_bytes()[..body_len]).await?;
            return Ok(raw_left[body_len..].to_string());
        } else {
            file.write(raw_left.as_bytes()).await?;
            read += raw_left.len();
        }

        while read < body_len {
            n = match stream.read_exact(&mut buffer).await {
                Ok(n) => n,
                Err(err) => return Err(err),
            };

            read += n;

            if read > body_len {
                file.write_all(&buffer[..(n - (read - body_len))]).await?;
            } else {
                file.write_all(&buffer[..n]).await?;
            }
        }

        let end = n - (read - body_len);

        return Ok(String::from_utf8_lossy(&buffer[end..]).to_string());
    }

    /*------------------------------------------------------------*/
    /*-----------------------[ Response ]-------------------------*/
    /*------------------------------------------------------------*/

    async fn build_response(&self, request: &Request) -> Result<Response, io::Error> {
        eprintln!("Building response...");
        if request.path().is_dir() {
            return utils::build_auto_index(request.path()).await;
        }

        match request.method() {
            &Method::GET => return self.build_get_response(request).await,
            &Method::POST => return self.build_get_response(request).await,
            _ => return Err(io::Error::new(ErrorKind::Other, "method not implemented")), // not implemented
        }
    }

    async fn send_response(
        &self,
        stream: &mut TcpStream,
        request: &Request,
    ) -> Result<(), io::Error> {
        let mut response = self.build_response(request).await?;

        response.send(stream).await
    }

    /*------------------------------------------------------------*/
    /*-------------------------[ GET ]----------------------------*/
    /*------------------------------------------------------------*/

    async fn build_get_response(&self, request: &Request) -> Result<Response, io::Error> {
        eprintln!(
            "Building GET response for '{}'...",
            request.path().display()
        );
        // todo! list files if no auto_index

        let file = self.get_GET_request_file(request).await?;

        eprintln!("GET response build...");
        let mut response = Response::new(ResponseCode::default()).file(file);

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
            todo!("List directory");
        } else {
            Err(io::Error::new(ErrorKind::NotFound, "file not found"))
        }
    }

    /*------------------------------------------------------------*/
    /*-------------------------[ CGI ]----------------------------*/
    /*------------------------------------------------------------*/

    fn is_cgi(&self, request: &Request) -> bool {
        let path = request.path();

        if path.extension().is_none() {
            return false;
        }

        let extension = match path.extension() {
            Some(extension) => extension.to_string_lossy().to_string(),
            None => return false,
        };

        return self.cgi().contains_key(&extension);
    }

    async fn execute_cgi(
        &self,
        request: &Request,
        stream: &mut TcpStream,
        raw_left: &mut str,
    ) -> Result<(Output, String), io::Error> {
        // execute cgi :
        //		- check program path
        //		- execute path with Command::new() (set envs)
        //		- send body to command stdin (previously piped)
        //		- get output and check status
        //		- return output

        if request.path().is_file() == false {
            return Err(io::Error::new(
                ErrorKind::NotFound,
                "CGI failure: not found",
            ));
        }

        let file = request.path();
        let executor = self
            .cgi()
            .get(file.extension().unwrap().to_str().unwrap())
            .unwrap();

        let mut child = Command::new(executor)
            .arg(file)
            .env_clear()
            .envs(self.cgi_envs(request))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let raw_left = utils::send_body_to_cgi(request, stream, &mut child, raw_left).await?;

        let output = child.wait_with_output().await?;

        if output.status.success() == false {
            return Err(io::Error::new(
                ErrorKind::Other,
                format!("CGI failure: program exited with status {}", output.status),
            ));
        }

        Ok((output, raw_left))
    }

    fn cgi_envs(&self, request: &Request) -> HashMap<String, String> {
        let mut env: HashMap<String, String> = HashMap::new();

        env.insert("REQUEST_METHOD".to_owned(), request.method().to_string());
        env.insert(
            "HTTP_CONNECTION".to_owned(),
            if request.keep_connection_alive() == true {
                "keep-alive"
            } else {
                "close "
            }
            .to_owned(),
        );

        if let Some(content_len) = request.content_length() {
            env.insert("CONTENT_LENGTH".to_owned(), content_len.to_string());
        }
        if let Some(accept) = request.accept() {
            env.insert("HTTP_ACCEPT".to_owned(), accept.to_owned());
        }
        if let Some(content_type) = request.content_type() {
            env.insert("HTTP_CONTENT_TYPE".to_owned(), content_type.to_owned());
        }
        if let Some(host) = request.host() {
            env.insert("HTTP_HOST".to_owned(), host.to_owned());
        }

        for (key, value) in request.headers() {
            let env_key = format!("HTTP_{}", key.replace('-', "_").to_uppercase());
            env.insert(env_key, value.to_owned());
        }

        if let Some(query) = request.query() {
            env.insert("QUERY_STRING".to_owned(), query.to_owned());
        } else {
            env.insert("QUERY_STRING".to_owned(), String::new());
        }

        env
    }
}
