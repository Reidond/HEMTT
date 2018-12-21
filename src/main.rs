extern crate clap;
use clap::{Arg, App, SubCommand};

#[cfg(target_os = "windows")]
extern crate winreg;

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

extern crate colored;

mod files;
mod project;
mod armake;
#[cfg(target_os = "windows")]
mod mikero;

use std::io::{stdin, stdout, Write};
use std::path::Path;

const HEMTT_FILE: &str = "hemtt.json";

fn input(text: &str) -> String {
  let mut s=String::new();
  print!("{}",text);
  stdout().flush().unwrap();
  stdin().read_line(&mut s).expect("Did not enter a correct string");
  if let Some('\n')=s.chars().next_back() {
    s.pop();
  }
  if let Some('\r')=s.chars().next_back() {
    s.pop();
  }
  s
}

fn main() {
  let matches = App::new("HEMTT")
                .version("0.1.0")
                .author("Synixe Brett <dev@synixe.com>")
                .about("Arma 3 Build Manager")
                .subcommand(SubCommand::with_name("init")
                            .about("Create a new HEMTT project from existing files")
                            .version("0.1")
                          )
                .subcommand(SubCommand::with_name("create")
                            .about("Create a new HEMTT project")
                            .version("0.1")
                          )
                .subcommand(SubCommand::with_name("addon")
                            .about("Create a new addon")
                            .version("0.1")
                            .arg(Arg::with_name("name")
                                .help("Component name")
                                .required(true))
                          )
                .subcommand(SubCommand::with_name("build")
                            .about("Build the project")
                            .version("0.1")
                            .arg(Arg::from_usage("-r --release 'Create a release version'"))
                            .arg(Arg::from_usage("--force 'Recreate any existing pbos'"))
                            .arg(Arg::with_name("toolchain")
                                .help("Toolchain to use, armake or mikero"))
                          )
                .subcommand(SubCommand::with_name("details")
                            .about("View the details of the current HEMTT project")
                            .version("0.1")
                          )
                .subcommand(SubCommand::with_name("download-cba-macros")
                            .about("Download the latest CBA common macros")
                            .version("0.1")
                          )
                .arg(Arg::from_usage("--no-cba 'Do not create CBA dependent files'"))
                .arg(Arg::from_usage("-n --no-color 'Do not use terminal colors'"))
                .get_matches();

  if matches.is_present("no-color") {
    colored::control::set_override(false);
  }
  let cba = !matches.is_present("no-cba");
  if let Some(command) = matches.subcommand_name() {
    match matches.subcommand_name().unwrap() {
      "init" => {
        if Path::new(HEMTT_FILE).exists() {
          let con = input("hemtt.json already exists, would you like to overwrite it? (y/n) ");
          if con != "y" {
            return;
          }
        }
        let name = input("Project Name: ");
        let prefix = input("Prefix: ");
        let author = input("Author: ");
        project::create(name, prefix, author);
      },
      "create" => {
        if Path::new(HEMTT_FILE).exists() {
          let con = input("hemtt.json already exists, would you like to overwrite it? (y/n) ");
          if con != "y" {
            return;
          }
        }
        let name = input("Project Name: ");
        let prefix = input("Prefix: ");
        let author = input("Author: ");
        let p = project::create(name, prefix, author);
        let main = "main".to_owned();
        files::modcpp(&p);
        files::create_addon(&main, &p);
        files::scriptmodhpp(&p);
        files::scriptversionhpp(&p);
        files::scriptmacroshpp(&p);
        files::script_component(&main, &p);
        files::pboprefix(&main, &p);
        files::configcpp(&main, &p, false);
        if cba {
          files::create_include();
        }
      },
      "addon" => {
        if let Some(args) = matches.subcommand_matches("addon") {
          if Path::new(HEMTT_FILE).exists() {
            let p = project::get_project();
            let name = args.value_of("name").expect("Name is a required field").to_owned();
            if Path::new(&format!("addons/{}", name)).exists() {
              println!("Addon {} already exists!", name);
              return;
            }
            println!("Creating addon: {}", name);
            files::create_addon(&name, &p).expect("error");
            files::pboprefix(&name, &p);
            files::script_component(&name, &p);
            files::configcpp(&name, &p, cba);
            if cba {
              files::xeh(&name, &p).expect("idk");
            }
          } else {
            println!("No HEMTT Project Found");
          }
        }
      },
      "build" => {
        if Path::new(HEMTT_FILE).exists() {
          if let Some(args) = matches.subcommand_matches("build") {
            let p = project::get_project();
            let mut toolchain = args.value_of("toolchain").unwrap_or("default");
            if toolchain == "default" {
              toolchain = &p.toolchain;
            }
            println!("Using Toolchain: {}", toolchain);
            if args.is_present("force") {
              println!("Removing existing PBOs");
              files::clear_pbos(&p);
            }
            match toolchain {
              "armake" => {
                let releases = armake::get_releases().unwrap();
                let latest = armake::get_latest(releases);
                let installed = armake::get_installed();
                println!("Current: {}", installed);
                println!("Available: {}", latest.tag_name);
                if (!Path::new("tools/armake").exists() && !Path::new("tools/armake.exe").exists()) || installed != latest.tag_name {
                  armake::download(&latest);
                }
                println!("Using armake {}", latest.tag_name);

                if args.is_present("release") {
                  armake::release(&p);
                } else {
                  armake::build(&p);
                }
              },
              #[cfg(target_os = "windows")]
              "mikero" => {
                if cfg!(windows) {
                  let tmp_pdrive = !Path::new("P:").exists();
                  if p.pdrive == "" {
                    println!("PDrive is not set! Add \"pdrive\": \"[Path to P Drive]\" to your hemtt.json file");
                    return;
                  } else {
                    if tmp_pdrive {
                      mikero::create_pdrive(&p);
                    }
                  }
                  let tools = mikero::toolchain();
                  match tools {
                    Ok(tools) => {
                      if args.is_present("release") {
                        tools.release(&p);
                      } else {
                        tools.build(&p);
                      }
                    },
                    Err(e) => {
                      panic!(e);
                    }
                  };
                  if tmp_pdrive {
                    mikero::remove_pdrive();
                  };
                } else {
                  println!("The mikero toolchain is not supported on your platform.");
                }
              },
              _ => {
                println!("{} is not a valid toolchain. Toolchains: armake, mikero", toolchain);
              }
            }
          }
        } else {
          println!("No HEMTT Project Found");
        }
      },
      "download-cba-macros" => {
        files::create_include();
      },
      "details" => {
        if Path::new(HEMTT_FILE).exists() {
          let p = project::get_project();
          println!("Name: {}", p.name);
          println!("Prefix: {}", p.prefix);
          println!("Author: {}", p.author);
          println!("Toolchain: {}", p.toolchain);
        } else {
          println!("No HEMTT Project Found");
        }
      },
      _ => {

      }
    }
  } else {
    println!("No command provided");
  }
}
