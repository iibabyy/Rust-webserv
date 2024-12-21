use std::{
    borrow::Cow,
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
        if request.path().is_dir() {
            return utils::build_auto_index(request.path()).await;
        }

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
        let path = request.path();
        let path_str = request.path().to_string_lossy();

        if path.is_file() == true {
            return Ok(());
        } else if path.is_dir() == true {
            match self.format_dir_path(path_str) {
                Ok(Some(path)) => request.set_path(path),
                Ok(None) => (),
                Err(response) => return Err(response),
            };

            return Ok(());
        } else {
            return Err(ResponseCode::from_code(404));
        }
    }

    fn format_dir_path(&self, path_str: Cow<'_, str>) -> Result<Option<PathBuf>, ResponseCode> {
        if self.index().is_some() {
            let path = format!("{}/{}", path_str, self.index().unwrap(),);
            let path = PathBuf::from(path);
            if path.is_file() {
                return Ok(Some(path));
            }
        }

        if self.auto_index() == false {
            return Err(ResponseCode::from_code(403));
        }

        Ok(None)
    }

    fn add_root_or_alias(&self, request: &mut Request) -> Result<(), ResponseCode> {
        let path = if self.alias().is_some() {
            let path = request.path().to_str().unwrap().to_string();
            let path = path.replacen(
                self.path().to_str().unwrap(),
                self.alias().unwrap().to_str().unwrap(),
                1,
            );

            PathBuf::from(path)
        } else if self.root().is_some() {
            let mut path = self.root().unwrap().clone();
            path = PathBuf::from(format!(
                "{}{}",
                path.to_str().unwrap(),
                request.path().to_str().unwrap(),
            ));

            path
        } else {
            return Err(ResponseCode::from_code(404));
        }; // no root nor alias

        if path.is_dir() && path.to_string_lossy().ends_with("/") == false {
            let redirect = PathBuf::from(format!("{}/", request.path().to_string_lossy()));
            eprintln!("redirected on {}", redirect.display());
            let response = ResponseCode::new_redirect(301, "Moved Premanently", redirect);
            return Err(response);
        }

        request.set_path(path);
        Ok(())
    }

    fn parse_method(&self, request: &Request) -> Result<(), ResponseCode> {
        let methods = self.methods();
        if methods.is_none() {
            eprintln!("NO METHODS ALLOWED");
            return Err(ResponseCode::from_code(405));
        } // No method allowed
        if !methods.as_ref().unwrap().contains(request.method()) {
            eprintln!("METHOD NOT ALLOWED !");
            return Err(ResponseCode::from_code(405));
        } // Ok

        return match request.method() {
            // check if implemented (wip)
            &Method::UNKNOWN | &Method::UNDEFINED => Err(ResponseCode::from_code(501)), // Not allowed
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
    use std::{
        io::{self, ErrorKind},
        path::PathBuf,
        usize,
    };

    use tokio::{
        fs,
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
        process::Child,
    };

    use crate::{
        request::Request,
        response::response::{Response, ResponseCode},
    };

    pub async fn build_auto_index(dir: &PathBuf) -> io::Result<Response> {
        eprintln!(
            "building auto-index for {:?}",
            dir.to_string_lossy().to_string()
        );

        let files_ref = html_files_ref_from(dir).await?;
        let files_ref = format_file_ref(files_ref);

        let mut html = String::new();

        let dir_name = match dir.file_name() {
            Some(name) => name.to_str().unwrap_or("directory"),
            None => "directory",
        };

        html.push_str("<!DOCTYPE html>\r\n");
        html.push_str("<html lang=\"en\">\r\n");
        html.push_str("<head>\r\n");
        html.push_str("    <meta charset=\"UTF-8\">\r\n");
        html.push_str(
            "    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\r\n",
        );
        html.push_str(&format!("    <title>Index of {dir_name}</title>\r\n"));
        html.push_str("    <style>\r\n");
        html.push_str("        body {\r\n");
        html.push_str("            font-family: Arial, sans-serif;\r\n");
        html.push_str("            margin: 0;\r\n");
        html.push_str("            padding: 0;\r\n");
        html.push_str("            display: flex;\r\n");
        html.push_str("            flex-direction: column;\r\n");
        html.push_str("            align-items: center;\r\n");
        html.push_str("            justify-content: center;\r\n");
        html.push_str("            height: 100vh;\r\n");
        html.push_str("            background: linear-gradient(135deg, #1e3c72, #2a5298, #f6d365, #fda085);\r\n");
        html.push_str("            background-size: 300% 300%;\r\n");
        html.push_str("            animation: gradientAnimation 15s ease infinite;\r\n");
        html.push_str("            color: #fff;\r\n");
        html.push_str("        }\r\n");
        html.push_str("        h1 {\r\n");
        html.push_str("            color: #fff;\r\n");
        html.push_str("            font-size: 24px;\r\n");
        html.push_str("            margin-top: 20px;\r\n");
        html.push_str("            margin-bottom: 20px;\r\n");
        html.push_str("            border-bottom: 3px solid #fff;\r\n");
        html.push_str("            padding-bottom: 10px;\r\n");
        html.push_str("            text-shadow: 2px 2px 5px rgba(0, 0, 0, 0.3);\r\n");
        html.push_str("        }\r\n");
        html.push_str("        .button-container {\r\n");
        html.push_str("            display: flex;\r\n");
        html.push_str("            flex-direction: column;\r\n");
        html.push_str("            align-items: center;\r\n");
        html.push_str("            gap: 10px;\r\n");
        html.push_str("        }\r\n");
        html.push_str("        .button-container button {\r\n");
        html.push_str("            font-size: 16px;\r\n");
        html.push_str("            padding: 10px 20px;\r\n");
        html.push_str("            border: none;\r\n");
        html.push_str("            border-radius: 8px;\r\n");
        html.push_str("            cursor: pointer;\r\n");
        html.push_str("            color: #fff;\r\n");
        html.push_str("            font-weight: bold;\r\n");
        html.push_str("            box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);\r\n");
        html.push_str("            transition: transform 0.2s ease, box-shadow 0.2s ease;\r\n");
        html.push_str("        }\r\n");
        html.push_str("        .button-container button:hover {\r\n");
        html.push_str("            transform: scale(1.1);\r\n");
        html.push_str("            box-shadow: 0 6px 10px rgba(0, 0, 0, 0.15);\r\n");
        html.push_str("        }\r\n");
        html.push_str("        .color-1 { background-color: #ff6b6b; }\r\n");
        html.push_str("        .color-1:hover { background-color: #ff4b4b; }\r\n");
        html.push_str("        .color-2 { background-color: #48dbfb; }\r\n");
        html.push_str("        .color-2:hover { background-color: #33c7f2; }\r\n");
        html.push_str("        .color-3 { background-color: #1dd1a1; }\r\n");
        html.push_str("        .color-3:hover { background-color: #10b892; }\r\n");
        html.push_str("        .color-4 { background-color: #feca57; }\r\n");
        html.push_str("        .color-4:hover { background-color: #fdb844; }\r\n");
        html.push_str("        .color-5 { background-color: #ff9ff3; }\r\n");
        html.push_str("        .color-5:hover { background-color: #ff7eea; }\r\n");
        html.push_str("        @keyframes gradientAnimation {\r\n");
        html.push_str("            0% { background-position: 0% 50%; }\r\n");
        html.push_str("            50% { background-position: 100% 50%; }\r\n");
        html.push_str("            100% { background-position: 0% 50%; }\r\n");
        html.push_str("        }\r\n");
        html.push_str("    </style>\r\n");
        html.push_str("</head>\r\n");
        html.push_str("<body>\r\n");
        html.push_str(&format!("    <h1>Index of {dir_name}</h1>\r\n"));
        html.push_str("    <div class=\"button-container\">\r\n");
        html.push_str(&files_ref);
        html.push_str("    <div style=\"margin-top: 20px; text-align: center;\">\r\n");
        html.push_str("        <button class=\"color-1\" onclick=\"window.history.back()\">Go Back</button>\r\n");
        html.push_str("    </div>\r\n");
        html.push_str("    </div>\r\n");

        html.push_str("</body>\r\n");
        html.push_str("</html>\r\n");
        let mut response = Response::new(ResponseCode::new(200, "OK"));

        response.set_content(html);

        Ok(response)
    }

    fn format_file_ref(files: Vec<String>) -> String {
        let mut html = String::new();

        let mut color_index: usize = 2;
        for file_name in files {
            let color_class = format!("color-{}", color_index.to_string());
            color_index = (color_index % 5) + 1;

            html.push_str(&format!(
                r#"        <button class="{}" onclick="window.location.href='{}'">{}</button>"#,
                color_class,
                // dir.to_string_lossy().to_string(),
                file_name,
                file_name
            ));
            html.push_str("\r\n");
        }

        html
    }

    async fn html_files_ref_from(dir: &PathBuf) -> io::Result<Vec<String>> {
        let mut files = vec![];

        let mut entries = fs::read_dir(dir).await?;
        let dir_str = dir.to_string_lossy().to_string();

        while let Some(entry) = entries.next_entry().await? {
            let filename = entry.file_name();
            let filename = match filename.to_str() {
                Some(name) => name,
                None => continue,
            };

            if filename.starts_with(".") {
                continue;
            }

            files.push(filename.to_owned());
        }

        files.sort();

        Ok(files)
    }

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
            return Ok(String::from_utf8_lossy(&buffer[end..end + read_too_much]).to_string());
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

        return Ok(String::from_utf8_lossy(&buffer[end..end + read_too_much]).to_string());
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
