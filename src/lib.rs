use std::sync::{Arc, Mutex};

#[macro_use]
pub mod macros;

pub mod commands;
pub mod error;
pub mod files;
pub mod flow;
pub mod project;
pub mod render;

use hashbrown::HashMap;

pub use commands::Command;
pub use commands::build;
pub use build::addon::{Addon, AddonLocation};
pub use error::{HEMTTError, FileErrorLineNumber, IOPathError};
pub use files::{FileCache, RenderedFiles};
pub use flow::{Flow, Report, Task, Step};
pub use project::Project;

lazy_static::lazy_static! {
    pub static ref CACHED: Arc<Mutex<FileCache>> = Arc::new(Mutex::new(FileCache::new()));
    pub static ref RENDERED: Arc<Mutex<RenderedFiles>> = Arc::new(Mutex::new(RenderedFiles::new()));
    pub static ref REPORTS: Arc<Mutex<HashMap<String, Report>>> = Arc::new(Mutex::new(HashMap::new()));

    pub static ref CI: bool = std::env::args().find(|x| x == "--ci").is_some() || is_ci();
}

pub fn is_ci() -> bool {
    // TODO: replace with crate if a decent one comes along
    let checks = vec![
        "CI",
        "APPVEYOR",
        "SYSTEM_TEAMFOUNDATIONCOLLECTIONURI",
        "bamboo_planKey",
        "BITBUCKET_COMMIT",
        "BITRISE_IO",
        "BUDDY_WORKSPACE_ID",
        "BUILDKITE",
        "CIRCLECI",
        "CIRRUS_CI",
        "CODEBUILD_BUILD_ARN",
        "DRONE",
        "DSARI",
        "GITLAB_CI",
        "GO_PIPELINE_LABEL",
        "HUDSON_URL",
        "MAGNUM",
        "NETLIFY_BUILD_BASE",
        "PULL_REQUEST",
        "NEVERCODE",
        "SAILCI",
        "SEMAPHORE",
        "SHIPPABLE",
        "TDDIUM",
        "STRIDER",
        "TEAMCITY_VERSION",
        "TRAVIS",
    ];
    for check in checks {
        if std::env::var(check).is_ok() {
            return true;
        }
    }
    false
}
