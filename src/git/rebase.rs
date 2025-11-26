use super::GitCmd;
use std::process::Command;

// FIXME: Add a derive(Builder)
pub struct Rebase {
    onto: String,
    interactive: bool,
    autosquash: bool,
    add_missing_prefix: bool,
}

pub fn rebase<T: Into<String>>(onto: T) -> Rebase {
    Rebase {
        onto: onto.into(),
        interactive: false,
        autosquash: false,
        add_missing_prefix: false,
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

    pub fn add_missing_prefix(self) -> Rebase {
        Rebase {
            add_missing_prefix: true,
            ..self
        }
    }
}

impl GitCmd for Rebase {
    fn setup(self, cmd: &mut Command) {
        if self.interactive {
            cmd.arg("-c").arg("sequence.editor=true");
        }

        if self.add_missing_prefix {
            cmd.arg("-c")
                .arg("core.editor=sed -i '1{/^gccrs: /!s/^/gccrs: /}'");
            cmd.arg("-c")
                .arg("sequence.editor=sed -i -e 's/pick/reword/g'");
        }

        cmd.arg("rebase");

        if self.interactive || self.add_missing_prefix {
            cmd.arg("--interactive");
        }

        if self.autosquash {
            cmd.arg("--autosquash");
        }
        cmd.arg(self.onto);
    }
}
