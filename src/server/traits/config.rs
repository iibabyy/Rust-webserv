use std::{borrow::Cow, collections::HashMap, path::PathBuf};

use crate::{
    request::{Method, Request},
    response::response::ResponseCode,
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

    /*------------------------------------------------------------*/
    /*-----------------------[ Parsing ]--------------------------*/
    /*------------------------------------------------------------*/

    fn parse_request(&self, request: &mut Request) -> Result<(), ResponseCode> {
        // eprintln!("Parsing request...");

        self.parse_method(request)?;

        if self.max_body_size().is_some() && request.content_length() > self.max_body_size() {
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

    fn get_request_location<'a>(&'a self, request: &Request) -> Option<&'a Location> {
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
pub mod utils {
    use std::{
        io::{self, ErrorKind},
        path::PathBuf,
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
        raw_left: &mut [u8],
        buffer: &mut [u8; 8196],
    ) -> Result<Vec<u8>, io::Error> {
        if let Some(mut stdin) = child.stdin.take() {
            if request.content_length().is_none() {
                return Ok(raw_left.to_vec());
            }

            let content_len = request.content_length().unwrap().to_owned();

            if raw_left.len() >= content_len {
                stdin.write_all(&raw_left[..content_len]).await?;

                return Ok(raw_left[content_len..].to_owned());
            }

            let length_missing = content_len - raw_left.len();
            let mut read = 0;
            let mut n = 0;

            {
                stdin.write_all(raw_left).await?;

                while read < length_missing {
                    n = match stream.read(buffer).await {
                        Ok(n) => n,
                        Err(err) => return Err(err),
                    };
                    read += n;

                    if read < length_missing {
                        stdin.write_all(buffer).await?;
                    }
                }
            }

            let read_too_much = read - length_missing;
            let end = n - read_too_much;

            stdin.write_all(&buffer[..end]).await?;
            return Ok(buffer[end..end + read_too_much].to_vec());
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

        return Some(format!("--{boundary}"));
    }

    pub async fn consume_body(
        request: &Request,
        stream: &mut TcpStream,
        raw_left: &mut [u8],
        buffer: &mut [u8; 8196],
    ) -> Result<Vec<u8>, io::Error> {
        let content_length = request.content_length().unwrap().to_owned() as usize;

        if raw_left.len() >= content_length {
            return Ok(raw_left[content_length..].to_vec());
        }

        stream.read_exact(raw_left).await?;

        let length_missing = content_length - raw_left.len();

        match consume_stream(stream, length_missing, buffer).await {
            Ok(left) => Ok(left),
            Err(err) => Err(err),
        }
    }

    async fn consume_stream(
        stream: &mut TcpStream,
        len: usize,
        buffer: &mut [u8; 8196],
    ) -> io::Result<Vec<u8>> {
        let mut read = 0;
        let mut n = 0;
        while read < len {
            n = match stream.read(buffer).await {
                Ok(n) => n,
                Err(err) => return Err(err),
            };

            read += n;
        }

        let read_too_much = read - len;
        let end = n - read_too_much;

        return Ok(buffer[end..end + read_too_much].to_vec());
    }

    pub enum UploadType {
        Normal,
        Multipart,
    }

    pub fn choose_upload_type(request: &Request) -> UploadType {
        let content_type = request.content_type();

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

    pub fn find_in<T: PartialEq>(big: &[T], little: &[T]) -> Option<usize> {
        big.windows(little.len())
            .position(|window| window == little)
    }
}
