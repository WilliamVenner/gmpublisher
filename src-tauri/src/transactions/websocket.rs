use crate::NTStringWriter;
use byteorder::{BigEndian, WriteBytesExt};
use crossbeam::channel::{Receiver, Sender};
use std::{
	convert::TryInto,
	net::{TcpListener, TcpStream},
};
use websocket::{
	server::{NoTlsAcceptor, WsServer},
	sync::{Client, Server},
	OwnedMessage,
};

const CANCEL_TRANSACTION: &[u8] = &[123];

enum WebSocketMessage {
	OwnedMessage(OwnedMessage),
	TransactionMessage(TransactionMessage),
}
impl Into<OwnedMessage> for WebSocketMessage {
	fn into(self) -> OwnedMessage {
		match self {
			WebSocketMessage::OwnedMessage(message) => message,
			WebSocketMessage::TransactionMessage(message) => OwnedMessage::Binary(message.as_bytes()),
		}
	}
}

pub enum TransactionMessage {
	Finished(u32, serde_json::Value),
	Error(u32, String, serde_json::Value),
	Data(u32, serde_json::Value),
	Status(u32, String),
	Progress(u32, u16),
	IncrProgress(u32, u16),
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
		}

		bytes
	}
}
impl Into<OwnedMessage> for TransactionMessage {
	fn into(self) -> OwnedMessage {
		OwnedMessage::Binary(self.as_bytes())
	}
}

pub struct TransactionServer {
	pub port: u16,
	tx: Sender<WebSocketMessage>,
}
impl TransactionServer {
	pub fn init() -> Result<TransactionServer, anyhow::Error> {
		let socket = Server::bind("0.0.0.0:0")?;
		let addr = socket.local_addr()?;

		let (tx, rx) = crossbeam::channel::unbounded::<WebSocketMessage>();

		let tx_clone = tx.clone();
		std::thread::spawn(move || TransactionServer::accept(socket, tx_clone, rx));

		Ok(TransactionServer { port: addr.port(), tx })
	}

	fn accept(mut socket: WsServer<NoTlsAcceptor, TcpListener>, tx: Sender<WebSocketMessage>, rx: Receiver<WebSocketMessage>) {
		loop {
			dprintln!("WebSocket: Waiting for client on {:?}", socket.local_addr().unwrap());
			if let Ok(connection) = socket.accept() {
				if connection.protocols().contains(&"gmpublisher".to_string()) {
					if let Ok(client) = connection.use_protocol("gmpublisher").accept() {
						dprintln!("WebSocket: Connection Established with {:#?}", client.peer_addr().unwrap());
						TransactionServer::listen(tx.clone(), &rx, client);
					}
				} else {
					dprintln!("WebSocket Error: Invalid Protocol");
				}
			}
		}
	}

	fn listen(tx: Sender<WebSocketMessage>, rx: &Receiver<WebSocketMessage>, client: Client<TcpStream>) {
		let (mut receiver, mut sender) = client.split().unwrap();

		std::thread::spawn(move || loop {
			let message = match receiver.recv_message() {
				Ok(message) => message,
				Err(err) => {
					match &err {
						websocket::WebSocketError::NoDataAvailable => continue,
						websocket::WebSocketError::IoError(error) => match error.kind() {
							std::io::ErrorKind::ConnectionReset => break,
							_ => {}
						},
						_ => {}
					};

					dprintln!("WebSocketError: {:#?}", err);
					continue;
				}
			};

			match message {
				OwnedMessage::Close(_) => {
					dprintln!("WebSocket Closed");
					tx.send(WebSocketMessage::OwnedMessage(OwnedMessage::Close(None))).unwrap();
					return;
				}

				OwnedMessage::Ping(ping) => {
					tx.send(WebSocketMessage::OwnedMessage(OwnedMessage::Pong(ping))).unwrap();
				}

				OwnedMessage::Binary(bytes) => {
					if &bytes[0..0] == CANCEL_TRANSACTION {
						match &bytes[1..9].try_into() {
							Ok(bytes) => {
								super::cancel_transaction(u32::from_be_bytes(*bytes));
								dprintln!("[1] WebSocket Invalid Message: {:?}", bytes);
							}
							Err(bytes) => {
								dprintln!("[0] WebSocket Invalid Message: {:?}", bytes);
								continue;
							}
						}
					}
				}

				OwnedMessage::Text(text) => {
					#[cfg(debug_assertions)]
					println!("WebSocket Message: {}", text);
					#[cfg(not(debug_assertions))]
					unreachable!();
				}

				_ => {}
			}
		});

		while let Ok(message) = rx.recv() {
			match message {
				WebSocketMessage::OwnedMessage(message) => match message {
					OwnedMessage::Binary(_) => {
						ignore! { sender.send_message(&message) };
					}
					OwnedMessage::Close(_) => break,
					_ => unreachable!(),
				},

				WebSocketMessage::TransactionMessage(message) => {
					if let Err(_) = sender.send_message::<OwnedMessage>(&OwnedMessage::Binary(message.as_bytes())) {
						TransactionServer::send_tauri_event(message);
					}
				}
			}
		}
	}

	pub fn send(&'static self, message: TransactionMessage) {
		if let Err(err) = self.tx.send(WebSocketMessage::TransactionMessage(message)) {
			TransactionServer::send_tauri_event(match err.into_inner() {
				WebSocketMessage::TransactionMessage(message) => message,
				_ => unreachable!(),
			});
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
		}
	}
}
