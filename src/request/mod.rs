use std::{collections::HashMap, io, path::PathBuf, slice::Iter};

use crate::response::response::ResponseCode;

/*------------------------------------------------------------------------------------*/
/*										REQUEST										  */
/*------------------------------------------------------------------------------------*/

#[derive(Debug, Clone, Default)]
pub struct RequestError {
    invalid_request: bool,
    io_error: bool,
    io_error_kind: Option<io::ErrorKind>,
    error_string: String,
}

impl From<io::Error> for RequestError {
    fn from(value: io::Error) -> Self {
        let mut request = RequestError::default();
        request.io_error = true;
        request.io_error_kind = Some(value.kind());
        request.error_string = value.to_string();

        request
    }
}

impl From<String> for RequestError {
    fn from(value: String) -> Self {
        let mut request = RequestError::default();
        request.invalid_request = true;
        request.error_string = value;

        request
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Request {
    method: Method,
    http_version: String,
    path: PathBuf,
    query: Option<String>,
    accept: Option<String>,
    host: Option<String>,
    headers: HashMap<String, String>,
    content_length: Option<usize>,
    content_type: Option<String>,
    raw_body: Option<String>,
    raw_header: String,
    state: State,
    keep_connection_alive: bool,
}

impl Default for Request {
    fn default() -> Self {
        Request {
            query: Option::default(),
            method: Method::default(),
            http_version: String::default(),
            path: PathBuf::default(),
            accept: Option::default(),
            host: Option::default(),
            headers: HashMap::default(),
            content_length: Option::default(),
            content_type: Option::default(),
            raw_body: Option::default(),
            raw_header: String::default(),
            state: State::default(),
            keep_connection_alive: true,
        }
    }
}

impl TryFrom<&[u8]> for Request {
    type Error = ResponseCode;
    fn try_from(value: &[u8]) -> Result<Request, Self::Error> {
		let value = String::from_utf8_lossy(value);
        let headers = value.split("\r\n").map(|str| str.to_string()).collect();
        match Self::deserialize(headers) {
            Ok(request) => Ok(request),
            Err(err) => Err(ResponseCode::new(400, err.as_str())),
        }
    }
}

impl Request {
    // pub fn push(&mut self, request: &str) -> Result<Option<String>, u16> {
    //	 if self.state != State::Undefined && self.state != State::OnHeader {
    //		 todo!()
    //	 }

    // 	self.state = State::OnHeader;
    //	 let (header, rest) = match request.split_once("\r\n\r\n") {
    //		 None => {
    //			 // Header not finished
    //			 self.raw_header.push_str(&request);
    //			 return Ok(None);
    //		 }
    //		 Some((header, rest)) => (header, rest),
    //	 };
    //	 // Header complete
    //	 self.raw_header.push_str(header);
    //	 if rest.is_empty() == false {
    // 		self.raw_body = Some(rest.to_owned());
    //	 }

    //	 self.deserialize()?;
    //	 self.raw_header.clear();
    //	 self.state = if self.raw_body.is_some() {
    // 		State::OnBody
    //	 } else {
    //		 State::Finished
    //	 };

    //	 Ok(())
    // }

    // async fn from(mut stream: &mut TcpStream) -> Result<Self, RequestError> {
    // 	let headers = Self::read_header_from(&mut stream).await?;

    // 	let request = match Self::deserialize(headers) {
    // 		Ok(request) => request,
    // 		Err(err) => return Err(RequestError::from(err)),
    // 	};

    // 	Ok(request)
    // }

    // async fn read_header_from(mut stream: &mut TcpStream) -> io::Result<Vec<String>> {
    // 	let mut headers = vec![];
    // 	let mut size = 0;

    // 	while size < 4096 {
    // 		stream.readable().await;
    // 		let reader = BufReader::new(&mut stream);
    // 		let mut lines = reader.lines();

    // 		while let Some(line) = lines.next_line().await? {
    // 			if line.is_empty() { return Ok(headers) }
    // 			size += line.as_bytes().len();
    // 			headers.push(line);
    // 		}
    // 	}

    // 	Err(io::Error::new(io::ErrorKind::FileTooLarge, "header too large: expected less than 4096 bytes"))

    // }

    fn deserialize(headers: Vec<String>) -> Result<Self, String> {
        eprintln!("Deserializing header...");
        let mut headers: Iter<'_, String> = headers.iter();
        let first_line = headers.next();
        if first_line.is_none() {
            return Err("empty header".to_owned());
        }

        let mut request = Request::default();

        request.parse_first_line(first_line.unwrap())?;
        request.parse_other_lines(headers)?;

        Ok(request)
    }

    fn parse_other_lines(&mut self, headers: Iter<'_, String>) -> Result<(), String> {
        for header in headers {
            if header.is_empty() {
                break;
            } // end of header

            let split = header.split_once(":");
            if split.is_none() {
                return Err(format!("invalid header: {}", header));
            }

            let name = split.unwrap().0;
            let value = split.unwrap().1;

            match name {
                "Host" => {
                    if self.host.is_none() {
                        self.host = Some(value.trim().to_owned())
                    } else {
                        return Err("duplicate header: Host".to_owned());
                    }
                }
                "Connection" => {
                    if value == "close" {
                        self.keep_connection_alive = false
                    }
                }
                "Accept" => {
                    if self.accept.is_none() {
                        self.accept = Some(value.to_owned())
                    }
                    // set if don't exists
                    else {
                        self.accept = Some(format!("{} {value}", self.accept.as_ref().unwrap()))
                    } // concat a space (' ') and the value if already exists
                }
                "Content-Length" => {
                    if self.content_length.is_some() {
                        return Err("invalid header: Content-Length: duplicated header".to_string());
                    } else {
                        self.content_length = match value.trim().parse::<usize>() {
							Ok(len) => Some(len),
							Err(err) => return Err(format!("invalid header: Content-Length ({value}): failed to convert value: {err}")),
						};
                    }
                }
                "Content-Type" => {
                    if self.content_type.is_some() {
                        return Err("invalid header: Content-type: duplicated header".to_string());
                    } else {
                        self.content_type = Some(value.to_string());
                    }
                }
                _ => {
                    self.headers
                        .entry(name.to_owned()) // finding key name
                        .and_modify(|val|	// modify if exists
						val.push_str(format!("  {value}").as_str()))
                        .or_insert(value.to_owned()); // else, insert
                }
            }
        }

        Ok(())
    }

    fn parse_first_line(&mut self, line: &str) -> Result<(), String> {
        let split: Vec<&str> = line.split_whitespace().collect();

        if split.len() != 3 {
            return Err(format!("invalid header: first line invalid: [{line}]"));
        } // Bad Request

        let method = split[0];
        self.method = match Method::try_from_str(method) {
            Ok(method) => method,
            Err(_) => Method::UNKNOWN,
        };

        self.path = PathBuf::from(split[1]);
        self.http_version = split[2].to_owned();
        Ok(())
    }

    fn add_path(&mut self, path: &str) {
        if let Some(query_pos) = path.find("?") {
            self.query = Some(path[query_pos + 1..].to_string());
            self.path = PathBuf::from(&path[..query_pos]);
        } else {
            self.path = PathBuf::from(path)
        }
    }

    pub fn get(&self, header: &str) -> Option<&String> {
        match self.headers.get(header) {
            None => None,
            Some(value) => Some(value),
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.path = path
    }

    pub fn accept(&self) -> Option<&String> {
        self.accept.as_ref()
    }

    pub fn host(&self) -> Option<&String> {
        self.host.as_ref()
    }

    pub fn content_length(&self) -> Option<&usize> {
        self.content_length.as_ref()
    }

    pub fn keep_connection_alive(&self) -> bool {
        self.keep_connection_alive
    }

    pub fn http_version(&self) -> &str {
        &self.http_version
    }

    pub fn content_type(&self) -> Option<&String> {
        self.content_type.as_ref()
    }

    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    pub fn query(&self) -> Option<&String> {
        self.query.as_ref()
    }
}

/*------------------------------------------------------------------------------------*/
/*										METHOD										  */
/*------------------------------------------------------------------------------------*/

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Method {
    UNDEFINED,
    GET,
    POST,
    DELETE,
    OPTIONS,
    HEAD,
    PUT,
    PATCH,
    TRACE,
    CONNECT,
    UNKNOWN,
}

impl Default for Method {
    fn default() -> Self {
        Self::UNDEFINED
    }
}

impl From<&str> for Method {
    fn from(method: &str) -> Self {
        match method {
            "GET" => Method::GET,
            "POST" => Method::POST,
            "DELETE" => Method::DELETE,
            "OPTIONS" => Method::OPTIONS,
            "HEAD" => Method::HEAD,
            "PUT" => Method::PUT,
            "CONNECT" => Method::CONNECT,
            "PATCH" => Method::PATCH,
            "TRACE" => Method::TRACE,
            _ => Method::UNKNOWN,
        }
    }
}

impl<'a> TryInto<&'a str> for Method {
    type Error = ();
    fn try_into(self) -> Result<&'a str, Self::Error> {
        match self {
            Method::GET => Ok("GET"),
            Method::POST => Ok("POST"),
            Method::DELETE => Ok("DELETE"),
            Method::HEAD => Ok("HEAD"),
            Method::PUT => Ok("PUT"),
            Method::CONNECT => Ok("CONNECT"),
            Method::PATCH => Ok("PATCH"),
            Method::TRACE => Ok("TRACE"),
            Method::OPTIONS => Ok("OPTIONS"),
            Method::UNDEFINED | Method::UNKNOWN => Err(()),
        }
    }
}

impl Method {
    pub fn try_from_str(method: &str) -> Result<Self, String> {
        match method {
            "GET" => Ok(Method::GET),
            "POST" => Ok(Method::POST),
            "DELETE" => Ok(Method::DELETE),
            "OPTIONS" => Ok(Method::OPTIONS),
            "HEAD" => Ok(Method::HEAD),
            "PUT" => Ok(Method::PUT),
            "CONNECT" => Ok(Method::CONNECT),
            "PATCH" => Ok(Method::PATCH),
            "TRACE" => Ok(Method::TRACE),
            _ => Err("unknown method".to_string()),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Method::GET => "GET".to_owned(),
            Method::POST => "POST".to_owned(),
            Method::DELETE => "DELETE".to_owned(),
            Method::HEAD => "HEAD".to_owned(),
            Method::PUT => "PUT".to_owned(),
            Method::CONNECT => "CONNECT".to_owned(),
            Method::PATCH => "PATCH".to_owned(),
            Method::TRACE => "TRACE".to_owned(),
            Method::OPTIONS => "OPTIONS".to_owned(),
            Method::UNDEFINED | Method::UNKNOWN => "UNKNOWN".to_owned(),
        }
    }
}

/*------------------------------------------------------------------------------------*/
/*										STATE										  */
/*------------------------------------------------------------------------------------*/

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub enum State {
    #[default]
    Undefined,
    OnHeader,
    OnBody,
    Finished,
}

impl State {
    pub fn is(&self, other: Self) -> bool {
        self.eq(&other)
    }
    pub fn is_not(&self, other: Self) -> bool {
        self.eq(&other)
    }
}

mod utils {
    use lazy_static::lazy_static;
    use std::collections::HashMap;

    lazy_static! {
        static ref CONTENT_TYPE_TO_EXT: HashMap<&'static str, &'static str> = {
            let mut m = HashMap::new();
            m.insert("application/json", "json");
            m.insert("application/javascript", "js");
            m.insert("text/html", "html");
            m.insert("text/css", "css");
            m.insert("text/plain", "txt");
            m.insert("text/csv", "csv");
            m.insert("image/jpeg", "jpg");
            m.insert("image/png", "png");
            m.insert("image/gif", "gif");
            m.insert("image/svg+xml", "svg");
            m.insert("audio/mpeg", "mp3");
            m.insert("audio/wav", "wav");
            m.insert("video/mp4", "mp4");
            m.insert("application/pdf", "pdf");
            m.insert("application/xml", "xml");
            m.insert("application/zip", "zip");
            m
        };
        static ref EXT_TO_CONTENT_TYPE: HashMap<&'static str, &'static str> = {
            let mut m = HashMap::new();
            for (content_type, ext) in CONTENT_TYPE_TO_EXT.iter() {
                m.insert(*ext, *content_type);
            }
            m
        };
    }

    /// Convertit un Content-Type en extension de fichier
    /// Retourne None si le Content-Type n'est pas reconnu
    pub fn content_type_to_extension(content_type: &str) -> Option<&'static str> {
        CONTENT_TYPE_TO_EXT.get(content_type).copied()
    }

    /// Convertit une extension de fichier en Content-Type
    /// Retourne None si l'extension n'est pas reconnue
    pub fn extension_to_content_type(extension: &str) -> Option<&'static str> {
        let ext = extension.trim_start_matches('.');
        EXT_TO_CONTENT_TYPE.get(ext).copied()
    }
}

