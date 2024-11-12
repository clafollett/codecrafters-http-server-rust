use std::fmt;

const CRLF: &str = "\r\n";

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

#[derive(Debug)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub version: String,
    pub headers: Vec<HttpHeader>,
    pub body: Option<Vec<u8>>
}

impl HttpRequest {
    pub fn get_header(&self, name: &str) -> Option<&HttpHeader> {
        self.headers.iter().find(|h| h.name.to_lowercase() == name.to_lowercase())
    }
}

#[derive(Debug)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_message: String,
    pub headers: Vec<HttpHeader>,
    pub body: Option<Vec<u8>>
}

impl HttpResponse {
    // fn get_header(&self, name: &str) -> Option<&HttpHeader> {
    //     self.headers.iter().find(|h| h.name.to_lowercase() == name.to_lowercase())
    // }

    pub fn new(status_code: u16, status_message: &str) -> HttpResponse {
        HttpResponse {
            status_code,
            status_message: status_message.to_string(),
            headers: Vec::new(),
            body: None
        }
    }

    pub fn new_with_body(status_code: u16, status_message: &str, headers: &[HttpHeader], body: &[u8]) -> HttpResponse {
        HttpResponse {
            status_code,
            status_message: status_message.to_string(),
            headers: headers.to_vec(),
            body: Some(body.to_vec())
        }
    }
    
    pub fn to_bytes(&mut self) -> Vec::<u8> {
        let response_line = format!("HTTP/1.1 {} {}{CRLF}", self.status_code, self.status_message);
    
        if let Some(body) = &self.body {
            self.headers.push(HttpHeader::new(
                "Content-Length",
                body.len().to_string().as_str()
            ));
        }
    
        let mut header_lines = String::with_capacity(self.headers.len() * 128);
    
        for header in &self.headers {
            header_lines.push_str(header.to_string().as_str());
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

    // fn get_header(&self, name: &str) -> Option<&HttpHeader> {
    //     self.headers.iter().find(|h| h.name.to_lowercase() == name.to_lowercase())
    // }   
}
