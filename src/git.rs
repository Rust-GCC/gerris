use std::collections::HashMap;
use std::ffi::OsStr;

extern crate log as ext_log;
use ext_log::{error, info, warn};

use std::process::{self, Command, Stdio};
use std::{fmt, io};

use thiserror::Error;

// TODO: Mark all subcommands types as must use
mod branch;
mod cherry_pick;
mod fetch;
mod log;
mod merge;
mod mergebase;
mod rebase;
mod rev_list;
mod revparse;
mod show;
mod switch;

pub use branch::{branch, StartingPoint};
pub use cherry_pick::cherry_pick;
pub use fetch::fetch;
pub use log::log;
pub use merge::merge;
pub use mergebase::merge_base;
pub use rebase::rebase;
pub use rev_list::rev_list;
pub use revparse::revparse;
pub use show::show;
pub use switch::switch;

#[derive(Debug, Error)]
pub enum Error {
    IO(#[from] io::Error),
    Status(process::Output),
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
}

impl Format {
    fn as_str(&self) -> &str {
        match self {
            Format::Hash => "%h",
            Format::Title => "%s",
        }
    }
}

pub struct Revision<T: Into<String>>(pub T);
pub struct Branch<T: Into<String>>(pub T);
pub struct Commit<T: Into<String>>(pub T);

pub trait GitCmd: Sized {
    // FIXME: Spawn needs to check the exit code and encode that in its return type - non-zero should be Err
    fn spawn(self) -> Result<process::Output, Error> {
        let mut cmd = Command::new("git");
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        self.setup(&mut cmd);
        let output = cmd.spawn()?.wait_with_output()?;

        if output.status.success() {
            Ok(output)
        } else {
            Err(Error::Status(output))
        }
    }
    fn setup(self, cmd: &mut Command);
}

pub fn split_remote_branch(rev: &str) -> Option<(&str, &str)> {
    let split: Vec<&str> = rev.splitn(2, '/').collect();

    if split.len() < 2 {
        return None;
    }

    return Some((split[0], split[1]));
}

pub fn maybe_fetch_from_branch(rev: &str) -> Result<(), Error> {
    let (remote, branch) = if let Some((rem, br)) = split_remote_branch(rev) {
        info!("Remote: {rem}, branch: {br}");
        (Some(rem), br)
    } else {
        info!("branch spec has no remote: {rev}");
        (None, rev)
    };

    if let Some(rem) = remote {
        info!("Fetching from {rem}");
        fetch().remote(rem).spawn()?;
    }

    Ok(())
}
