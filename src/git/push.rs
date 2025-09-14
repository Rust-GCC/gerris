use super::GitCmd;

use std::process::Command;

// FIXME: Add a derive(Builder)
#[derive(Default)]
pub struct Push {
    set_upstream: bool,
    force: bool,
    remote: Option<String>,
    refspec: Option<String>,
}

pub fn push() -> Push {
    Push::default()
}

impl Push {
    pub fn set_upstream(self) -> Push {
        Push {
            set_upstream: true,
            ..self
        }
    }

    pub fn force(self) -> Push {
        Push {
            force: true,
            ..self
        }
    }

    pub fn remote<T: Into<String>>(self, remote: T) -> Push {
        Push {
            remote: Some(remote.into()),
            ..self
        }
    }

    pub fn refspec<T: Into<String>>(self, refspec: T) -> Push {
        Push {
            refspec: Some(refspec.into()),
            ..self
        }
    }
}

impl GitCmd for Push {
    fn setup(self, cmd: &mut Command) {
        cmd.arg("push");
        if self.set_upstream {
            cmd.arg("--set-upstream");
        }

        self.remote.map(|x| cmd.arg(x));

        if self.force {
            cmd.arg("--force");
        }

        self.refspec.map(|r| cmd.arg(r));
    }
}
