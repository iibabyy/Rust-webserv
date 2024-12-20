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
use utils::UploadType;

use crate::{
    request::{Method, Request},
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
    fn upload_folder(&self) -> Option<&PathBuf>;
    fn alias(&self) -> Option<&PathBuf>;
    fn port(&self) -> Option<&u16>;
    fn index(&self) -> Option<&String>;
    fn max_body_size(&self) -> Option<&usize>; //-
    fn name(&self) -> Option<&Vec<String>>;
    fn path(&self) -> &PathBuf;
    fn methods(&self) -> Option<&Vec<Method>>;
    fn cgi(&self) -> &HashMap<String, PathBuf>; //-
    fn error_pages(&self) -> &HashMap<u16, String>;
    fn error_redirect(&self) -> &HashMap<u16, (Option<u16>, String)>;
    fn locations(&self) -> Option<&HashMap<PathBuf, Location>>;
    fn return_(&self) -> Option<&(u16, Option<String>)>;
    fn internal(&self) -> bool;
    fn is_location(&self) -> bool;

    /*------------------------------------------------------------*/
    /*-------------------------[ Body ]---------------------------*/
    /*------------------------------------------------------------*/

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
        match request.method() {
            &Method::GET => return self.build_get_response(request).await,
            &Method::POST => return self.build_get_response(request).await,
            _ => return Err(io::Error::new(ErrorKind::Other, "method not implemented")), // not implemented
        }
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
        // todo! handle GET on cgi

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
            // TODO!: List directory
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

        let extension = path.extension().unwrap().to_string_lossy().to_string();

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

        if request.content_length() > self.max_body_size() {
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
        if request.path().is_dir() == false {
            return Ok(());
        } // not a dir -> no index needed

        if self.auto_index() == true {
            if self.index().is_some() {
                let path = request.path().to_str().unwrap();
                let path = format!("{}/{}", path, self.index().unwrap(),);

                let path = PathBuf::from(path);
                request.set_path(path);
            } else {
                // auto index on but no index
                return Err(ResponseCode::from_code(404));
            }
        }

        // TODO!: later in the code, when building response, check if directory, and list the directory if it's one

        Ok(())
    }

    fn add_root_or_alias(&self, request: &mut Request) -> Result<(), ResponseCode> {
        let path = if self.alias().is_some() {
            let path = request.path().to_str().unwrap().to_string();
            let path = path.replacen(
                self.path().to_str().unwrap(),
                self.alias().unwrap().to_str().unwrap(),
                1,
            );
            eprintln!(
                "[\n\tAlias: {} -> {}\n\tpath: {}\n\t result: {}\n]",
                self.path().display(),
                self.alias().unwrap().display(),
                request.path().display(),
                path,
            );

            PathBuf::from(path)
        } else if self.root().is_some() {
            let mut path = self.root().unwrap().clone();
            path = PathBuf::from(format!(
                "{}/{}",
                path.to_str().unwrap(),
                request.path().to_str().unwrap(),
            ));

            path
        } else {
            return Err(ResponseCode::from_code(404));
        }; // no root nor alias

        request.set_path(path);
        Ok(())
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
            &Method::UNKNOWN => Err(ResponseCode::from_code(501)), // Not allowed
            _ => Ok(()),
        };
    }

    fn get_request_location(&self, request: &Request) -> Option<&Location> {
        if self.is_location() == true {
            return None;
        }
        if self.locations().is_none() {
            return None;
        }

        let locations = self.locations().unwrap();
        let mut save: Option<&Location> = None;
        let mut save_path = None;

        for (location_path, location) in locations {
            let location_path = match location_path.to_str() {
                Some(location_path) => location_path,
                None => continue,
            };

            let request_path = match request.path().to_str() {
                Some(path) => path,
                None => continue,
            };

            if request_path.starts_with(location_path) {
                match location.exact_path() {
                    true => {
                        if save.is_none() && location_path == request_path {
                            save = Some(location);
                            save_path = Some(location_path)
                        }
                    }
                    false => {
                        if save.is_none() {
                            save = Some(location);
                            save_path = Some(location_path)
                        } else if location_path > save_path.unwrap() {
                            save = Some(location)
                        }
                    }
                }
            }
        }

        save
    }
}

