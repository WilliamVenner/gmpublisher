use crate::NTStringWriter;
use byteorder::{BigEndian, WriteBytesExt};
use crossbeam::channel::{Receiver, Sender};
use std::{
	net::{TcpListener, TcpStream},
	time::Duration,
};
use tungstenite::{
	handshake::server::{ErrorResponse, Request, Response},
	http::{HeaderValue, StatusCode},
	Message, WebSocket,
};

const CANCEL_TRANSACTION: &[u8] = &[123];

pub enum TransactionMessage {
	Finished(u32, serde_json::Value),
	Error(u32, String, serde_json::Value),
	Data(u32, serde_json::Value),
	Status(u32, String),
	Progress(u32, u16),
	IncrProgress(u32, u16),
	ResetProgress(u32),
}
impl TransactionMessage {
	fn write_json(bytes: &mut Vec<u8>, json: &serde_json::Value) {
		if matches!(json, serde_json::Value::Null) {
			bytes.write_u8(0).unwrap();
		} else {
			bytes.write_u8(1).unwrap(); // indicate that it's JSON
			bytes.write_nt_string(serde_json::to_string(json).unwrap()).unwrap();
		}
	}

	fn as_bytes(&self) -> Vec<u8> {
		let mut bytes = Vec::new();

		match self {
			TransactionMessage::Finished(id, json) => {
				bytes.write_u8(0).unwrap();
				bytes.write_u32::<BigEndian>(*id).unwrap();
				TransactionMessage::write_json(&mut bytes, json);
			}
			TransactionMessage::Error(id, msg, json) => {
				bytes.write_u8(1).unwrap();
				bytes.write_u32::<BigEndian>(*id).unwrap();
				bytes.write_nt_string(msg).unwrap();
				TransactionMessage::write_json(&mut bytes, json);
			}
			TransactionMessage::Data(id, json) => {
				bytes.write_u8(2).unwrap();
				bytes.write_u32::<BigEndian>(*id).unwrap();
				TransactionMessage::write_json(&mut bytes, json);
			}
			TransactionMessage::Status(id, status) => {
				bytes.write_u8(3).unwrap();
				bytes.write_u32::<BigEndian>(*id).unwrap();
				bytes.write_nt_string(status).unwrap();
			}
			TransactionMessage::Progress(id, progress) => {
				bytes.write_u8(4).unwrap();
				bytes.write_u32::<BigEndian>(*id).unwrap();
				bytes.write_u16::<BigEndian>(*progress).unwrap();
			}
			TransactionMessage::IncrProgress(id, incr) => {
				bytes.write_u8(5).unwrap();
				bytes.write_u32::<BigEndian>(*id).unwrap();
				bytes.write_u16::<BigEndian>(*incr).unwrap();
			}
			TransactionMessage::ResetProgress(id) => {
				bytes.write_u8(6).unwrap();
				bytes.write_u32::<BigEndian>(*id).unwrap();
			}
		}

		bytes
	}
}

pub struct TransactionServer {
	pub port: u16,
	tx: Sender<TransactionMessage>,
}
impl TransactionServer {
	pub fn init() -> Result<TransactionServer, anyhow::Error> {
		let listener = TcpListener::bind("127.0.0.1:0")?;
		let addr = listener.local_addr()?;

		let (tx, rx) = crossbeam::channel::unbounded::<TransactionMessage>();

		std::thread::spawn(move || TransactionServer::accept(listener, rx));

		Ok(TransactionServer { port: addr.port(), tx })
	}

	fn accept(listener: TcpListener, rx: Receiver<TransactionMessage>) {
		let negotiate = |req: &Request, mut resp: Response| -> Result<Response, ErrorResponse> {
			let supported = req
				.headers()
				.get_all("Sec-WebSocket-Protocol")
				.iter()
				.filter_map(|protocols| protocols.to_str().ok())
				.flat_map(|protocols| protocols.split(','))
				.any(|protocol| protocol.trim() == "gmpublisher");
			if supported {
				resp.headers_mut()
					.insert("Sec-WebSocket-Protocol", HeaderValue::from_static("gmpublisher"));
				Ok(resp)
			} else {
				let mut err = ErrorResponse::new(None);
				*err.status_mut() = StatusCode::BAD_REQUEST;
				Err(err)
			}
		};

		loop {
			dprintln!("WebSocket: Waiting for client on {:?}", listener.local_addr().unwrap());
			let stream = match listener.accept() {
				Ok((stream, _)) => stream,
				Err(_) => continue,
			};

			match tungstenite::accept_hdr(stream, negotiate) {
				Ok(client) => {
					dprintln!("WebSocket: Connection Established with {:#?}", client.get_ref().peer_addr().unwrap());
					TransactionServer::listen(&rx, client);
				}
				Err(err) => {
					dprintln!("WebSocket Error: {:#?}", err);
				}
			}
		}
	}

	fn listen(rx: &Receiver<TransactionMessage>, mut client: WebSocket<TcpStream>) {
		client.get_ref().set_read_timeout(Some(Duration::from_millis(1))).unwrap();

		loop {
			if let Ok(message) = rx.recv_timeout(Duration::from_millis(50)) {
				for message in std::iter::once(message).chain(rx.try_iter()) {
					if client.send(Message::Binary(message.as_bytes().into())).is_err() {
						TransactionServer::send_tauri_event(message);
						return;
					}
				}
			}

			loop {
				match client.read() {
					Ok(message) => match message {
						Message::Close(_) => {
							dprintln!("WebSocket Closed");
							return;
						}

						Message::Binary(bytes) => {
							if bytes.len() >= 5 && bytes.starts_with(CANCEL_TRANSACTION) {
								super::cancel_transaction(u32::from_be_bytes(bytes[1..5].try_into().unwrap()));
							}
						}

						Message::Text(text) => {
							#[cfg(debug_assertions)]
							println!("WebSocket Message: {}", text);
							#[cfg(not(debug_assertions))]
							unreachable!();
						}

						_ => {}
					},

					Err(tungstenite::Error::Io(error))
						if matches!(
							error.kind(),
							std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut | std::io::ErrorKind::Interrupted
						) =>
					{
						break
					}

					Err(err) => {
						dprintln!("WebSocketError: {:#?}", err);
						return;
					}
				}
			}
		}
	}

	pub fn send(&'static self, message: TransactionMessage) {
		if let Err(err) = self.tx.send(message) {
			TransactionServer::send_tauri_event(err.into_inner());
		}
	}

	pub fn send_tauri_event(message: TransactionMessage) {
		match message {
			TransactionMessage::Finished(id, data) => {
				webview_emit!("TransactionFinished", (id, data));
			}
			TransactionMessage::Error(id, msg, data) => {
				webview_emit!("TransactionError", (id, msg, data));
			}
			TransactionMessage::Data(id, data) => {
				webview_emit!("TransactionData", (id, data));
			}
			TransactionMessage::Status(id, status) => {
				webview_emit!("TransactionStatus", (id, status));
			}
			TransactionMessage::Progress(id, progress) => {
				webview_emit!("TransactionProgress", (id, progress));
			}
			TransactionMessage::IncrProgress(id, incr) => {
				webview_emit!("TransactionIncrProgress", (id, incr));
			}
			TransactionMessage::ResetProgress(id) => {
				webview_emit!("TransactionResetProgress", id);
			}
		}
	}
}
