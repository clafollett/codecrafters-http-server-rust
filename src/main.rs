//#[allow(unused_imports)]
mod http;
mod threadpool;

use crate::http::{
    HttpHeader,
    HttpRequest,
    HttpRequestContext,
    HttpResponse
};

use crate::threadpool::ThreadPool;

use clap::Parser;

use std::{
    self,
    env,
    fs,
    io::{
        BufRead,
        BufReader,
        Error,
        ErrorKind,
        Read,
        Write
    },
    net::{
        TcpListener,
        TcpStream
    },
    path::{
        Path,
        PathBuf
    },
    time::Duration,
    vec
};

const CRLF: &str = "\r\n";

const MSG_OK: &str = "OK";
const MSG_CREATED: &str = "Created";
const MSG_BAD_REQUEST: &str = "Bad Request";
const MSG_INTERNAL_SERVER_ERROR: &str = "Internal Server Error";
const MSG_NOT_FOUND: &str = "Not Found";
const MSG_METHOD_NOT_ALLOWED: &str = "Method Not Allowed";

const ROUTE_ECHO: &str = "/echo/";
const ROUTE_FILES: &str = "/files/";
const ROUTE_HOME: &str = "/";
const ROUTE_USER_AGENT: &str = "/user-agent";

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the root file server directory
    #[arg(short, long)]
    directory: Option<String>
}

fn main() {
    let args = Args::parse();
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    let pool = ThreadPool::new(4);
    let current_dir = env::current_dir().unwrap();
    let file_directory = match args.directory {
        Some(v) => Path::new(&v).to_path_buf(),
        None => current_dir.join("file_directory")
    };

    if file_directory.try_exists().is_err() {
        fs::create_dir(&file_directory).unwrap();
    }

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New connection accepted....");
                let file_directory = file_directory.clone();

                // TODO: Make timeout configurable
                stream.set_read_timeout(Some(Duration::from_secs(30))).unwrap();

                pool.queue(move || {
                    handle_request(&stream, &file_directory).unwrap();
                });
            }
            Err(e) => {
                // TODO: Need to handle potential errors better here
                println!("error: {}", e);
            }
        }
    }
}

fn handle_request(stream: &TcpStream, file_directory: &PathBuf) -> std::io::Result<()> {
    let mut stream = stream.try_clone().unwrap();
    let request_result = read_request_from_stream(&mut stream);
    
    if request_result.is_err() {
        let error = request_result.err().unwrap();
        return match error.kind() {
            ErrorKind::InvalidInput => handle_bad_request(&mut stream),
            _ => handle_error(&mut stream, &error)
        }
    };

    let mut context = HttpRequestContext::new(
        &stream,
        file_directory,
        &request_result.unwrap()
    );

    return match context.request.method.as_str() {
        http::METHOD_GET => handle_get(&mut context),
        http::METHOD_POST => handle_post(&mut context),
        _ => {
            let response = HttpResponse::new(405, MSG_METHOD_NOT_ALLOWED, None, None).to_bytes();
            return stream.write_all(&response);
        }
    };
}

