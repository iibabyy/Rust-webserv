use std::{
    collections::HashMap,
    io::{self, ErrorKind},
    path::PathBuf,
    process::{Output, Stdio},
};

use nom::{AsBytes, FindSubstring};
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

use super::config::{
    utils::{self, UploadType},
    Config,
};

#[allow(dead_code)]
pub struct MultipartFile {
    filename: String,
    content_type: Option<String>,
}

pub trait Handler: Config {
    async fn handle_request(
        &self,
        mut request: Request,
        stream: &mut TcpStream,
        raw_left: &mut [u8],
        buffer: &mut [u8; 8196],
    ) -> Option<Vec<u8>> {
        // eprintln!("Request: {request:#?}");
        match self.parse_request(&mut request) {
            Ok(location) => location,
            Err(err) => {
                eprintln!("Error: parsing request: {}", err.to_string());
                send_error_response(stream, err, buffer).await;
                return if request.keep_connection_alive() == true {
                    Some(raw_left.to_vec())
                } else {
                    None
                };
            }
        }

        let raw_left = if self.is_cgi(&request) {
            self.handle_cgi(&request, stream, raw_left, buffer).await
        } else {
            self.handle_non_cgi(&request, stream, raw_left, buffer)
                .await
        };

        return raw_left;
    }

