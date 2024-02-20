use std::io;
use std::process;
use std::str;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("i/o error: {0}")]
    IO(#[from] io::Error),
    #[error("non-zero exit code: {} {0:?}", .0.status)]
    Status(process::Output),
    #[error("invalid UTF8: {0}")]
    Utf8(#[from] str::Utf8Error),
}

#[derive(Debug)]
pub struct Output {
    pub status: process::ExitStatus,
    pub stdout: String,
    pub stderr: Vec<u8>,
}

impl TryFrom<process::Output> for Output {
    type Error = str::Utf8Error;

    fn try_from(out: process::Output) -> Result<Self, Self::Error> {
        let stdout = str::from_utf8(out.stdout.as_slice())?;
        let stdout = stdout.trim_end().to_string();

        Ok(Output {
            status: out.status,
            stderr: out.stderr,
            stdout,
        })
    }
}

// FIXME: Should we have something like a shell::Command::new(<bin>) function
// which setups the process::Command and pipes both stderr and stdout?
pub trait Command: Sized {
    fn spawn_with_output(mut cmd: process::Command) -> Result<Output, Error> {
        let output = cmd.spawn()?.wait_with_output()?;

        if output.status.success() {
            Ok(output.try_into()?)
        } else {
            Err(Error::Status(output))
        }
    }

    // FIXME: Documentation: Spawn needs to check the exit code and encode that in its return type - non-zero should be Err
    fn spawn(self) -> Result<Output, Error>;
}
