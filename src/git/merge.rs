use super::{Branch, Format, GitCmd};

use std::process::Command;

// FIXME: Add a derive(Builder)
pub struct Merge {
    other_branch: String,
    strategy: Option<String>,
    message: Option<String>,
    no_edit: bool,
}

pub fn merge<T: Into<String>>(other_branch: T) -> Merge {
    Merge {
        other_branch: other_branch.into(),
        strategy: None,
        message: None,
        no_edit: false,
    }
}

impl Merge {
    pub fn message<T: Into<String>>(self, message: T) -> Merge {
        Merge {
            message: Some(message.into()),
            ..self
        }
    }

    pub fn no_edit(self) -> Merge {
        Merge {
            no_edit: true,
            ..self
        }
    }

    // FIXME add better type for strategy
    pub fn strategy<T: Into<String>>(self, strategy: T) -> Merge {
        Merge {
            strategy: Some(strategy.into()),
            ..self
        }
    }
}

impl GitCmd for Merge {
    fn setup(self, cmd: &mut Command) {
        cmd.arg("merge");

        self.strategy.map(|s| cmd.arg("--strategy").arg(s));

        if self.no_edit {
            cmd.arg("--no-edit");
        }

        self.message.map(|m| cmd.arg("-m").arg(m.as_str()));
        cmd.arg(self.other_branch);
    }
}
