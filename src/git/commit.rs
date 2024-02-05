use std::process::Command;

use super::GitCmd;

#[derive(Default)]
pub struct Commit {
    amend: bool,
    message: Option<String>,
}

pub fn commit() -> Commit {
    Commit::default()
}

impl Commit {
    pub fn amend(self) -> Commit {
        Commit {
            amend: true,
            ..self
        }
    }

    pub fn message<T: Into<String>>(self, message: T) -> Commit {
        Commit {
            message: Some(message.into()),
            ..self
        }
    }
}

impl GitCmd for Commit {
    fn setup(self, cmd: &mut Command) {
        cmd.arg("commit");

        if self.amend {
            cmd.arg("--amend");
        }

        self.message.map(|msg| cmd.arg("-m").arg(msg));
    }
}
