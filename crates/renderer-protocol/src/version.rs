use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VersionData {
    version_code: (u16, u16, u16),
    version_string: String,
}

impl VersionData {
    pub fn current() -> Self {
        const VERSION: &str = env!("CARGO_PKG_VERSION");
        const VERSION_MAJOR: &str = env!("CARGO_PKG_VERSION_MAJOR");
        const VERSION_MINOR: &str = env!("CARGO_PKG_VERSION_MINOR");
        const VERSION_PATCH: &str = env!("CARGO_PKG_VERSION_PATCH");
        Self {
            version_code: (
                VERSION_MAJOR.parse().unwrap(),
                VERSION_MINOR.parse().unwrap(),
                VERSION_PATCH.parse().unwrap(),
            ),
            version_string: String::from(VERSION),
        }
    }
}

impl Display for VersionData {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}.{}.{})",
            self.version_string, self.version_code.0, self.version_code.1, self.version_code.2
        )
    }
}
