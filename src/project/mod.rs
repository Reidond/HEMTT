use std::collections::{BTreeMap, HashMap};
use std::env;
use std::path::{Path, PathBuf};

use config::{Config, Environment, File};
use handlebars::to_json;
use serde::{Deserialize, Serialize};
use serde_json::value::Value as Json;

use crate::error::PrintableError;
use crate::HEMTTError;

mod signing;

#[derive(Clone, Deserialize, Serialize)]
pub struct Project {
    pub name: String,
    pub prefix: String,
    pub author: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default = "String::new")]
    pub template: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default = "String::new")]
    pub version: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default = "String::new")]
    pub modname: String,
    #[serde(default = "default_mainprefix")]
    pub mainprefix: String,

    #[serde(skip_serializing_if = "HashMap::is_empty")]
    #[serde(default = "HashMap::new")]
    #[serde(rename(deserialize = "headerexts"))] // DEPRECATED
    pub header_exts: HashMap<String, String>,

    // Files
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default = "default_include")]
    pub include: Vec<PathBuf>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default = "Vec::new")]
    pub exclude: Vec<String>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default = "Vec::new")]
    pub files: Vec<String>,

    // Signing
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default = "default_reuse_private_key")]
    pub reuse_private_key: Option<bool>,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default = "String::new")]
    #[serde(rename(deserialize = "keyname"))] // DEPRECATED
    key_name: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default = "String::new")]
    #[serde(rename(deserialize = "signame"))] // DEPRECATED
    pub sig_name: String,

    #[serde(default = "default_sig_version")]
    #[serde(rename(deserialize = "sigversion"))] // DEPRECATED
    pub sig_version: u8,

    // Scripts
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default = "Vec::new")]
    pub check: Vec<String>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default = "Vec::new")]
    pub prebuild: Vec<String>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default = "Vec::new")]
    pub postbuild: Vec<String>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default = "Vec::new")]
    pub releasebuild: Vec<String>,

    #[serde(skip_serializing_if = "HashMap::is_empty")]
    #[serde(default = "HashMap::new")]
    pub scripts: HashMap<String, crate::BuildScript>,
}
impl Project {
    pub fn new(name: String, prefix: String, author: String, template: String) -> Self {
        Self {
            name,
            prefix,
            author,
            template,

            version: String::new(),

            modname: String::new(),
            mainprefix: default_mainprefix(),

            header_exts: HashMap::new(),

            include: default_include(),
            exclude: Vec::new(),
            files: if std::path::Path::new("mod.cpp").exists() {
                vec!["mod.cpp".to_owned()]
            } else {
                Vec::new()
            },

            reuse_private_key: default_reuse_private_key(),
            key_name: String::new(),
            sig_name: String::new(),
            sig_version: default_sig_version(),

            check: Vec::new(),
            postbuild: Vec::new(),
            prebuild: Vec::new(),
            releasebuild: Vec::new(),
            scripts: HashMap::new(),
        }
    }

    pub fn read() -> Result<Self, HEMTTError> {
        let mut p = Config::new();
        let env = environment();
        let root = find_root()?;
        debug!("Root Directory: {:?}", root);
        std::env::set_current_dir(root)?;

        if Path::new("hemtt.toml").exists() || Path::new("hemtt.json").exists() {
            // Single file
            p.merge(File::with_name("hemtt").required(true))?;
        } else {
            // Project folder
            if !Path::new(".hemtt/").exists() {
                return Err(HEMTTError::simple("No HEMTT project folder"));
            }
            p.merge(File::with_name(".hemtt/base").required(true))?;
            p.merge(File::with_name(&format!(".hemtt/{}", env)).required(false))?;
            p.merge(File::with_name(".hemtt/local").required(false))?;
        }

        p.merge(Environment::with_prefix("app"))?;

        p.try_into().map_err(From::from)
    }

    /// Values used for rendering
    pub fn get_variables(&self) -> BTreeMap<&'static str, Json> {
        let mut vars = BTreeMap::new();
        vars.insert("author", to_json(self.author.clone()));
        vars.insert("env", to_json(environment()));
        vars.insert("mainprefix", to_json(self.mainprefix.clone()));
        vars.insert("name", to_json(self.name.clone()));
        vars.insert("prefix", to_json(self.prefix.clone()));
        vars.insert("version", to_json(self.version().unwrap_or_print()));
        vars
    }

    /// Render a handlebars string
    pub fn render(&self, text: &str, filename: Option<&str>) -> Result<String, HEMTTError> {
        crate::render::run(text, filename, &self.get_variables())
    }

    /// `@modname` without `@`, uses prefix if undefined by project file
    pub fn modname(&self) -> Result<String, HEMTTError> {
        Ok(if self.modname.is_empty() {
            self.prefix.clone()
        } else {
            self.render(&self.modname, Some("project:modname"))?
        })
    }

    /// Version number as defined or detected by the templating engine
    pub fn version(&self) -> Result<String, HEMTTError> {
        if self.version.is_empty() {
            let template = crate::commands::Template::new();
            template.get_version()
        } else {
            Ok(self.version.clone().trim().to_string())
        }
    }

    /// Release directory `releases/{version}/@{modname}`
    pub fn release_dir(&self) -> Result<PathBuf, HEMTTError> {
        let version = self.version()?;
        let modname = self.modname()?;
        Ok(PathBuf::from(iformat!("releases/{version}/@{modname}", version, modname)))
    }
}

pub fn environment() -> String {
    env::var("ENV").unwrap_or_else(|_| if *crate::CI { "ci".into() } else { "dev".into() })
}

/// Finds the root of the project
pub fn find_root() -> Result<PathBuf, HEMTTError> {
    let mut dir = std::env::current_dir().unwrap();
    loop {
        let mut search = dir.clone();
        search.push(".hemtt");
        if search.exists() {
            search.pop();
            return Ok(search);
        } else {
            let mut search = dir.clone();
            search.push("hemtt.toml");
            if search.exists() {
                search.pop();
                return Ok(search);
            }
        }
        dir.pop();
        search.pop();
        if dir == search {
            return Err(HEMTTError::simple("No HEMTT Project File was found"));
        }
    }
}

fn default_include() -> Vec<PathBuf> {
    let mut includes = vec![];

    if PathBuf::from("./include").exists() {
        includes.push(PathBuf::from("./include"));
    }

    includes
}

fn default_mainprefix() -> String {
    String::from("z")
}

fn default_reuse_private_key() -> Option<bool> {
    None
}

pub fn default_sig_version() -> u8 {
    3
}
