use http_muncher;
use token::HttpToken;
use parser_handler::ParserHandler;

#[derive(Clone)]
pub struct ParserError {
    pub error: String,
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
                nparsed = self.parser.parse(data);
                if self.parser.has_error() {
                    self.error = Some(ParserError {
                        error: self.parser.error().to_string(),
                    });
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
    #[test]
    fn testbed() {
        use http_muncher::*;

        struct MyHandler;
        impl ParserHandler for MyHandler {
            fn on_header_field(&mut self, header: &[u8]) -> bool {
                println!("{:?}: ", String::from_utf8(header.iter().cloned().collect::<Vec<_>>()).unwrap());
                true
            }
            fn on_header_value(&mut self, value: &[u8]) -> bool {
                println!("\t {:?}", String::from_utf8(value.iter().cloned().collect::<Vec<_>>()).unwrap());
                true
            }
        }

        let http_request = b"GET / HTTP/1.0\r\n\
                             Cont";

        let handler = MyHandler;
        let mut parser = Parser::request(handler);
        parser.parse(http_request);
        let http_request = b"ent-Length: 00";
        parser.parse(http_request);
        let http_request = b"00\r\n\r\n";
        parser.parse(http_request);
    }

    #[test]
    fn testbed2() {
        use super::*;

        let mut parser = Parser::request();
        let mut http_request: Vec<_> = b"GET / HTTP/1.0\r\n\
                                         Content-Length: 0000\r\n\r\n".iter()
            .cloned().collect();

        while let (Ok(Some(t)), nparsed) = parser.next_token(Some(&http_request)) {
            http_request.drain(..nparsed);
            println!("{:?}", t);
        }
    }
}
