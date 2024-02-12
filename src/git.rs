use std::process::{Command, Stdio};
use std::str;

use crate::shell::{self, Error, Output};

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
pub use cherry_pick::cherry_pick;
pub use commit::commit;
pub use fetch::fetch;
pub use log::log;
pub use push::push;
pub use rev_list::rev_list;
pub use switch::switch;

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

pub trait GitCmd: Sized {
    fn setup(self, cmd: &mut Command);
}

impl<T> shell::Command for T
where
    T: GitCmd,
{
    fn spawn(self) -> Result<Output, Error> {
        let mut cmd = Command::new("git");
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        self.setup(&mut cmd);

        T::spawn_with_output(cmd)
    }
}
