use std::collections::VecDeque;
use http_parser::{CallbackResult, HttpParser, HttpParserCallback, ParseAction};
use token::HttpToken;

pub enum ParserType {
    Request,
    Response,
}

enum State {
    Default,
    Url(String),
    Field(String),
    Value(String, String),
}

fn to_string(bytes: &[u8]) -> String {
    String::from_utf8(bytes.iter().cloned().collect::<Vec<_>>()).unwrap()
}

pub struct ParserHandler {
    pub tokens: VecDeque<HttpToken>,
    pending_tokens: VecDeque<HttpToken>,
    reason_phrase: Option<String>,
    state: State,
    pub parser_type: ParserType,
}

impl ParserHandler {
    pub fn new(parser_type: ParserType) -> ParserHandler {
        ParserHandler {
            tokens: VecDeque::new(),
            pending_tokens: VecDeque::new(),
            reason_phrase: None,
            state: State::Default,
            parser_type: parser_type,
        }
    }
}

impl HttpParserCallback for ParserHandler {
    fn on_message_begin(&mut self, _parser: &mut HttpParser) -> CallbackResult {
        return Ok(ParseAction::None);
    }

    fn on_url(&mut self, _parser: &mut HttpParser, url: &[u8])
              -> CallbackResult {
        let url = to_string(url);
        match self.state {
            State::Default => self.state = State::Url(url),
            State::Url(ref mut buf) => buf.push_str(&url),
            _ => unreachable!(),
        }
        return Ok(ParseAction::None);
    }

    fn on_status(&mut self, _parser: &mut HttpParser, status: &[u8])
                 -> CallbackResult {
        let status = to_string(status);
        match self.reason_phrase {
            Some(ref mut buf) => buf.push_str(&status),
            None => self.reason_phrase = Some(status),
        }
        return Ok(ParseAction::None);
    }

    fn on_header_field(&mut self, _parser: &mut HttpParser,
                       field: &[u8]) -> CallbackResult {
        let field = to_string(field);
        let mut new_state = None;
        match self.state {
            State::Default => {
                new_state = Some(State::Field(field));
            }
            State::Url(ref mut url) => {
                let url = {
                    let mut t = String::new();
                    use std::mem;
                    mem::swap(&mut t, url);
                    t
                };
                self.pending_tokens.push_back(HttpToken::Url(url));
                new_state = Some(State::Field(field));
            }
            State::Field(ref mut buf) => buf.push_str(&field),
            State::Value(ref mut f, ref mut v) => {
                let f = {
                    let mut t = String::new();
                    use std::mem;
                    mem::swap(&mut t, f);
                    t
                };
                let v = {
                    let mut t = String::new();
                    use std::mem;
                    mem::swap(&mut t, v);
                    t
                };
                self.pending_tokens.push_back(HttpToken::Field(f, v));
                new_state = Some(State::Field(field));
            }
        }
        if let Some(state) = new_state.take() {
            self.state = state;
        }
        return Ok(ParseAction::None);
    }

    fn on_header_value(&mut self, _parser: &mut HttpParser,
                       value: &[u8]) -> CallbackResult {
        let value = to_string(value);
        let mut new_state = None;
        match self.state {
            State::Field(ref mut f) => {
                let f = {
                    let mut t = String::new();
                    use std::mem;
                    mem::swap(&mut t, f);
                    t
                };
                new_state = Some(State::Value(f, value));
            }
            State::Value(_, ref mut buf) => buf.push_str(&value),
            _ => unreachable!(),
        }
        if let Some(state) = new_state.take() {
            self.state = state;
        }
        return Ok(ParseAction::None);
    }

    fn on_headers_complete(&mut self, parser: &mut HttpParser)
                           -> CallbackResult {
        let mut new_state = None;
        if let State::Value(ref mut f, ref mut v) = self.state {
                let f = {
                    let mut t = String::new();
                    use std::mem;
                    mem::swap(&mut t, f);
                    t
                };
                let v = {
                    let mut t = String::new();
                    use std::mem;
                    mem::swap(&mut t, v);
                    t
                };
            self.pending_tokens.push_back(HttpToken::Field(f, v));
            new_state = Some(State::Default);
        }
        if let Some(state) = new_state.take() {
            self.state = state;
        }
        let t = match self.parser_type {
            ParserType::Request => {
                HttpToken::Method(parser.method.unwrap().to_string())
            }
            ParserType::Response => {
                HttpToken::Status(parser.status_code.unwrap(),
                                  self.reason_phrase.take().unwrap())
            }
        };
        self.tokens.push_back(t);
        self.tokens.append(&mut self.pending_tokens);
        return Ok(ParseAction::None);
    }

    fn on_body(&mut self, _parser: &mut HttpParser, body: &[u8])
               -> CallbackResult {
        self.tokens.push_back(HttpToken::Body(body.iter().cloned().collect()));
        return Ok(ParseAction::None);
    }

    fn on_message_complete(&mut self, _parser: &mut HttpParser)
                           -> CallbackResult {
        self.state = State::Default;
        self.tokens.push_back(HttpToken::EndOfMessage);
        return Ok(ParseAction::None);
    }
}