#[allow(dead_code)]
mod utils {
    use std::io::{self, ErrorKind};

    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
        process::Child,
    };

    use crate::request::Request;

    pub async fn send_body_to_cgi(
        request: &Request,
        stream: &mut TcpStream,
        child: &mut Child,
        raw_left: &mut str,
    ) -> Result<String, io::Error> {
        if let Some(mut stdin) = child.stdin.take() {
            if request.content_length().is_none() {
                return Ok(raw_left.to_owned());
            }

            let content_len = request.content_length().unwrap().to_owned();

            if raw_left.len() >= content_len {
                unsafe {
                    stdin
                        .write_all(raw_left[..content_len].as_bytes_mut())
                        .await?;
                }

                return Ok(raw_left[content_len..].to_owned());
            }

            let length_missing = content_len - raw_left.len();
            let mut buffer = [0; 65536];
            let mut read = 0;
            let mut n = 0;

            {
                unsafe {
                    stdin.write_all(raw_left.as_bytes_mut()).await?;
                }

                while read < length_missing {
                    n = match stream.read(&mut buffer).await {
                        Ok(n) => n,
                        Err(err) => return Err(err),
                    };
                    read += n;

                    if read < length_missing {
                        stdin.write_all(&mut buffer).await?;
                    }
                }
            }

            let read_too_much = read - length_missing;
            let end = n - read_too_much;

            stdin.write_all(&buffer[..end]).await?;
            return Ok(String::from_utf8_lossy(&buffer[end..]).to_string());
        } else {
            return Err(io::Error::new(
                ErrorKind::BrokenPipe,
                "failed to establish connection with CGI's pipe",
            ));
        }
    }

    pub fn extract_boundary(content_type: Option<&String>) -> Option<String> {
        let to_find = "boundary=";

        if content_type.is_none() {
            return None;
        }

        let content_type = content_type.unwrap();
        if content_type.contains(to_find) == false {
            return None;
        }

        let boundary_pos = content_type.find(to_find).unwrap() + to_find.len();
        if content_type.len() < boundary_pos {
            return None;
        }

        let mut boundary = content_type[boundary_pos..].split_whitespace();
        let boundary = boundary.nth(0)?;

        return Some(boundary.to_string());
    }

    pub async fn consume_body(
        request: &Request,
        stream: &mut TcpStream,
        raw_left: &mut str,
    ) -> Result<String, io::Error> {
        let content_length = request.content_length().unwrap().to_owned() as usize;

        if raw_left.len() >= content_length {
            return Ok(raw_left[content_length..].to_string());
        }

        unsafe {
            stream.read_exact(raw_left.as_bytes_mut()).await?;
        }

        let length_missing = content_length - raw_left.len();

        match consume_stream(stream, length_missing).await {
            Ok(str) => Ok(str),
            Err(err) => Err(err),
        }
    }

    async fn consume_stream(stream: &mut TcpStream, len: usize) -> io::Result<String> {
        let mut buffer = [0; 65536];
        let mut read = 0;
        let mut n = 0;
        while read < len {
            n = match stream.read(&mut buffer).await {
                Ok(n) => n,
                Err(err) => return Err(err),
            };

            read += n;
        }

        let read_too_much = read - len;
        let end = n - read_too_much;

        return Ok(String::from_utf8_lossy(&buffer[end..]).to_string());
    }

    pub enum UploadType {
        Normal,
        Multipart,
    }

    pub fn choose_upload_type(request: &Request) -> UploadType {
        let content_type = request.get("Content-Type");

        if content_type.is_none() {
            return UploadType::Normal;
        }

        let content_type = content_type.unwrap();

        if content_type.contains("multipart/form-data") {
            return UploadType::Multipart;
        } else {
            return UploadType::Normal;
        }
    }
}
