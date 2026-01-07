use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct ErrorWrapper(Box<dyn Error>);

impl Error for ErrorWrapper {}
unsafe impl Send for ErrorWrapper {}

impl Display for ErrorWrapper {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ErrorWrapper {
    pub fn new(err: Box<dyn Error>) -> Self {
        Self(err)
    }
}