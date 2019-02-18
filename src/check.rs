use regex::Regex;

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_changed() -> Vec<u8> {
  let output = if cfg!(target_os = "windows") {
      Command::new("cmd")
              .args(&["/C", "git diff --cached --name-only"])
              .output()
              .expect("failed to execute process")
  } else {
      Command::new("sh")
              .arg("-c")
              .arg("git diff --cached --name-only")
              .output()
              .expect("failed to execute process")
  };
  output.stdout
}

fn addons() -> Result<Vec<PathBuf>, std::io::Error> {
  let mut addons = vec![];
  for entry in fs::read_dir("addons")? {
    let entry = entry?;
    let path = entry.path();
    if !path.is_dir() { continue };
    addons.push(path);
  }
  Ok(addons)
}

pub fn debug(p: &crate::project::Project) -> Result<bool, std::io::Error> {
  let re = Regex::new(r"\|\s?#define (DEBUG_[^\s]*?)\s*?\|+?").unwrap();
  let changed = String::from_utf8(get_changed()).unwrap();
  let files = changed.lines().collect::<Vec<_>>();
  let mut error = false;
  for file in files {
    let path = PathBuf::from(file);
    let scp = path.as_path();
    if scp.exists() {
      let content = fs::read_to_string(scp)?;
      let replaced = content.replace("\n","||");
      if re.is_match(&replaced) {
        re.captures(&replaced).and_then(|cap| {
          println!("{} was detected in {:?}", cap.get(1)?.as_str(), path.as_path());
          error = true;
          Some(cap)
        });
      }
    }
  }
  Ok(error)
}
