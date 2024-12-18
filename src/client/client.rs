// use std::sync::{Arc, Mutex};

// use tokio::net::TcpStream;

// use crate::request::request::Request;

// #[derive(Debug, Clone)]
// #[allow(dead_code)]
// pub struct Client {
//     stream: Arc<Mutex<TcpStream>>,
//     request: Option<Request>,
//     ready_to_response: bool,
//     response_code: u16,
//     error_message: Option<String>,
// }

// #[allow(dead_code)]
// impl Client {
//     pub fn new(stream: TcpStream) -> Self {
//         Self {
//             stream: Arc::new(Mutex::new(stream)),
//             ready_to_response: false,
//             response_code: 200,
//             request: None,
//             error_message: None,
//         }
//     }

//     // pub async fn read_header(&mut self) {
//     //     let mut buffer = [0; 65536];

//     //     if self.request().as_ref().is_some()
//     //         && self.request.as_ref().unwrap().state() != &State::Undefined
//     //     {
//     //         // not First Request
//     //         let readed = self
//     //             .stream
//     //             .lock()
//     //             .unwrap()
//     //             .read(&mut buffer)
//     //             .await
//     //             .expect("failed to receive request !");
//     //         let mut request = self.request.as_ref().unwrap().clone();
//     //         match request.push(String::from_utf8_lossy(&buffer[..readed]).into_owned()) {
//     //             Ok(_) => {}
//     //             Err(err_code) => {
//     //                 self.response_code = err_code;
//     //                 self.ready_to_response = true;
//     //             }
//     //         }
//     //     } else {
//     //         // First Request
//     //         let readed = self
//     //             .stream
//     //             .lock()
//     //             .unwrap()
//     //             .read(&mut buffer)
//     //             .await
//     //             .expect("failed to receive request !");
//     //         self.request =
//     //             match Request::try_from(String::from_utf8_lossy(&buffer[..readed]).into_owned()) {
//     //                 Ok(request) => Some(request),
//     //                 Err(_) => {
//     //                     // self.stream.lock().unwrap().write(format!("HTTP/1.1 {code} OK\r\n\r\n{str}\r\n").as_bytes()).await.expect("failed to send response");
//     //                     return;
//     //                 }
//     //             };
//     //     }

//     //     //	sending RESPONSE
//     //     // self.stream.write(format!("HTTP/1.1 {response_code} OK\r\n\r\nHello from server !\r\n").as_bytes()).await.expect("failed to send response");
//     // }

//     pub fn request(&mut self) -> Option<&Request> {
//         self.request.as_ref()
//     }

//     pub fn ready_to_response(&self) -> bool {
//         self.ready_to_response
//     }

//     pub fn response_code(&self) -> u16 {
//         self.response_code
//     }

//     pub fn stream(&self) -> &Mutex<TcpStream> {
//         &self.stream
//     }
// }
