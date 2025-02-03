use std::process::{self, Command, Stdio};
use std::{fmt, io, str};

use thiserror::Error;

// TODO: Mark all subcommands types as must use
mod branch;
mod cherry_pick;
mod commit;
mod fetch;
mod log;
mod push;
mod rev_list;
mod switch;

pub use branch::{branch, StartingPoint};
pub use cherry_pick::{cherry_pick, cherry_pick_abort};
pub use commit::commit;
pub use fetch::fetch;
pub use log::log;
pub use push::push;
pub use rev_list::rev_list;
pub use switch::switch;

#[derive(Debug, Error)]
pub enum Error {
    IO(#[from] io::Error),
    Status(process::Output),
    Utf8(#[from] str::Utf8Error),
}

// FIXME:
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:#?}")
    }
}

// FIXME: Move to `log` module?
#[derive(Clone, Copy)]
pub enum Format {
    Hash,
    Title,
    Body,
}

impl Format {
    fn as_str(&self) -> &str {
        match self {
            Format::Hash => "%h",
            Format::Title => "%s",
            Format::Body => "%B",
        }
    }
}

pub struct Branch<T: Into<String>>(pub T);
pub struct Commit<T: Into<String>>(pub T);
pub struct Remote<T: Into<String>>(pub T);

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

pub trait GitCmd: Sized {
    // FIXME: Spawn needs to check the exit code and encode that in its return type - non-zero should be Err
    fn spawn(self) -> Result<Output, Error> {
        let mut cmd = Command::new("git");
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        self.setup(&mut cmd);

        let output = cmd.spawn()?.wait_with_output()?;

        if output.status.success() {
            Ok(output.try_into()?)
        } else {
            Err(Error::Status(output))
        }
    }

    fn setup(self, cmd: &mut Command);
}
