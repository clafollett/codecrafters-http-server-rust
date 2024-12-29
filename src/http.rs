#![allow(dead_code)]
use flate2::{write::GzEncoder, Compression};

use std::{fmt, io::Write, net::TcpStream, path::PathBuf};

const CRLF: &str = "\r\n";

// HTTP Content Types
pub const CT_TEXT_PLAIN: &str = "text/plain";
pub const CT_APP_OCTET_STREAM: &str = "application/octet-stream";

// HTTP Encoding Types
pub const ENCODING_GZIP: &str = "gzip";

// HTTP Header Names
pub const HDR_ACCEPT_ENCODING: &str = "Accept-Encoding";
pub const HDR_CONTENT_ENCODING: &str = "Content-Encoding";
pub const HDR_CONTENT_LENGTH: &str = "Content-Length";
pub const HDR_CONTENT_TYPE: &str = "Content-Type";
pub const HDR_USER_AGENT: &str = "User-Agent";

pub const METHOD_GET: &str = "GET";
pub const METHOD_POST: &str = "POST";
// pub const METHOD_PUT: &str = "PUT";
// pub const METHOD_DELETE: &str = "DELETE";

#[derive(Clone, Debug)]
pub struct HttpHeader {
    pub name: String,
    pub value: String,
}

impl HttpHeader {
    pub fn new(name: String, value: String) -> HttpHeader {
        HttpHeader { name, value }
    }
}

impl fmt::Display for HttpHeader {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let str = format!("{}: {}", self.name, self.value);
        fmt.write_str(&str)?;
        return Ok(());
    }
}

#[derive(Clone, Debug)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub version: String,
    pub headers: Vec<HttpHeader>,
    pub body: Option<Vec<u8>>,
}

impl HttpRequest {
    pub fn get_header(&self, name: &str) -> Option<&HttpHeader> {
        self.headers
            .iter()
            .find(|h| h.name.to_lowercase() == name.to_lowercase())
    }

    pub fn supports_encoding(&self, encoding: &str) -> bool {
        if let Some(header) = self.get_header(HDR_ACCEPT_ENCODING) {
            let encoding_index = header
                .value
                .to_lowercase()
                .split(",")
                .map(|s| s.trim())
                .position(|e| e == encoding.to_lowercase());

            return encoding_index.is_some();
        }

        return false;
    }
}

#[derive(Debug)]
pub struct HttpRequestContext {
    pub request: HttpRequest,
    pub stream: TcpStream,
    pub file_directory: PathBuf,
}

impl HttpRequestContext {
    pub fn new(
        request: HttpRequest,
        stream: TcpStream,
        file_directory: PathBuf,
    ) -> HttpRequestContext {
        HttpRequestContext {
            request: request,
            stream: stream,
            file_directory,
        }
    }
}

#[derive(Clone, Debug)]
pub struct HttpResponse<'a> {
    pub status_code: u16,
    pub status_message: String,
    pub headers: Vec<HttpHeader>,
    body: Option<Vec<u8>>,
    context: Option<&'a HttpRequestContext>,
}

impl<'a> HttpResponse<'_> {
    pub fn new(
        context: &'_ HttpRequestContext,
        status_code: u16,
        status_message: String,
        headers: Option<Vec<HttpHeader>>,
        body: Option<Vec<u8>>,
    ) -> HttpResponse {
        let mut response = HttpResponse {
            status_code,
            status_message,
            headers: match headers {
                Some(h) => h,
                None => Vec::<HttpHeader>::new(),
            },
            // Set None now and we will use the set_body method and it's logic later
            body: None,
            context: Some(context),
        };

        if body.is_some() {
            response.set_body(body.unwrap());
        }

        return response;
    }

    pub fn with_no_context(
        status_code: u16,
        status_message: String,
        headers: Option<Vec<HttpHeader>>,
        body: Option<Vec<u8>>,
    ) -> HttpResponse<'a> {
        let mut response = HttpResponse {
            status_code,
            status_message,
            headers: match headers {
                Some(h) => h,
                None => Vec::<HttpHeader>::new(),
            },
            // Set None now and we will use the set_body method and it's logic later
            body: None,
            context: None,
        };

        if body.is_some() {
            response.set_body(body.unwrap());
        }

        return response;
    }

    pub fn get_header(&mut self, name: &str) -> Option<&mut HttpHeader> {
        self.headers
            .iter_mut()
            .find(|h| h.name.to_lowercase() == name.to_lowercase())
    }

    pub fn get_header_index(&mut self, name: &str) -> Option<usize> {
        self.headers
            .iter_mut()
            .position(|h| h.name.to_lowercase() == name.to_lowercase())
    }

    pub fn get_header_value(&mut self, name: &str) -> Option<&str> {
        return match self.get_header(name) {
            Some(header) => Some(header.value.as_str()),
            None => None,
        };
    }

    pub fn get_body(&self) -> Option<&[u8]> {
        return self.body.as_deref();
    }

    pub fn remove_header(&mut self, name: &str) {
        if let Some(index) = self.get_header_index(name) {
            self.headers.remove(index);
        }
    }

    pub fn set_or_add_header_value(&mut self, header_name: &str, value: String) {
        if let Some(header) = self.get_header(header_name) {
            header.value = value;
        } else {
            self.headers
                .push(HttpHeader::new(header_name.into(), value));
        }
    }

    pub fn set_body(&mut self, body: Vec<u8>) {
        // If we doon't have a body, we need to remove the Content-Length header
        if body.len() == 0 {
            self.body = None;

            self.remove_header(HDR_CONTENT_LENGTH);
            self.remove_header(HDR_CONTENT_ENCODING);

            return;
        }

        let mut body = body;

        if let Some(context) = self.context {
            if context.request.supports_encoding(ENCODING_GZIP) {
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());

                // TODO: Handle errors better here
                encoder.write_all(&body).unwrap();
                body = encoder.finish().unwrap();

                self.set_or_add_header_value(HDR_CONTENT_ENCODING, ENCODING_GZIP.into());
            }
        }

        let content_length = body.len().to_string();
        self.set_or_add_header_value(HDR_CONTENT_LENGTH, content_length);

        self.body = Some(body);
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let response_line = format!(
            "HTTP/1.1 {} {}{CRLF}",
            self.status_code, self.status_message
        );
        let mut header_lines = String::with_capacity(self.headers.len() * 128);

        for header in &self.headers {
            header_lines.push_str(&header.to_string());
            header_lines.push_str(CRLF);
        }

        header_lines.push_str(CRLF);

        let mut response_bytes = Vec::<u8>::new();

        let body = match &self.body {
            Some(b) => b,
            None => &Vec::<u8>::new(),
        };

        response_bytes.extend_from_slice(response_line.as_bytes());
        response_bytes.extend_from_slice(header_lines.as_bytes());
        response_bytes.extend_from_slice(body.as_slice());

        return response_bytes;
    }
}
