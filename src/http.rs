#![allow(dead_code)]
use std::{fmt, net::TcpStream, path::PathBuf};

const CRLF: &str = "\r\n";

// HTTP Header Names
pub const HDR_CONTENT_LENGTH: &'static str = "Content-Length";
pub const HDR_CONTENT_TYPE: &'static str = "Content-Type";
pub const HDR_USER_AGENT: &'static str = "User-Agent";

// HTTP Content Types
pub const CT_TEXT_PLAIN: &'static str = "text/plain";
pub const CT_APP_OCTET_STREAM: &'static str = "application/octet-stream";

pub const METHOD_GET: &'static str = "GET";
pub const METHOD_POST: &'static str = "POST";
// pub const METHOD_PUT: &'static str = "PUT";
// pub const METHOD_DELETE: &'static str = "DELETE";

#[derive(Clone)]
#[derive(Debug)]
pub struct HttpHeader {
    pub name: String,
    pub value: String
}

impl HttpHeader {
    pub fn new(name: &str, value: &str) -> HttpHeader {
        HttpHeader {
            name: name.to_string(),
            value: value.to_string()
        }
    }
}

impl fmt::Display for HttpHeader {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let str = format!("{}: {}", self.name, self.value);
        fmt.write_str(str.as_str())?;
        return Ok(());
    }
}

#[derive(Clone)]
#[derive(Debug)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub version: String,
    pub headers: Vec<HttpHeader>,
    pub body: Option<Vec<u8>>
}

impl HttpRequest {
    pub fn get_header(&mut self, name: &str) -> Option<&mut HttpHeader> {
        self.headers.iter_mut().find(|h| h.name.to_lowercase() == name.to_lowercase())
    }
}

#[derive(Debug)]
pub struct HttpRequestContext {
    pub file_directory: PathBuf,
    pub request: HttpRequest,
    pub stream: TcpStream
}

impl HttpRequestContext {
    pub fn new(stream: &TcpStream, file_directory: &PathBuf, request: &HttpRequest) -> HttpRequestContext {
        HttpRequestContext {
            file_directory: file_directory.clone(),
            request: request.clone(),
            stream: stream.try_clone().unwrap()
        }
    }
}

#[derive(Clone)]
#[derive(Debug)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_message: String,
    pub headers: Vec<HttpHeader>,
    body: Option<Vec<u8>>
}

impl HttpResponse {
    pub fn new(status_code: u16, status_message: &str, headers: Option<&[HttpHeader]>, body: Option<&[u8]>) -> HttpResponse {
        let mut response = HttpResponse {
            status_code,
            status_message: status_message.to_string(),
            headers: match headers {
                Some(h) => h.to_vec(),
                None => Vec::<HttpHeader>::new()
            },
            // Set None now and we will use the set_body method and it's logic later
            body: None
        };

        if body.is_some() {
            response.set_body(body.unwrap());
        }

        return response;
    }

    pub fn get_header(&mut self, name: &str) -> Option<&mut HttpHeader> {
        self.headers.iter_mut().find(|h| h.name.to_lowercase() == name.to_lowercase())
    }

    pub fn get_header_index(&mut self, name: &str) -> Option<usize> {
        self.headers.iter_mut().position(|h| h.name.to_lowercase() == name.to_lowercase())
    }

    pub fn get_body(&self) -> Option<&[u8]> {
        return self.body.as_deref();
    }

    pub fn set_body(&mut self, body: &[u8]) {

        // If we doon't have a body, we need to remove the Content-Length header
        if body.len() == 0 {
            self.body = None;

            if let Some(index) = self.get_header_index("Content-Length") {
                self.headers.remove(index);
            }

            return;
        }
        
        self.body = Some(body.to_vec());
        let content_length = body.len().to_string();

        if let Some(header) = self.get_header("Content-Length") {
            header.value = content_length;
        } else {
            self.headers.push(HttpHeader::new("Content-Length", &content_length));
        }
    }
    
    pub fn to_bytes(&self) -> Vec::<u8> {
        let response_line = format!("HTTP/1.1 {} {}{CRLF}", self.status_code, self.status_message);
        let mut header_lines = String::with_capacity(self.headers.len() * 128);
    
        for header in &self.headers {
            header_lines.push_str(&header.to_string());
            header_lines.push_str(CRLF);
        }
    
        header_lines.push_str(CRLF);

        let mut response_bytes = Vec::<u8>::new();
        
        let body = match &self.body {
            Some(b) => b,
            None => &Vec::<u8>::new()
        };

        response_bytes.extend_from_slice(response_line.as_bytes());
        response_bytes.extend_from_slice(header_lines.as_bytes());
        response_bytes.extend_from_slice(body.as_slice());
        
        return response_bytes.to_vec();
    }
}
