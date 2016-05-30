use std::fmt;
use std;
use http_muncher;
use token::HttpToken;
use parser_handler::ParserHandler;

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

enum ParserType {
    Request,
    Response,
}

pub struct Parser {
    parser: http_muncher::Parser<ParserHandler>,
    parser_type: ParserType,
    error: Option<ParserError>,
    first_line_sent: bool,
}

impl Parser {
    pub fn request() -> Parser {
        Parser {
            parser: http_muncher::Parser::request(ParserHandler::default()),
            parser_type: ParserType::Request,
            error: None,
            first_line_sent: false,
        }
    }

    pub fn response() -> Parser {
        Parser {
            parser: http_muncher::Parser::response(ParserHandler::default()),
            parser_type: ParserType::Response,
            error: None,
            first_line_sent: false,
        }
    }

    pub fn next_token(&mut self, data: Option<&[u8]>)
                      -> (Result<Option<HttpToken>, ParserError>, usize) {
        let mut nparsed = 0;
        if self.error.is_none() {
            if let Some(data) = data {
                if data.len() > 0 {
                    nparsed = self.parser.parse(data);
                    if self.parser.has_error() {
                        self.error = Some(ParserError {
                            error: self.parser.error().to_string(),
                        });
                    }
                }
            }
        }

        if self.parser.get().tokens.front().is_some() && !self.first_line_sent {
            self.first_line_sent = true;
            match self.parser_type {
                ParserType::Request => {
                    return (Ok(Some(HttpToken::Method(self.parser.http_method()
                                                      .to_string()))),
                            nparsed);
                }
                ParserType::Response => {
                    let token = match self.parser.get().tokens.pop_front()
                        .unwrap() {
                            HttpToken::Status(_, reason) => {
                                HttpToken::Status(self.parser.status_code(),
                                                  reason)
                            }
                            _ => unreachable!(),
                    };
                    return (Ok(Some(token)), nparsed);
                }
            }
        }

        if let Some(token) = self.parser.get().tokens.pop_front() {
            if token == HttpToken::EndOfMessage {
                self.first_line_sent = false;
            }
            return (Ok(Some(token)), nparsed);
        } else {
            if let Some(ref e) = self.error {
                return (Err(e.clone()), nparsed);
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
