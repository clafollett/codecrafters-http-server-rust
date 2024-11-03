#[allow(unused_imports)]
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

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

fn format_response(status_code: u16, status_message: &str) -> String {
    format!("HTTP/1.1 {} {}\r\n\r\n", status_code, status_message)
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

        let request_line_parts: Vec<&str> = request_envelop_items[0].split(' ').collect();
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

fn read_request_data(stream: &mut TcpStream) -> Result<String, Result<(), std::io::Error>> {
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
        }
        _ => {
            handle_not_found(stream);
        }
    }
}

fn handle_bad_request(mut stream: TcpStream) {
    let response = format_response(400, "Bad Request");
    stream.write_all(response.as_bytes()).unwrap();
}

fn handle_not_found(mut stream: TcpStream) {
    let response = format_response(404, "Not Found");
    stream.write_all(response.as_bytes()).unwrap();
}

fn handle_ok(mut stream: TcpStream) {
    let response = format_response(200, "OK");
    stream.write_all(response.as_bytes()).unwrap();
}

