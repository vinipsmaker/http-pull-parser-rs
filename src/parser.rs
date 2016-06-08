use std::fmt;
use std;
use http_parser::{HttpParser, HttpParserType};
use token::HttpToken;
use parser_handler::{ParserHandler, ParserType};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParserError {
    pub error: String,
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl std::error::Error for ParserError {
    fn description(&self) -> &str {
        &self.error[..]
    }
}

pub struct Parser {
    parser: HttpParser,
    handler: ParserHandler,
}

impl Parser {
    pub fn request() -> Parser {
        Parser {
            parser: HttpParser::new(HttpParserType::Request),
            handler: ParserHandler::new(ParserType::Request),
        }
    }

    pub fn response() -> Parser {
        Parser {
            parser: HttpParser::new(HttpParserType::Response),
            handler: ParserHandler::new(ParserType::Response),
        }
    }

    pub fn next_token(&mut self, data: Option<&[u8]>)
                      -> (Result<Option<HttpToken>, ParserError>, usize) {
        let mut nparsed = 0;
        if self.parser.errno.is_none() {
            if let Some(data) = data {
                if data.len() > 0 {
                    nparsed = self.parser.execute(&mut self.handler, data);
                }
            }
        }

        if let Some(token) = self.handler.tokens.pop_front() {
            return (Ok(Some(token)), nparsed);
        } else {
            if let Some(ref e) = self.parser.errno {
                return (Err(ParserError { error: format!("{}", e) }), nparsed);
            } else {
                return (Ok(None), nparsed);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use token::HttpToken;

    #[test]
    fn simple() {
        let mut parser = Parser::request();
        let mut tokens = Vec::new();
        let mut http_request: Vec<_> = b"GET /te".iter()
            .cloned().collect();

        loop {
            let (t, nparsed) = parser.next_token(Some(&http_request));
            http_request.drain(..nparsed);
            if let Ok(Some(t)) = t {
                tokens.push(t);
            } else {
                break;
            }
        }

        http_request.extend(b"st HTTP/1.0\r\n\
                              Cont".iter().cloned());

        loop {
            let (t, nparsed) = parser.next_token(Some(&http_request));
            http_request.drain(..nparsed);
            if let Ok(Some(t)) = t {
                tokens.push(t);
            } else {
                break;
            }
        }

        http_request.extend(b"ent-Length: 00".iter().cloned());

        loop {
            let (t, nparsed) = parser.next_token(Some(&http_request));
            http_request.drain(..nparsed);
            if let Ok(Some(t)) = t {
                tokens.push(t);
            } else {
                break;
            }
        }

        http_request.extend(b"00\r\n\r\n".iter().cloned());

        loop {
            let (t, nparsed) = parser.next_token(Some(&http_request));
            http_request.drain(..nparsed);
            if let Ok(Some(t)) = t {
                tokens.push(t);
            } else {
                break;
            }
        }

        assert_eq!(tokens,
                   [HttpToken::Method("GET".to_string()),
                    HttpToken::Url("/test".to_string()),
                    HttpToken::Field("Content-Length".to_string(),
                                     "0000".to_string()),
                    HttpToken::EndOfMessage]);
        assert_eq!(parser.next_token(None), (Ok(None), 0));
    }
}
