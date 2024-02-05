use std::process::Command;

use super::{Branch, GitCmd, Remote};

#[derive(Default)]
pub struct Push {
    upstream: Option<String>,
    refspecs: Vec<String>,
}

pub fn push() -> Push {
    Push::default()
}

impl Push {
    pub fn upstream<T: Into<String>>(self, Remote(upstream): Remote<T>) -> Push {
        Push {
            upstream: Some(upstream.into()),
            ..self
        }
    }

    pub fn branch<T: Into<String>>(self, Branch(branch): Branch<T>) -> Push {
        let mut refspecs = self.refspecs;
        refspecs.push(branch.into());

        Push { refspecs, ..self }
    }
}

impl GitCmd for Push {
    fn setup(self, cmd: &mut Command) {
        cmd.arg("push");

        self.upstream.map(|remote| cmd.arg("-u").arg(remote));
        self.refspecs.iter().for_each(|r| {
            cmd.arg(r);
        });
    }
}
