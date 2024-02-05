use std::process::Command;

use super::GitCmd;

pub enum StartingPoint<T: Into<String>> {
    Commit(T),
    Branch(T),
    Either(T),
}

#[derive(Default)]
pub struct Branch {
    name: Option<String>,
    starting_point: Option<String>,
}

pub fn branch() -> Branch {
    Branch::default()
}

impl Branch {
    pub fn name<T: Into<String>>(self, name: T) -> Branch {
        Branch {
            name: Some(name.into()),
            ..self
        }
    }

    pub fn starting_point<T: Into<String>>(self, starting_point: StartingPoint<T>) -> Branch {
        Branch {
            starting_point: Some(match starting_point {
                StartingPoint::Commit(s) | StartingPoint::Branch(s) | StartingPoint::Either(s) => {
                    s.into()
                }
            }),
            ..self
        }
    }
}

impl GitCmd for Branch {
    fn setup(self, cmd: &mut Command) {
        cmd.arg("branch");

        self.name.map(|n| cmd.arg(n));
        self.starting_point.map(|s| cmd.arg(s));
    }
}