    async fn handle_non_cgi(
        &self,
        request: &Request,
        stream: &mut TcpStream,
        raw_left: &mut [u8],
        buffer: &mut [u8; 8196],
    ) -> Option<Vec<u8>> {
        let raw_left = match self
            .handle_body_request(&request, stream, raw_left, buffer)
            .await
        {
            Ok(raw_left) => raw_left,
            Err(err) => {
                match err.kind() {
                    ErrorKind::UnexpectedEof => return None, // end of stream

                    _ => {
                        println!("Error: handling body: {}", err.to_string());
                        send_error_response(stream, ResponseCode::from_error(&err), buffer).await;

                        if request.keep_connection_alive() == true {
                            return Some(raw_left.to_vec()); // keep stream alive
                        } else {
                            return None; // kill stream
                        };
                    }
                }
            }
        };

        match self.send_response(stream, &request, buffer).await {
            Ok(_) => (),
            Err(err) => {
                println!("Error: sending response: {err}");
                send_error_response(stream, ResponseCode::from_error(&err), buffer).await;
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
        raw_left: &mut [u8],
        buffer: &mut [u8; 8196],
    ) -> Option<Vec<u8>> {
        eprintln!("executing CGI");

        let (output, raw_left) = match self.execute_cgi(request, stream, raw_left, buffer).await {
            Ok(res) => res,
            Err(err) => {
                eprintln!(
                    "Error : {}: sending response: {err}",
                    request.path().display()
                );
                send_error_response(stream, ResponseCode::from_error(&err), buffer).await;
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
                send_error_response(stream, ResponseCode::from_error(&err), buffer).await;
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
        raw_left: &mut [u8],
        buffer: &mut [u8; 8196],
    ) -> Result<Vec<u8>, io::Error> {
        if request.content_length().is_none() {
            return Ok(raw_left.to_vec());
        }

        match request.method() {
            &Method::POST => self.upload_body(request, stream, raw_left, buffer).await,
            _ => utils::consume_body(request, stream, raw_left, buffer).await,
        }
    }

    /*------------------------------------------------------------*/
    /*-----------------------[ Upload ]---------------------------*/
    /*------------------------------------------------------------*/

    async fn upload_body(
        &self,
        request: &Request,
        stream: &mut TcpStream,
        raw_left: &[u8],
        buffer: &mut [u8; 8196],
    ) -> Result<Vec<u8>, io::Error> {
        eprintln!("uploading file");

        if request.content_length().is_none() {
            eprintln!("No content length -> No upload");
            return Ok(raw_left.to_vec());
        } else if self.upload_folder().is_none() {
            eprintln!("no upload folder");
            return Err(io::Error::new(ErrorKind::NotFound, "No upload folder"));
        }

        let upload_folder = self.upload_folder().unwrap();

        if upload_folder.exists() == false {
            eprintln!("no upload folder");
            return Err(io::Error::new(
                ErrorKind::NotFound,
                "Upload folder not found",
            ));
        }

        let res = match utils::choose_upload_type(request) {
            UploadType::Multipart => {
                self.handle_mutlipart_upload(request, stream, raw_left, upload_folder, buffer)
                    .await
            }
            _ => {
                self.upload_default_content(request, stream, raw_left, upload_folder, buffer)
                    .await
            }
        };

        eprintln!("finish upload");
        return res;
    }

    /*------------------------------------------------------------*/
    /*------------------[ Multipart Upload ]----------------------*/
    /*------------------------------------------------------------*/

    async fn handle_mutlipart_upload(
        &self,
        request: &Request,
        stream: &mut TcpStream,
        raw_left: &[u8],
        upload_folder: &PathBuf,
        buffer: &mut [u8; 8196],
    ) -> Result<Vec<u8>, io::Error> {
        let boundary = match utils::extract_boundary(request.content_type()) {
            Some(boundary) => boundary,
            None => {
                return Err(io::Error::new(
                    ErrorKind::InvalidInput,
                    "boundary not found",
                ))
            }
        };

        Self::upload_multipart_content(
            stream,
            request.content_length().unwrap().clone(),
            raw_left,
            boundary,
            upload_folder,
            buffer,
        )
        .await
    }

    async fn upload_multipart_content(
        stream: &mut TcpStream,
        content_len: usize,
        raw_left: &[u8],
        boundary: String,
        upload_folder: &PathBuf,
        buffer: &mut [u8; 8196],
    ) -> io::Result<Vec<u8>> {
        let mut readed = 0;

        let mut raw_left = raw_left.to_owned();

        while readed < content_len {
            let index;
            (raw_left, index) = Self::read_until_find(
                boundary.as_bytes(),
                content_len - readed,
                &raw_left,
                stream,
                buffer,
            )
            .await?;

            if index != Some(0) {
                return Err(io::Error::new(ErrorKind::InvalidData, "expected boundary"));
            }

            let temp = String::from_utf8_lossy(&raw_left[boundary.len()..boundary.len() + 4]);
            readed += boundary.len();

            if temp.starts_with("--\r\n") {
                readed += 4;
                if readed == content_len {
                    return Ok(raw_left[boundary.len() + 4..].to_vec());
                } else {
                    return Err(io::Error::new(
                        ErrorKind::InvalidData,
                        "Invalid Content length",
                    ));
                }
            } else if temp.starts_with("\r\n") {
                readed += 2;
            } else {
                return Err(io::Error::new(ErrorKind::InvalidData, "invalid boundary"));
            };

            let file;
            let read;
            (file, raw_left, read) = Self::deserialize_multipart_header(
                content_len - readed,
                stream,
                &raw_left[boundary.len() + 2..],
                buffer,
            )
            .await?;

            readed += read;

            let read;
            (raw_left, read) = Self::create_and_upload(
                file,
                raw_left,
                stream,
                upload_folder,
                boundary.as_bytes(),
                buffer,
            )
            .await?;
            readed += read;
        }

        Ok(raw_left)
    }

    async fn create_and_upload(
        file: MultipartFile,
        raw_left: Vec<u8>,
        stream: &mut TcpStream,
        upload_folder: &PathBuf,
        boundary: &[u8],
        buffer: &mut [u8; 8196],
    ) -> io::Result<(Vec<u8>, usize)> {
        let mut file = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .truncate(true)
            .open(PathBuf::from(format!(
                "{}/{}",
                upload_folder.to_string_lossy().to_string(),
                file.filename
            )))
            .await?;

        if let Some(index) = utils::find_in(&raw_left, boundary) {
            file.write_all(&raw_left[..index - 2]).await?;
            return Ok((raw_left[index..].to_vec(), index));
        }

        let boundary = boundary.as_bytes();
        let mut raw_left = raw_left.as_bytes();
        let mut readed = 0;
        let mut temp;

        loop {
            let n = match stream.read(buffer).await? {
				0 => return Err(io::Error::new(ErrorKind::UnexpectedEof, "unexpected end of stream")),
				n => n,
			};
            temp = raw_left.to_vec();
            temp.append(&mut buffer[..n].to_vec());
            raw_left = temp.as_bytes();

            if let Some(index) = raw_left.find_substring(boundary) {
                file.write_all(&raw_left[..index - 2]).await?;
                readed += index;
                return Ok((raw_left[index/* + boundary.len() + 2 */..].to_vec(), readed));
            }

            let security = if raw_left.len() > boundary.len() {
                raw_left.len() - boundary.len()
            } else {
                0
            };

            file.write_all(&raw_left[..security]).await?;
            readed += security;
            raw_left = &raw_left[security..];
        }
    }

    async fn deserialize_multipart_header(
        read_limit: usize,
        stream: &mut TcpStream,
        raw_left: &[u8],
        buffer: &mut [u8; 8196],
    ) -> io::Result<(MultipartFile, Vec<u8>, usize)> {
        let readed;

        let (raw_left, index) =
            Self::read_until_find(b"\r\n\r\n", read_limit, raw_left, stream, buffer).await?;
        let (headers, raw_left) = match index {
            Some(index) => {
                readed = index + 4;
                (
                    String::from_utf8_lossy(&raw_left[..index + 2]),
                    raw_left[index + 4..].to_vec(),
                )
            }
            None => {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "invalid multipart content",
                ))
            }
        };

        let mut content_type = None;
        let mut content_disposition = None;

        for header in headers.split("\r\n") {
            if header.starts_with("Content-Type: ") {
                content_type = Some(header["Content-Type: ".len()..].to_string());
            } else if header.starts_with("Content-Disposition") {
                content_disposition = Some(header["Content-Disposition".len()..].to_string());
            }
        }

        if content_disposition.is_none() {
            return Ok((
                MultipartFile {
                    filename: "temp".to_owned(),
                    content_type,
                },
                raw_left,
                readed,
            ));
        }
        let content_disposition = content_disposition.unwrap();

        let filename_index = content_disposition.find("filename=");
        let filename = if filename_index.is_none() {
            "\"temp\""
        } else {
            let filename_index = filename_index.unwrap() + "filename=".len();
            &content_disposition[filename_index..]
        };

        if filename.starts_with("\"") == false || filename.ends_with("\"") == false {
            return Err(io::Error::new(ErrorKind::InvalidData, "invalid filename"));
        }

        let filename = filename[1..filename.len() - 1].to_string();

        return Ok((
            MultipartFile {
                filename,
                content_type,
            },
            raw_left,
            readed,
        ));
    }

    async fn read_until_find(
        to_find: &[u8],
        read_limit: usize,
        raw_left: &[u8],
        stream: &mut TcpStream,
        buffer: &mut [u8; 8196],
    ) -> io::Result<(Vec<u8>, Option<usize>)> {
        if let Some(index) = utils::find_in(raw_left, to_find) {
            return Ok((raw_left.to_owned(), Some(index)));
        }

        let mut readed = 0;
        let mut raw_left = raw_left.to_owned();

        while readed < read_limit {
            let n = match stream.read(buffer).await? {
				0 => return Err(io::Error::new(ErrorKind::UnexpectedEof, "unexpected end of stream")),
				n => n,
			};

            readed += n;

            let start_point = if raw_left.len() < to_find.len() {
                0
            } else {
                raw_left.len() - to_find.len()
            };

            raw_left.extend_from_slice(&buffer[..n]);

            if let Some(index) = utils::find_in(&raw_left[start_point..], to_find) {
                return Ok((raw_left, Some(index)));
            } else if readed > read_limit {
                return Ok((raw_left, None));
            }
        }

        Ok((raw_left, None))
    }

    /*------------------------------------------------------------*/
    /*-------------------[ Default Upload ]-----------------------*/
    /*------------------------------------------------------------*/

    async fn upload_default_content(
        &self,
        request: &Request,
        stream: &mut TcpStream,
        raw_left: &[u8],
        upload_folder: &PathBuf,
        buffer: &mut [u8; 8196],
    ) -> Result<Vec<u8>, io::Error> {
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

        self.default_upload(&mut file, stream, request, raw_left, buffer)
            .await
    }

    async fn default_upload(
        &self,
        file: &mut File,
        stream: &mut TcpStream,
        request: &Request,
        raw_left: &[u8],
        buffer: &mut [u8; 8196],
    ) -> Result<Vec<u8>, io::Error> {
        let mut read = 0;
        let mut n = 0;
        let body_len = request.content_length().unwrap().clone() as usize;

        if body_len <= raw_left.len() {
            file.write(&raw_left[..body_len]).await?;
            return Ok(raw_left[body_len..].to_vec());
        } else {
            file.write(raw_left).await?;
            read += raw_left.len();
        }

		while read < body_len {
            n = match stream.read(buffer).await? {
				0 => return Err(io::Error::new(ErrorKind::UnexpectedEof, "stream ended")),
                n => n,
            };

			eprintln!("n: {n}	|	read: {read}");

			if n == 0 { return Err(io::Error::new(ErrorKind::UnexpectedEof, "stream ended")) }

            read += n;

            if read > body_len {
                file.write_all(&buffer[..(n - (read - body_len))]).await?;
            } else {
                file.write_all(&buffer[..n]).await?;
            }
        }

        let end = n - (read - body_len);

        return Ok(buffer[end..n].to_vec());
    }

    /*------------------------------------------------------------*/
    /*-----------------------[ Response ]-------------------------*/
    /*------------------------------------------------------------*/

    async fn build_response(&self, request: &Request) -> Result<Response, io::Error> {
        // eprintln!("Building response...");
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
        buffer: &mut [u8; 8196],
    ) -> Result<(), io::Error> {
        let mut response = self.build_response(request).await?;

        response.send(stream, buffer).await
    }

    /*------------------------------------------------------------*/
    /*-------------------------[ GET ]----------------------------*/
    /*------------------------------------------------------------*/

    async fn build_get_response(&self, request: &Request) -> Result<Response, io::Error> {
        let file = self.get_GET_request_file(request).await?;

        // eprintln!("GET response build...");
        let mut response = Response::new(ResponseCode::default()).file(file);

        response.add_header("Content-Type".to_owned(), "text/html".to_owned());

        Ok(response)
    }

    #[allow(non_snake_case)]
    async fn get_GET_request_file(&self, request: &Request) -> io::Result<File> {
        // eprintln!("trying to open '{}'...", request.path().display());
        if request.path().is_file() {
            match File::open(request.path()).await {
                Ok(file) => Ok(file),
                Err(err) => Err(err),
            }
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
        raw_left: &mut [u8],
        buffer: &mut [u8; 8196],
    ) -> Result<(Output, Vec<u8>), io::Error> {
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

        let raw_left =
            utils::send_body_to_cgi(request, stream, &mut child, raw_left, buffer).await?;

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
