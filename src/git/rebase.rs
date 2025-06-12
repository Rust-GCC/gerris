use super::{Branch, Format, GitCmd};
use std::process::Command;

// FIXME: Add a derive(Builder)
pub struct Rebase {
    onto: String,
    interactive: bool,
    autosquash: bool,
}

pub fn rebase<T: Into<String>>(onto: T) -> Rebase {
    Rebase {
        onto: onto.into(),
        interactive: false,
        autosquash: false,
    }
}

impl Rebase {
    pub fn interactive(self) -> Rebase {
        Rebase {
            interactive: true,
            ..self
        }
    }

    pub fn autosquash(self) -> Rebase {
        Rebase {
            autosquash: true,
            ..self
        }
    }
}

impl GitCmd for Rebase {
    fn setup(self, cmd: &mut Command) {
        if self.interactive {
            cmd.arg("-c").arg("sequence.editor=true");
        }

        cmd.arg("rebase");

        if self.interactive {
            cmd.arg("--interactive");
        }

        if self.autosquash {
            cmd.arg("--autosquash");
        }
        cmd.arg(self.onto);
    }
}
