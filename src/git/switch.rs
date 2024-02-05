use super::GitCmd;

use std::process::Command;

pub struct Switch {
    to: String,
}

pub fn switch<T: Into<String>>(to: T) -> Switch {
    Switch { to: to.into() }
}

impl GitCmd for Switch {
    fn setup(self, cmd: &mut Command) {
        cmd.arg("switch").arg(self.to);
    }
}
