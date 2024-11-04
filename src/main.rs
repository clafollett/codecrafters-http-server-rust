#[allow(unused_imports)]
use std::io::{Error, Read, Write};
use std::net::{TcpListener, TcpStream};

const CRLF: &str = "\r\n";
// const CRLF_CRLF: &str = "\r\n\r\n";

const MSG_OK: &str = "OK";
const MSG_NOT_FOUND: &str = "Not Found";
const MSG_BAD_REQUEST: &str = "Bad Request";

const ROUTE_ECHO: &str = "/echo/";

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    
    for stream in listener.incoming() {
        match stream {
            Ok(_stream) => {
                handle_request(_stream).unwrap();
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn build_response(status_code: u16, status_message: &str, response_headers: Option<Vec<String>>, body: Option<&str>) -> String {
    let response_line = format!("HTTP/1.1 {} {}{CRLF}", status_code, status_message);

    let mut headers = match response_headers {
        Some(value) => value,
        None => vec![]
    };

    let mut header_lines= String::new();

    let body_content = match body {
        Some(value) => value,
        None => ""
    };

    if body_content.len() > 0 {
        let content_length_header = format!("Content-Length: {}", body_content.len());
        headers.push(content_length_header);
    }

    for header in headers {
        header_lines.push_str(&header);
        header_lines.push_str(CRLF);
    }

    if header_lines.len() > 0 {
        header_lines.push_str(CRLF);
    }   
    
    return format!("{response_line}{header_lines}{body_content}");
}

fn handle_request(mut stream: TcpStream) -> Result<(), std::io::Error> {
    let request_raw = match read_request_data(&mut stream) {
        Ok(value) => value,
        Err(value) => return value,
    };

    // Headers and body are separated by \r\n\r\n
    const REQUEST_RAW_DELIMITER: &str = "\r\n\r\n";
    let ( request_envelope, _request_body ) = request_raw.split_once(REQUEST_RAW_DELIMITER).unwrap();
    let request_envelop_items: Vec<&str> = request_envelope.split("\r\n").collect();

    if request_envelop_items.len() > 0 {
        println!("HTTP Request: {:?}", request_envelop_items);

        let request_line_parts: Vec<&str> = request_envelop_items[0].split(" ").collect();
        let http_method = request_line_parts[0];
        let target_path = request_line_parts[1];
        let http_version = request_line_parts[2];

        match http_method {
            "GET" => {
                handle_get(target_path, http_version, stream);
            }
            // "POST" => {
            //     let response = format!("HTTP/1.1 200 OK\r\n\r\n{}", body);
            //     stream.write_all(response.as_bytes()).unwrap();
            // }
            _ => {
                let response = "HTTP/1.1 405 Method Not Allowed\r\n\r\n";
                stream.write_all(response.as_bytes()).unwrap();
            }
            
        }
    } else {
        handle_bad_request(stream);
    }

    return Ok(())
}

fn read_request_data(stream: &mut TcpStream) -> Result<String, Result<(), Error>> {
    const BUFFER_SIZE: usize = 512;

    let mut request_buffer = [0; BUFFER_SIZE];
    let mut request_raw = String::with_capacity(BUFFER_SIZE);
    let mut bytes_read = stream.read(&mut request_buffer).unwrap();
    
    println!("New connection accepted....");
    println!("Read {} bytes", bytes_read);
    
    if bytes_read == 0 {
        return Err(Ok(()));
    }
    
    request_raw.push_str(std::str::from_utf8(&request_buffer).unwrap());
    
    while bytes_read == BUFFER_SIZE {
        request_buffer = [0; BUFFER_SIZE];
        bytes_read = stream.read(&mut request_buffer).unwrap();

        if bytes_read == 0 {
            break;
        }

        request_raw.push_str(std::str::from_utf8(&request_buffer).unwrap());
    }

    Ok(request_raw)
}

fn handle_get(target_path: &str, _http_version: &str, stream: TcpStream) {
    match target_path {
        "/" => {
            handle_ok(stream);
        },
        path if path.starts_with(ROUTE_ECHO) => {
            handle_echo(path, stream);
        },
        _ => {
            handle_not_found(stream);
        }
    }
}

fn handle_echo(path: &str, mut stream: TcpStream) {
    let echo_message = path.get(ROUTE_ECHO.len()..).unwrap();
    let headers = vec!["Content-Type: text/plain".to_string()];
    let response = build_response(200, MSG_OK, Some(headers), Some(&echo_message));
        
    println!("HTTP Response:\r\n\r\n{}", response);
    
    stream.write_all(response.as_bytes()).unwrap();
}

fn handle_bad_request(mut stream: TcpStream) {
    let response = build_response(400, MSG_BAD_REQUEST, None, None);
    stream.write_all(response.as_bytes()).unwrap();
}

fn handle_not_found(mut stream: TcpStream) {
    let response = build_response(404, MSG_NOT_FOUND, None, None);
    stream.write_all(response.as_bytes()).unwrap();
}

fn handle_ok(mut stream: TcpStream) {
    let response = build_response(200, MSG_OK, None, None);
    stream.write_all(response.as_bytes()).unwrap();
}

