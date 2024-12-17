use std::{
    collections::HashMap,
    fmt::format,
    hash::Hash,
    io::{Error, ErrorKind},
    path::{Path, PathBuf},
};

use lazy_static::lazy_static;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use super::Body;

#[derive(Default, Debug)]
pub struct Response {
    code: ResponseCode,
    headers: HashMap<String, String>,
    file: Option<File>,
    body: Option<String>,
}

impl Response {
    pub fn new(code: ResponseCode) -> Response {
        Response {
            code: code,
            headers: HashMap::new(),
            file: None,
            body: None,
        }
    }

    // pub async fn send_to(&mut self, stream: &mut TcpStream) -> Result<(), Error> {
    //     let header = self.serialize_header().await?;

    //     let _ = stream.write_all(header.as_bytes()).await?;

    //     if self.file.is_none() {
    //         return Ok(());
    //     }

    //     let mut buffer = [0; 65536];
    //     loop {
    //         let n = self.file.as_mut().unwrap().read(&mut buffer).await?;
    //         if n == 0 {
    //             break;
    //         }

    //         let _ = stream.write_all(&buffer[..n]).await?;
    //     }

    //     stream.write(b"\r\n").await?;

    //     Ok(())
    // }

    // pub async fn from_file(code: ResponseCode, file: &Path) -> Result<Response, ErrorKind> {
    //     // if file.is_dir() {
    //     //     // TODO
    //     //     return Err(ErrorKind::NotFound);
    //     // } else if !file.is_file() {
    //     //     return Err(ErrorKind::NotFound);
    //     // }

    //     let file = match File::open(file).await {
    //         Ok(file) => file,
    //         Err(err) => return Err(err.kind()),
    //     };

    //     Ok(Response {
    //         code,
    //         headers: HashMap::new(),
    //         file: Some(file),
    //     })
    // }

    async fn serialize_header(&self, file: File) -> Result<String, Error> {
        let mut response: String = format!(
            "HTTP/1.1 {} {}\r\n",
            self.code.code(),
            self.code.to_string()
        );

		if self.body.is_some() {
			let file_length = format!(
				"Content-Length: {}\r\n",
				file.metadata().await?.len()
			);
			response = format!("{}{}", response, file_length);
		}

        let headers: String = self
            .headers
            .iter()
            .map(|(key, value)| format!("{key}: {value}"))
            .collect::<Vec<String>>()
            .join("\r\n");


        Ok(format!("{response}{headers}\r\n"))
    }

    pub fn file(&mut self, file: File) {
        self.file = Some(file);
    }
}

#[derive(Clone, Debug)]
pub struct ResponseCode {
    code: u16,
}

impl Default for ResponseCode {
    fn default() -> Self {
        ResponseCode { code: 200 }
    }
}

impl ResponseCode {
    pub fn new(code: u16) -> ResponseCode {
        ResponseCode { code }
    }

    pub fn from_error(err: ErrorKind) -> ResponseCode {
        ResponseCode {
            code: match err {
                ErrorKind::NotFound => 404,          // Not Found
                ErrorKind::PermissionDenied => 403,  // Forbidden
                ErrorKind::ConnectionRefused => 503, // Service Unavailable
                ErrorKind::TimedOut => 524,          // a Timeout occured
                ErrorKind::WriteZero => 500,         // Internal Server Error
                ErrorKind::Interrupted => 500,       // Internal Server Error
                _ => 500,                            // Default to Internal Server Error
            },
        }
    }

    pub fn into_error(&self) -> ErrorKind {
		match self.code {
			404 => ErrorKind::NotFound,          // Not Found
			_ => ErrorKind::Other,                            // Default to Internal Server Error
		}
    }

    pub fn to_string<'a>(&self) -> &'a str {
        match HTTP_CODES.get(&self.code) {
            Some(msg) => msg,
            None => "Unknow Error code",
        }
    }

    pub fn code(&self) -> u16 {
        self.code
    }

    fn set_code(&mut self, code: u16) {
        self.code = code;
    }
}

lazy_static! {
    pub static ref HTTP_CODES: HashMap<u16, &'static str> = {
        let mut m = HashMap::new();

        // Information Responses (1xx)
        m.insert(100, "Continue");
        m.insert(101, "Switching Protocols");
        m.insert(102, "Processing");
        m.insert(103, "Early Hints");

        // Successful Responses (2xx)
        m.insert(200, "OK");
        m.insert(201, "Created");
        m.insert(202, "Accepted");
        m.insert(203, "Non-Authoritative Information");
        m.insert(204, "No Content");
        m.insert(205, "Reset Content");
        m.insert(206, "Partial Content");
        m.insert(207, "Multi-Status");
        m.insert(208, "Already Reported");
        m.insert(226, "IM Used");

        // Redirection Messages (3xx)
        m.insert(300, "Multiple Choices");
        m.insert(301, "Moved Permanently");
        m.insert(302, "Found");
        m.insert(303, "See Other");
        m.insert(304, "Not Modified");
        m.insert(305, "Use Proxy");
        m.insert(307, "Temporary Redirect");
        m.insert(308, "Permanent Redirect");

        // Client Error Responses (4xx)
        m.insert(400, "Bad Request");
        m.insert(401, "Unauthorized");
        m.insert(402, "Payment Required");
        m.insert(403, "Forbidden");
        m.insert(404, "Not Found");
        m.insert(405, "Method Not Allowed");
        m.insert(406, "Not Acceptable");
        m.insert(407, "Proxy Authentication Required");
        m.insert(408, "Request Timeout");
        m.insert(409, "Conflict");
        m.insert(410, "Gone");
        m.insert(411, "Length Required");
        m.insert(412, "Precondition Failed");
        m.insert(413, "Payload Too Large");
        m.insert(414, "URI Too Long");
        m.insert(415, "Unsupported Media Type");
        m.insert(416, "Range Not Satisfiable");
        m.insert(417, "Expectation Failed");
        m.insert(418, "I'm a Teapot");
        m.insert(421, "Misdirected Request");
        m.insert(422, "Unprocessable Entity");
        m.insert(423, "Locked");
        m.insert(424, "Failed Dependency");
        m.insert(425, "Too Early");
        m.insert(426, "Upgrade Required");
        m.insert(428, "Precondition Required");
        m.insert(429, "Too Many Requests");
        m.insert(431, "Request Header Fields Too Large");
        m.insert(451, "Unavailable For Legal Reasons");

        // Server Error Responses (5xx)
        m.insert(500, "Internal Server Error");
        m.insert(501, "Not Implemented");
        m.insert(502, "Bad Gateway");
        m.insert(503, "Service Unavailable");
        m.insert(504, "Gateway Timeout");
        m.insert(505, "HTTP Version Not Supported");
        m.insert(506, "Variant Also Negotiates");
        m.insert(507, "Insufficient Storage");
        m.insert(508, "Loop Detected");
        m.insert(510, "Not Extended");
        m.insert(511, "Network Authentication Required");

        m
    };
}
