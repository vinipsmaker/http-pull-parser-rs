#[derive(Debug, PartialEq, Eq)]
pub enum HttpToken {
    Method(String),
    Status(u16, String),
    Url(String),
    Field(String, String),
    Body(Vec<u8>),
    EndOfMessage,
}