/*-----------------ERROR CODES-----------------*/

// codes_responses[100] = "Continue";
// codes_responses[101] = "Switching Protocols";
// codes_responses[102] = "Processing";
// codes_responses[103] = "Early Hints";
// codes_responses[200] = "OK";
// codes_responses[201] = "Created";
// codes_responses[202] = "Accepted";
// codes_responses[203] = "Non-Authoritative Information";
// codes_responses[204] = "No Content";
// codes_responses[205] = "Reset Content";
// codes_responses[206] = "Partial Content";
// codes_responses[207] = "Multi-Status";
// codes_responses[208] = "Already Reported";
// codes_responses[210] = "Content Different";
// codes_responses[226] = "IM Used";
// codes_responses[300] = "Multiple Choices";
// codes_responses[301] = "Moved Permanently";
// codes_responses[302] = "Found";
// codes_responses[303] = "See Other";
// codes_responses[304] = "Not Modified";
// codes_responses[305] = "Use Proxy";
// codes_responses[307] = "Temporary Redirect";
// codes_responses[308] = "Permanent Redirect";
// codes_responses[310] = "Too many Redirects";
// codes_responses[400] = "Bad Request";
// codes_responses[401] = "Unauthorized";
// codes_responses[402] = "Payment Required";
// codes_responses[403] = "Forbidden";
// codes_responses[404] = "Not Found";
// codes_responses[405] = "Method Not Allowed";
// codes_responses[406] = "Not Acceptable";
// codes_responses[407] = "Proxy Authentication Required";
// codes_responses[408] = "Request Time-out";
// codes_responses[409] = "Conflict";
// codes_responses[410] = "Gone";
// codes_responses[411] = "Length Required";
// codes_responses[412] = "Precondition Failed";
// codes_responses[413] = "Request Entity Too Large";
// codes_responses[414] = "Request-URI Too Long";
// codes_responses[415] = "Unsupported Media Type";
// codes_responses[416] = "Requested range unsatisfiable";
// codes_responses[417] = "Expectation failed";
// codes_responses[418] = "I'm a teapot";
// codes_responses[419] = "Page expired";
// codes_responses[421] = "Bad mapping / Misdirected Request";
// codes_responses[422] = "Unprocessable entity";
// codes_responses[423] = "Locked";
// codes_responses[424] = "Method failure";
// codes_responses[425] = "Too Early";
// codes_responses[426] = "Upgrade Required";
// codes_responses[427] = "Invalid digital signature";
// codes_responses[428] = "Precondition Required";
// codes_responses[429] = "Too Many Requests";
// codes_responses[431] = "Request Header Fields Too Large";
// codes_responses[449] = "Retry With";
// codes_responses[450] = "Blocked by Windows Parental Controls";
// codes_responses[451] = "Unavailable For Legal Reasons";
// codes_responses[456] = "Unrecoverable Erstatus()";
// codes_responses[500] = "Internal Server Error";
// codes_responses[501] = "Method Not Implemented";
// codes_responses[505] = "HTTP Version not supported";
// codes_responses[506] = "Variant Also Negotiates";
// codes_responses[507] = "Insufficient storage";
// codes_responses[508] = "Loop detected";
// codes_responses[509] = "Bandwidth Limit Exceeded";
// codes_responses[510] = "Not extended";
// codes_responses[511] = "Network authentication required";
// codes_responses[520] = "Unknown Error";
// codes_responses[521] = "Web Server Is Down";
// codes_responses[522] = "Connection Timed Out";
// codes_responses[523] = "Origin Is Unreachable";
// codes_responses[524] = "A Timeout Occurred";
// codes_responses[525] = "SSL Handshake Failed";
// codes_responses[526] = "Invalid SSL Certificate";
// codes_responses[527] = "Railgun Error";
