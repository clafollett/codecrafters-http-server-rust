#[allow(unused_imports)]
mod http;
mod threadpool;

use crate::http::{
    HttpHeader,
    HttpRequest,
    HttpResponse
};

use crate::threadpool::ThreadPool;

use std::io::{
    BufRead,
    BufReader,
    Error,
    ErrorKind,
    Read,
    Write
};

use std::net::{
    TcpListener,
    TcpStream
};

use std::time::Duration;

use std::vec;

const CRLF: &str = "\r\n";

const MSG_OK: &str = "OK";
const MSG_BAD_REQUEST: &str = "Bad Request";
const MSG_INTERNAL_SERVER_ERROR: &str = "Internal Server Error";
const MSG_NOT_FOUND: &str = "Not Found";
const MSG_METHOD_NOT_ALLOWED: &str = "Method Not Allowed";

const ROUTE_HOME: &str = "/";
const ROUTE_ECHO: &str = "/echo/";
const ROUTE_USER_AGENT: &str = "/user-agent";

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    let pool = ThreadPool::new(4);
    
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New connection accepted....");
                stream.set_read_timeout(Some(Duration::from_secs(30))).unwrap();

                pool.queue(move || {
                    handle_request(stream).unwrap();
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_request(stream: TcpStream) -> std::io::Result<()> {
    let mut stream = stream;
    let request_result = read_request_from_stream(&mut stream);
    
    if request_result.is_err() {
        let error = request_result.err().unwrap();
        return match error.kind() {
            ErrorKind::InvalidInput => handle_bad_request(&mut stream),
            _ => handle_error(&mut stream, &error)
        }
    };

    let request = request_result.unwrap();

    return match request.method.as_str() {
        "GET" => handle_get(&request, &mut stream),
        // "POST" => {
        //     let response = format!("HTTP/1.1 200 OK\r\n\r\n{}", body);
        //     stream.write_all(response.as_bytes()).unwrap();
        // }
        _ => {
            let response = HttpResponse::new(405, MSG_METHOD_NOT_ALLOWED).to_bytes();
            return stream.write_all(&response);
        }
    };
}

fn read_request_from_stream(stream: &mut TcpStream) -> Result<HttpRequest, Error> {
    let meta = read_meta(stream)?;
    let request_line_parts: Vec<&str> = meta[0].split(" ").collect();

    if request_line_parts.len() != 3 {
        return Err(Error::new(ErrorKind::InvalidInput, "Request line is invalid"));
    }

    let method = request_line_parts[0].to_string();
    let path = request_line_parts[1].to_string();
    let version = request_line_parts[2].to_string();
    let headers: Vec<HttpHeader> = meta[1..].iter().map(|x| {
        let header = match x.split_once(": ") {
            Some((n, v)) => Ok(HttpHeader { name: n.to_string(), value: v.to_string()}),
            None => Err(Error::new(ErrorKind::Other, "Invalid header"))
        };

        // TODO: Need to handle potential errors better here
        return header.unwrap();
    }).collect();

    let mut body = Vec::<u8>::new();
    let content_length_header = headers.iter().find(|h| h.name.to_lowercase() == "content-length");

    if let Some(header) = content_length_header {
        let content_length = header.value.parse::<usize>().unwrap();
        let mut buffer = vec![0; content_length];

        stream.read_exact(&mut buffer)?;
        body = buffer;
    }

    let request = HttpRequest {
        method,
        path,
        version,
        headers,
        body: Some(body)
    };

    println!("
HTTP Request:
    HTTP_METHOD: {}
    HTTP_PATH: {}
    HTTP_VERSION: {}
    HTTP_HEADERS: {:?}
    HTTP_BODY: {:?}
"
        , request.method
        , request.path
        , request.version
        , request.headers
        , request.body.as_ref().unwrap_or(&vec![])
    );

    return Ok(request);
}

fn  read_meta(stream: &mut TcpStream) -> Result<Vec<String>, Error> {
    let mut meta = Vec::<String>::new();
    let mut reader = BufReader::new(stream.try_clone()?);
    
    loop {
        let mut meta_line = String::with_capacity(128);
        
        match reader.read_line(&mut meta_line) {
            Ok(0) => break,
            Ok(_) => {
                // HTTP headers are terminated by a CRLF before the body start
                if meta_line == CRLF {
                    break;
                }

                meta.push(meta_line.trim_end_matches(CRLF).to_string());
            },
            Err(e) => return Err(e)
        };
    }

    Ok(meta)
}

fn handle_get(request: &HttpRequest, stream: &mut TcpStream) -> std::io::Result<()> {
    return match request.path.as_str() {
        ROUTE_HOME => handle_home(stream),
        ROUTE_USER_AGENT => handle_user_agent(request, stream),
        path if path.starts_with(ROUTE_ECHO) => handle_echo(stream, path),
        _ => handle_not_found(stream)
    }
}

fn handle_bad_request(stream: &mut TcpStream) -> std::io::Result<()> {
    let response = HttpResponse::new(400, MSG_BAD_REQUEST).to_bytes();
    return stream.write_all(&response);
}

fn handle_echo(stream: &mut TcpStream, path: &str) -> std::io::Result<()> {
    let echo_message = match path.get(ROUTE_ECHO.len()..) {
        Some(message) => message,
        None => ""
    };

    let headers = [HttpHeader::new("Content-Type", "text/plain")];
    let response = HttpResponse::new_with_body(200, MSG_OK, &headers, echo_message.as_bytes()).to_bytes();

    return stream.write_all(&response);
}

fn handle_error(stream: &mut TcpStream, error: &Error) -> std::io::Result<()> {
    let headers = [HttpHeader::new("Content-Type", "text/plain")];
    let error_message = error.to_string();
    let response = HttpResponse::new_with_body(500, MSG_INTERNAL_SERVER_ERROR, &headers, error_message.as_bytes()).to_bytes();

    return stream.write_all(&response);
}

fn handle_not_found(stream: &mut TcpStream) -> std::io::Result<()> {
    let response = HttpResponse::new(404, MSG_NOT_FOUND).to_bytes();
    return stream.write_all(&response);
}

fn handle_home(stream: &mut TcpStream) -> std::io::Result<()> {
    let response = HttpResponse::new(200, MSG_OK).to_bytes();
    return stream.write_all(&response);
}

fn handle_user_agent(request: &HttpRequest, stream: &mut TcpStream) -> std::io::Result<()> {
    let user_agent = match request.get_header("user-agent") {
        Some(header) => header.value.as_str(),
        None => "Unknown"
    };

    let headers = [HttpHeader::new("Content-Type", "text/plain")];
    let response = HttpResponse::new_with_body(200, MSG_OK, &headers, user_agent.as_bytes()).to_bytes();

    return stream.write_all(&response);
}
