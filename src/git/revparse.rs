use super::{Branch, Format, GitCmd, Revision};

use std::process::Command;

// FIXME: Add a derive(Builder)
#[derive(Default)]
pub struct RevParse {
    revision: String,
}

pub fn revparse() -> RevParse {
    RevParse::default()
}

impl RevParse {
    pub fn revision<T: Into<String>>(self, Revision(revision): Revision<T>) -> RevParse {
        RevParse {
            revision: revision.into(),
        }
    }
}

impl GitCmd for RevParse {
    fn setup(self, cmd: &mut Command) {
        cmd.arg("rev-parse").arg(self.revision);
    }
}
