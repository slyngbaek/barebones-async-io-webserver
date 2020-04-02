use crate::server::Res;
pub use http::status::StatusCode;
use http::{header::HeaderName, Request, Response};

static CRLF: &'static str = "\r\n";

pub fn serialize_response(res: Response<String>) -> String {
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

pub fn deserialize_request(buff: &str) -> Res<Request<String>> {
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
