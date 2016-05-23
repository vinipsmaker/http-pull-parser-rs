use std::collections::VecDeque;
use http_muncher;
use token::HttpToken;

enum State {
    Default,
    Url(String),
    Status(String),
    Field(String),
    Value(String, String),
}

fn to_string(bytes: &[u8]) -> String {
    String::from_utf8(bytes.iter().cloned().collect::<Vec<_>>()).unwrap()
}

pub struct ParserHandler {
    pub tokens: VecDeque<HttpToken>,
    state: State,
    pub message_complete: bool,
}

impl Default for ParserHandler {
    fn default() -> ParserHandler {
        ParserHandler {
            tokens: VecDeque::new(),
            state: State::Default,
            message_complete: false,
        }
    }
}

impl http_muncher::ParserHandler for ParserHandler {
    fn on_message_begin(&mut self) -> bool {
        return true;
    }

    fn on_url(&mut self, url: &[u8]) -> bool {
        let url = to_string(url);
        match self.state {
            State::Default => self.state = State::Url(url),
            State::Url(ref mut buf) => buf.push_str(&url),
            _ => unreachable!(),
        }
        return true;
    }

    fn on_status(&mut self, status: &[u8]) -> bool {
        let status = to_string(status);
        let mut new_state = None;
        match self.state {
            State::Default => new_state = Some(State::Status(status)),
            State::Status(ref mut buf) => buf.push_str(&status),
            _ => unreachable!(),
        }
        if let Some(state) = new_state.take() {
            self.state = state;
        }
        return true;
    }

    fn on_header_field(&mut self, field: &[u8]) -> bool {
        let field = to_string(field);
        let mut new_state = None;
        match self.state {
            State::Url(ref mut url) => {
                let url = {
                    let mut t = String::new();
                    use std::mem;
                    mem::swap(&mut t, url);
                    t
                };
                self.tokens.push_back(HttpToken::Url(url));
                new_state = Some(State::Field(field));
            }
            State::Status(ref mut status) => {
                let status = {
                    let mut t = String::new();
                    use std::mem;
                    mem::swap(&mut t, status);
                    t
                };
                self.tokens.push_back(HttpToken::Status(0, status));
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
                self.tokens.push_back(HttpToken::Field(f, v));
                new_state = Some(State::Field(field));
            }
            _ => unreachable!(),
        }
        if let Some(state) = new_state.take() {
            self.state = state;
        }
        return true;
    }

    fn on_header_value(&mut self, value: &[u8]) -> bool {
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
        return true;
    }

    fn on_headers_complete(&mut self) -> bool {
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
            self.tokens.push_back(HttpToken::Field(f, v));
            new_state = Some(State::Default);
        }
        if let Some(state) = new_state.take() {
            self.state = state;
        }
        return true;
    }

    fn on_body(&mut self, body: &[u8]) -> bool {
        self.tokens.push_back(HttpToken::Body(body.iter().cloned().collect()));
        return true;
    }

    fn on_message_complete(&mut self) -> bool {
        self.state = State::Default;
        self.tokens.push_back(HttpToken::EndOfMessage);
        return true;
    }
}
