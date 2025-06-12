use super::{GitCmd, Revision};

use std::process::Command;

#[derive(Default)]
pub struct Switch {
    to: String,
    create: bool,
    start_point: Option<String>,
    force: bool,
}

pub fn switch<T: Into<String>>(to: T) -> Switch {
    let d = Switch::default();
    Switch { to: to.into(), ..d }
}

impl Switch {
    pub fn create(self) -> Switch {
        Switch {
            create: true,
            ..self
        }
    }

    pub fn start_point<T: Into<String>>(self, Revision(revision): Revision<T>) -> Switch {
        Switch {
            start_point: Some(revision.into()),
            ..self
        }
    }

    pub fn force(self) -> Switch {
        Switch {
            force: true,
            ..self
        }
    }
}

impl GitCmd for Switch {
    fn setup(self, cmd: &mut Command) {
        cmd.arg("switch");
        if self.create {
            if self.force {
                cmd.arg("-C");
            } else {
                cmd.arg("-c");
            }
        }
        cmd.arg(self.to);
        self.start_point.map(|r| cmd.arg(r.as_str()));
    }
}
