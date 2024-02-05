use std::process::Command;

use super::GitCmd;

#[derive(Default)]
pub struct Fetch {
    remote: Option<String>,
}

pub fn fetch() -> Fetch {
    Fetch::default()
}

impl Fetch {
    pub fn remote<T: Into<String>>(self, remote: T) -> Fetch {
        Fetch {
            remote: Some(remote.into()),
        }
    }
}

impl GitCmd for Fetch {
    fn setup(self, cmd: &mut Command) {
        cmd.arg("fetch");

        self.remote.map(|r| cmd.arg(r));
    }
}
