use http::header::HeaderName;
pub use http::status::StatusCode;
use http::{response::Builder, Request, Response};
use std::fmt::Debug;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream, ToSocketAddrs},
};

type Res<T> = Result<T, Box<dyn std::error::Error>>;
pub struct Server<'a> {
    handle: Box<dyn FnMut(Request<String>, Builder) -> Res<Response<String>> + 'a>,
}

impl<'a> Server<'a> {
    pub fn new(handle: impl FnMut(Request<String>, Builder) -> Res<Response<String>> + 'a) -> Self {
        Server {
            handle: Box::new(handle),
        }
    }
    pub fn listen<A: Debug + ToSocketAddrs>(&mut self, addr: A) {
        let listener = TcpListener::bind(&addr)
            .expect(&format!("Could not bind to given address: {:?}", addr));

        for stream in listener.incoming() {
            self.handle_client(stream.expect("Couldn't connect to client."));
        }
    }
    fn handle_client(&mut self, mut stream: TcpStream) {
        let mut buff = [0_u8; 512];
        let mut buffer: Vec<u8> = Vec::with_capacity(512);

        // Read all bytes
        // TODO: Optimize by doing partial reads (and skipping if not handled)
        loop {
            match stream.read(&mut buff) {
                Ok(size) => {
                    buffer.extend_from_slice(&buff[..size]);
                    if size < buff.len() {
                        break;
                    }
                }
                Err(e) => {
                    break;
                }
            }
        }

        let request = deserialize_request(&buffer).unwrap();
        let response = (self.handle)(request, Response::builder());
        match response {
            Ok(res) => {
                stream.write(serialize_response(res).as_bytes()).unwrap();
            }
            Err(e) => {
                println!("Error {:?}", e);
                let res = Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(e.to_string())
                    .unwrap();
                stream.write(serialize_response(res).as_bytes()).unwrap();
            }
        }
        stream.flush().unwrap();
    }
}

static CRLF: &'static str = "\r\n";

fn serialize_response(res: Response<String>) -> String {
    // HTTP-Version Status-Code Reason-Phrase CRLF
    // headers CRLF
    // message-body
    let (parts, body) = res.into_parts();

    format!(
        "{:?} {} {}{}{}{}{}",
        parts.version,
        parts.status.as_str(),
        parts
            .status
            .canonical_reason()
            .unwrap_or(StatusCode::BAD_REQUEST.as_str()),
        CRLF,
        parts
            .headers
            .iter()
            .map(|(k, v)| format!("{}: {}{}", k.as_str(), v.to_str().unwrap(), CRLF))
            .collect::<Vec<String>>()
            .join(""),
        CRLF,
        body
    )
}

fn deserialize_request(buff: &[u8]) -> Res<Request<String>> {
    let buff = std::str::from_utf8(buff)?;
    // Method Request-URI HTTP-Version CRLF
    // headers CRLF
    // message-body
    let mut head = buff
        .lines()
        .next()
        .ok_or("Invalid HTTP Request")?
        .split_whitespace();
    let buff: Vec<&str> = buff.lines().skip(1).collect();
    let method = head.next().ok_or("Invalid HTTP Method")?;
    let uri = head.next().ok_or("Invalid HTTP URI")?;
    match (method, uri) {
        ("GET", uri) => build_get(uri, &buff),
        ("POST", uri) => build_post(uri, &buff),
        (method, _) => Err(format!("Unsupported HTTP Method: {}", method).into()),
    }
}
fn build_get(uri: &str, buff: &Vec<&str>) -> Res<Request<String>> {
    let req = Request::builder().method("GET").uri(uri);
    parse_parts(&buff, req)
}
fn build_post(uri: &str, buff: &Vec<&str>) -> Res<Request<String>> {
    let req = Request::builder().method("POST").uri(uri);
    parse_parts(&buff, req)
}
fn parse_parts(buff: &Vec<&str>, mut req: http::request::Builder) -> Res<Request<String>> {
    let headers = req.headers_mut().ok_or("Missing Headers")?;
    let mut body = String::new();
    let mut is_body = false;
    for line in buff.iter() {
        if line.is_empty() {
            is_body = true;
        }
        if is_body {
            body.push_str(line);
        } else {
            let header: Vec<&str> = line.splitn(2, ":").collect();
            let header_name = HeaderName::from_bytes(header[0].as_bytes())?;
            headers.insert(header_name, header[1].parse()?);
        }
    }
    Ok(req.body(body)?)
}
