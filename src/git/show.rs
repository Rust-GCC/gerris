use super::{Branch, Format, GitCmd};
use std::process::Command;

// FIXME: Add a derive(Builder)
pub struct Show {
    format: Option<String>,
    no_patch: bool,
    rev: String,
}

pub fn show<T: Into<String>>(rev: T) -> Show {
    Show {
        rev: rev.into(),
        no_patch: false,
        format: None,
    }
}

impl Show {
    pub fn no_patch(self) -> Show {
        Show {
            no_patch: true,
            ..self
        }
    }

    pub fn format<T: Into<String>>(self, format: T) -> Show {
        Show {
            format: Some(format.into()),
            ..self
        }
    }
}

impl GitCmd for Show {
    fn setup(self, cmd: &mut Command) {
        cmd.arg("show");
        self.format.map(|x| cmd.arg(format!("--format={x}")));
        if self.no_patch {
            cmd.arg("--no-patch");
        }
        cmd.arg(self.rev);
    }
}
