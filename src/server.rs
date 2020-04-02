use crate::connection::{ConnState, Connection};
pub use http::status::StatusCode;
use http::{response::Builder, Request, Response};
use mio::net::TcpListener;
use mio::{Events, Interest, Poll, Token};
use std::{collections::HashMap, io::ErrorKind};

pub type Res<T> = Result<T, Box<dyn std::error::Error>>;

const SERVER: Token = Token(0);

pub struct Server<'a> {
    handle: Box<dyn FnMut(Request<String>, Builder) -> Res<Response<String>> + 'a>,
}

impl<'a> Server<'a> {
    pub fn new(handle: impl FnMut(Request<String>, Builder) -> Res<Response<String>> + 'a) -> Self {
        Server {
            handle: Box::new(handle),
        }
    }
    pub fn listen(&mut self, addr: &str) -> Res<()> {
        let mut listener = TcpListener::bind(addr.parse()?)?;
        let mut poll = Poll::new()?;
        let mut events = Events::with_capacity(128);

        poll.registry()
            .register(&mut listener, SERVER, Interest::READABLE)?;

        let mut connections = HashMap::new();
        let mut next_token = Token(SERVER.0);

        // Start an EventLoop!
        loop {
            // Block forever until requests come in
            poll.poll(&mut events, None)?;

            for event in &events {
                match event.token() {
                    SERVER => loop {
                        let (mut stream, _addr) = match listener.accept() {
                            Ok(conn) => conn,
                            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                                break;
                            }
                            Err(e) => Err(e)?,
                        };

                        next_token = Token(next_token.0 + 1);

                        poll.registry()
                            .register(&mut stream, next_token, Interest::READABLE)?;

                        connections.insert(next_token, Connection::new(stream));
                    },
                    token => {
                        if let Some(connection) = connections.get_mut(&token) {
                            match connection.handle_event(event, &mut self.handle) {
                                Ok(ConnState::Read) => {}
                                Ok(ConnState::Write) => {}
                                Ok(ConnState::ReadDone) => {
                                    let _ = poll.registry().reregister(
                                        &mut connection.stream,
                                        token,
                                        Interest::WRITABLE,
                                    );
                                    connection.state = ConnState::Write;
                                }
                                Ok(ConnState::WriteDone) | Err(_) => {
                                    connections.remove(&token);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