fn read_request_from_stream(stream: &mut TcpStream) -> Result<HttpRequest, Error> {
    let mut reader = BufReader::new(stream);
    let meta = read_meta(&mut reader)?;

    assert!(meta.len() > 0);

    let request_line_parts: Vec<&str> = meta[0].split(" ").collect();

    assert!(request_line_parts.len() == 3);

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

    let mut body: Option<Vec::<u8>> = None;
    let content_length_header = headers.iter().find(
        |h| h.name.to_lowercase() == http::HDR_CONTENT_LENGTH.to_lowercase()
    );

    if let Some(header) = content_length_header {
        let content_length = header.value.parse::<usize>().unwrap();
        let mut buffer = vec![0; content_length];
        
        reader.read_exact(&mut buffer)?;
        body = Some(buffer);
    }

    let request = HttpRequest {
        method,
        path,
        version,
        headers,
        body
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

fn  read_meta(reader: &mut BufReader<&mut TcpStream>) -> Result<Vec<String>, Error> {
    let mut meta = Vec::<String>::new();
    
    loop {
        let mut meta_line = String::with_capacity(128);
        
        match reader.read_line(&mut meta_line) {
            Ok(0) => break,
            Ok(_) => {
                // HTTP headers are terminated by a CRLF before the body start
                if meta_line == CRLF {
                    break;
                }

                // Make sure to trim the trailing CRLF from the header
                meta.push(meta_line.trim_end_matches(CRLF).to_string());
            },
            Err(e) => return Err(e)
        };
    }

    Ok(meta)
}

fn handle_get(context: &mut HttpRequestContext) -> std::io::Result<()> {
    return match context.request.path.as_str() {
        ROUTE_HOME => handle_home(context),
        ROUTE_USER_AGENT => handle_user_agent(context),
        path if path.starts_with(ROUTE_ECHO) => handle_echo(context),
        path if path.starts_with(ROUTE_FILES) => handle_get_file(context),
        _ => handle_not_found(context)
    }
}

fn handle_post(context: &mut HttpRequestContext) -> std::io::Result<()> {
    return match context.request.path.as_str() {
        path if path.starts_with(ROUTE_FILES) => handle_post_file(context),
        _ => handle_not_found(context)
    }
}

fn handle_bad_request(stream: &mut TcpStream) -> std::io::Result<()> {
    let response = HttpResponse::new(400, MSG_BAD_REQUEST, None, None).to_bytes();
    return stream.write_all(&response);
}

fn handle_echo(context: &mut HttpRequestContext) -> std::io::Result<()> {
    let echo_message = match context.request.path.get(ROUTE_ECHO.len()..) {
        Some(message) => message,
        None => ""
    };

    let headers = [HttpHeader::new("Content-Type", "text/plain")];
    let response = HttpResponse::new(200, MSG_OK, Some(&headers), Some(echo_message.as_bytes())).to_bytes();

    return context.stream.write_all(&response);
}

fn handle_error(stream: &mut TcpStream, error: &Error) -> std::io::Result<()> {
    let headers = [HttpHeader::new(&http::HDR_CONTENT_TYPE, http::CT_TEXT_PLAIN)];
    let error_message = error.to_string();
    let response = HttpResponse::new(500, MSG_INTERNAL_SERVER_ERROR, Some(&headers), Some(error_message.as_bytes())).to_bytes();

    return stream.write_all(&response);
}

fn handle_get_file(context: &mut HttpRequestContext) -> std::io::Result<()> {
    let file_name = match context.request.path.get(ROUTE_FILES.len()..) {
        Some(f) => f,
        None => ""
    };

    let file_path = context.file_directory.join(file_name);

    if file_path.exists() == false {
        return handle_not_found(context);
    }

    let file_bytes = fs::read(&file_path).unwrap();
    let headers = [HttpHeader::new(&http::HDR_CONTENT_TYPE, http::CT_APP_OCTET_STREAM)];
    let response = HttpResponse::new(200, MSG_OK, Some(&headers), Some(&file_bytes)).to_bytes();

    return context.stream.write_all(&response);
}

fn handle_post_file(context: &mut HttpRequestContext) -> std::io::Result<()> {
    let file_name = match context.request.path.get(ROUTE_FILES.len()..) {
        Some(f) => f,
        None => ""
    };

    let file_path = context.file_directory.join(file_name);

    if file_path.exists() {
        // TODO: Handle the remove_file error better here
        fs::remove_file(&file_path).unwrap();
    }

    if context.request.body.is_some() {
        // TODO: Handle potential errors better here
        let body = context.request.body.as_ref().unwrap();
        fs::write(file_path, body).unwrap();
    }

    let response = HttpResponse::new(201, MSG_CREATED, None, None).to_bytes();

    return context.stream.write_all(&response);
}

fn handle_not_found(context: &mut HttpRequestContext) -> std::io::Result<()> {
    let response = HttpResponse::new(404, MSG_NOT_FOUND, None, None).to_bytes();
    return context.stream.write_all(&response);
}

fn handle_home(context: &mut HttpRequestContext) -> std::io::Result<()> {
    let response = HttpResponse::new(200, MSG_OK, None, None).to_bytes();
    return context.stream.write_all(&response);
}

fn handle_user_agent(context: &mut HttpRequestContext) -> std::io::Result<()> {
    let user_agent = match context.request.get_header(&http::HDR_USER_AGENT) {
        Some(header) => header.value.as_str(),
        None => "Unknown"
    };

    let headers = [HttpHeader::new(&http::HDR_CONTENT_TYPE, &http::CT_TEXT_PLAIN)];
    let response = HttpResponse::new(200, MSG_OK, Some(&headers), Some(user_agent.as_bytes())).to_bytes();

    return context.stream.write_all(&response);
}
