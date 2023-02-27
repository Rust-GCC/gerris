use std::error;
use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Debug)]
pub enum Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{self:#?}")
    }
}

impl error::Error for Error {}

pub fn prepare_commits() -> Result<(), Error> {
    todo!()
}
