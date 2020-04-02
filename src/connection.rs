use crate::{
    request::{deserialize_request, serialize_response},
    server::Res,
};
pub use http::status::StatusCode;
use http::{response::Builder, Request, Response};
use mio::event::Event;
use mio::net::TcpStream;
use std::io::{ErrorKind, Read, Write};

#[derive(Debug)]
pub enum ConnState {
    Read,
    ReadDone,
    Write,
    WriteDone,
}

#[derive(Debug)]
pub struct Connection {
    pub stream: TcpStream,
    pub state: ConnState,
    buffer: Option<String>,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        Connection {
            stream,
            state: ConnState::Read,
            buffer: None,
        }
    }
    pub fn handle_event(
        &mut self,
        event: &Event,
        handle: impl FnMut(Request<String>, Builder) -> Res<Response<String>>,
    ) -> Res<&ConnState> {
        if event.is_readable() {
            let mut buff = [0_u8; 512];
            let mut buffer: Vec<u8> = Vec::with_capacity(512);

            // Read all bytes
            // TODO: Optimize by doing partial reads (and skipping if not handled)
            loop {
                match self.stream.read(&mut buff) {
                    Ok(0) => Err("Invalid request.")?,
                    Ok(size) => buffer.extend_from_slice(&buff[..size]),
                    Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                    Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                    Err(e) => Err(e)?,
                }
            }

            self.buffer = Some(std::str::from_utf8(&buffer)?.to_string());
            self.state = ConnState::ReadDone;
        }

        if event.is_writable() {
            match send_res(&self.buffer, handle) {
                Ok(serialized) => {
                    let bytes = serialized.as_bytes();
                    match self.stream.write(bytes) {
                        Ok(b) if b < bytes.len() => return Err("Couldn't write to stream")?,
                        Ok(_b) => {}
                        Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                        Err(e) if e.kind() == ErrorKind::Interrupted => {}
                        Err(e) => send_err(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            e.to_string(),
                            &self.stream,
                        )?,
                    }
                }
                Err(e) => send_err(StatusCode::BAD_REQUEST, e.to_string(), &self.stream)?,
            }
            let _ = self.stream.flush();
            self.state = ConnState::WriteDone;
        }

        Ok(&self.state)
    }
}

fn send_err(status_code: StatusCode, msg: String, mut stream: &TcpStream) -> Res<()> {
    println!("{} - {:?}", status_code, msg);
    let response = Response::builder().status(status_code).body(msg)?;
    stream.write(serialize_response(response).as_bytes())?;
    Ok(())
}

fn send_res(
    buffer: &Option<String>,
    mut handle: impl FnMut(Request<String>, Builder) -> Res<Response<String>>,
) -> Res<String> {
    let buffer = &buffer.as_ref().ok_or("Missing HTTP Request")?;
    let request = deserialize_request(buffer)?;
    let response = (handle)(request, Response::builder())?;
    Ok(serialize_response(response))
}
