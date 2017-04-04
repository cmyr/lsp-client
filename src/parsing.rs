//MIT License

//Copyright (c) 2017 Colin Rothfels

//Permission is hereby granted, free of charge, to any person obtaining a copy
//of this software and associated documentation files (the "Software"), to deal
//in the Software without restriction, including without limitation the rights
//to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//copies of the Software, and to permit persons to whom the Software is
//furnished to do so, subject to the following conditions:

//The above copyright notice and this permission notice shall be included in all
//copies or substantial portions of the Software.

//THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//SOFTWARE.

//! Handles parsing of Language Server Protocol messages from a stream.

use std;
use std::io::{self, BufRead};

use serde_json;
use serde_json::value::Value;

macro_rules! print_err {
    ($($arg:tt)*) => (
        {
            use std::io::prelude::*;
            if let Err(e) = write!(&mut ::std::io::stderr(), "{}\n", format_args!($($arg)*)) {
                panic!("Failed to write to stderr.\
                    \nOriginal error output: {}\
                    \nSecondary error writing to stderr: {}", format!($($arg)*), e);
            }
        }
    )
}

#[derive(Debug)]
/// An Error type encapsulating the various failure possibilites of the parsing process.
pub enum ParseError {
    Io(io::Error),
    ParseInt(std::num::ParseIntError),
    Utf8(std::string::FromUtf8Error),
    Json(serde_json::Error),
    Unknown(String),
}

impl From<io::Error> for ParseError {
    fn from(err: io::Error) -> ParseError {
        ParseError::Io(err)
    }
}

impl From<std::string::FromUtf8Error> for ParseError {
    fn from(err: std::string::FromUtf8Error) -> ParseError {
        ParseError::Utf8(err)
    }
}

impl From<serde_json::Error> for ParseError {
    fn from(err: serde_json::Error) -> ParseError {
        ParseError::Json(err)
    }
}

impl From<std::num::ParseIntError> for ParseError {
    fn from(err: std::num::ParseIntError) -> ParseError {
        ParseError::ParseInt(err)
    }
}

impl From<String> for ParseError {
    fn from(s: String) -> ParseError {
        ParseError::Unknown(s)
    }
}

#[derive(Debug, PartialEq)]
/// A message header, as described in the Language Server Protocol specification.
enum LspHeader {
    ContentType,
    ContentLength(usize),
}

/// Given a reference to a reader, attempts to read a Language Server Protocol message,
/// blocking until a message is received.
pub fn read_message<B: BufRead>(reader: &mut B) -> Result<Value, ParseError> {
    let mut buffer = String::new();
    let mut content_length: Option<usize> = None;

    // read in headers. 
    loop {
            buffer.clear();
            reader.read_line(&mut buffer)?;
            match &buffer {
                s if s.trim().len() == 0 => { break }, // empty line is end of headers
                s => {
                    match parse_header(s)? {
                        LspHeader::ContentLength(len) => content_length = Some(len),
                        LspHeader::ContentType => (), // utf-8 only currently allowed value
                    };
                }
            };
        }
    
    let content_length = content_length.ok_or(format!("missing content-length header: {}", buffer))?;
    // message body isn't newline terminated, so we read content_length bytes
    let mut body_buffer = vec![0; content_length];
    reader.read_exact(&mut body_buffer)?;
    let body = String::from_utf8(body_buffer)?;
    Ok(serde_json::from_str::<Value>(&body)?)
}

const HEADER_CONTENT_LENGTH: &'static str = "content-length";
const HEADER_CONTENT_TYPE: &'static str = "content-type";

/// Given a header string, attempts to extract and validate the name and value parts.
fn parse_header(s: &str) -> Result<LspHeader, ParseError> {
    let split: Vec<String> = s.split(": ").map(|s| s.trim().to_lowercase()).collect();
    if split.len() != 2 { return Err(ParseError::Unknown(format!("malformed header: {}", s))) }
    match split[0].as_ref() {
        HEADER_CONTENT_TYPE => Ok(LspHeader::ContentType),
        HEADER_CONTENT_LENGTH => Ok(LspHeader::ContentLength(usize::from_str_radix(&split[1], 10)?)),
        _ => Err(ParseError::Unknown(format!("Unknown header: {}", s))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;
    
    #[test]
    fn test_parse_header() {
        let header = "Content-Length: 132";
        assert_eq!(parse_header(header).ok(), Some((LspHeader::ContentLength(132))));
    }

    #[test]
    fn test_parse_message() {
        let inps = vec!("Content-Length: 18\n\r\n\r{\"name\": \"value\"}", 
                        "Content-length: 18\n\r\n\r{\"name\": \"value\"}", 
                        "Content-Length: 18\n\rContent-Type: utf-8\n\r\n\r{\"name\": \"value\"}");
        for inp in inps {
            let mut reader = BufReader::new(inp.as_bytes());
            let result = match read_message(&mut reader) {
                Ok(r) => r,
                Err(e) => panic!("error: {:?}", e),
            };
            let exp = json!({"name": "value"});
            assert_eq!(result, exp);
        }
    }
}
