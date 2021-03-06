use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("system error or I/O failure")]
    IoFailure(#[from] std::io::Error),

    #[error("the implementation returns malformed strings")]
    InvalidString(#[from] std::string::FromUtf8Error),

    #[error("failed to parse the string returned from implementation")]
    UnexpectedOutput(&'static str),

    #[error("cannot find any dialog implementation (kdialog/zanity)")]
    NoImplementation,

    #[error("the implementation reports error")]
    ImplementationError(String),
}

pub type Result<T> = std::result::Result<T, Error>;

pub trait Dialog {
    type Output;

    fn show(self) -> Result<Self::Output>;
}

mod message;
pub use message::*;

mod file;
pub use file::*;

mod r#impl;
