use clap::load_yaml;
use clap::App;
use std::{env, path::Path, process};

use nodeup::{
    local, registry,
    verify::{self, ConfigurationCheck},
    Target, Version,
};

type CLIResult = Result<(), Box<dyn std::error::Error>>;

fn main() {
    env_logger::init();

    let mut args = env::args();
    let command = args.next().expect("Command name should have been there");
    let executable = Path::new(&command)
        .file_name()
        .expect("Should've been able to find execuatable name");
    match executable {
        cmd if cmd == "nodeup" => {
            if let Err(e) = nodeup_command() {
                println!("{}", e);
                process::exit(1);
            }
        }
        cmd if cmd == "node" => {
            if let Err(e) = node_command(args) {
                println!("{}", e);
                process::exit(1);
            }
        }
        cmd if cmd == "npm" => {
            if let Err(e) = npm_command(args) {
                println!("{}", e);
                process::exit(1);
            }
        }
        cmd if cmd == "npx" => {
            if let Err(e) = npx_command(args) {
                println!("{}", e);
                process::exit(1);
            }
        }
        other => panic!("Unrecognized command: {:?}", other),
    }
}

fn nodeup_command() -> CLIResult {
    let yaml = load_yaml!("cli.yaml");
    let args = App::from_yaml(yaml).get_matches();
    match args.subcommand() {
        ("override", args) => match args.unwrap().subcommand() {
            ("add", args) => {
                let args = args.unwrap();
                let version = args.value_of("version").expect("Version required");
                let version = nodeup::Version::parse(version)?;
                let target = Target::from_version(version);
                if args.is_present("default") {
                    nodeup::change_default_target(target)?;
                } else {
                    nodeup::override_cwd(target)?;
                }
            }
            ("list", _) => {
                print_active_versions()?;
            }
            ("remove", args) => {
                let args = args.unwrap();
                if args.is_present("default") {
                    remove_default_override()?
                } else {
                    remove_override()?
                }
            }
            ("which", _) => {
                which()?;
            }
            _ => println!("Run nodeup override --help to see available commands"),
        },
        ("versions", args) => match args.unwrap().subcommand() {
            ("add", args) => {
                let args = args.unwrap();
                let version = args.value_of("version").expect("Version required");
                let version = if version == "lts" {
                    nodeup::get_latest_lts()?
                } else {
                    Version::parse(version)?
                };
                let target = Target::from_version(version);
                println!("Installing {}...", target);

                match args.value_of("path") {
                    Some(path) => download_node_toolchain_at_path(target, Path::new(path))?,
                    None => download_node_toolchain(target)?,
                }

                if args.is_present("default") {
                    nodeup::change_default_target(target)?;
                }

                if args.is_present("override") {
                    nodeup::override_cwd(target)?;
                }
            }
            ("remove", args) => {
                let version = args.unwrap().value_of("version").expect("Version required");
                let version = nodeup::Version::parse(version)?;
                let target = Target::from_version(version);
                nodeup::remove_node(target)?;
                println!("{} successfully removed", version);
            }
            ("list", _) => {
                print_versions()?;
            }
            ("lts", _) => {
                let version = nodeup::get_latest_lts()?;
                println!("{}", version)
            }
            _ => println!("Run nodeup versions --help to see available commands"),
        },
        ("control", args) => match args.unwrap().subcommand() {
            ("link", _) => {
                link_command()?;
            }
            ("verify", _) => verify()?,
            _ => println!("Run nodeup control --help to see available commands"),
        },
        _ => println!("Run nodeup --help to see available commands"),
    }
    Ok(())
}

fn node_command<I: std::iter::Iterator<Item = String>>(args: I) -> CLIResult {
    nodeup::execute_bin("node", args).map_err(|e| e.into())
}

fn npm_command<I: std::iter::Iterator<Item = String>>(args: I) -> CLIResult {
    nodeup::execute_bin("npm", args).map_err(|e| e.into())
}

fn npx_command<I: std::iter::Iterator<Item = String>>(args: I) -> CLIResult {
    nodeup::execute_bin("npx", args).map_err(|e| e.into())
}

fn link_command() -> CLIResult {
    let links_path = local::links()?;
    match nodeup::link_node_bins(&links_path) {
        Ok(path) => {
            println!("Symlinks created for node, npm, and npx. Make sure {} is in your PATH environment variable.", path.to_str().unwrap_or("[not_found]"));
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

fn download_node_toolchain(target: Target) -> CLIResult {
    let download_dir = local::download_dir()?;
    registry::download_node_toolchain(&download_dir, target).map_err(|e| e.into())
}

fn download_node_toolchain_at_path(target: Target, download_dir: &Path) -> CLIResult {
    registry::download_node_toolchain(&download_dir, target).map_err(|e| e.into())
}

fn print_versions() -> CLIResult {
    let download_dir = local::download_dir()?;
    let targets = nodeup::installed_versions(&download_dir)?;
    targets
        .iter()
        .for_each(|target| println!("{}", target.to_string()));
    Ok(())
}

fn print_active_versions() -> CLIResult {
    nodeup::get_active_targets()?.for_each(|(dir, target)| {
        println!("({}) {}", dir.display(), target);
    });

    Ok(())
}

fn verify() -> CLIResult {
    let path = local::links()?;
    match verify::verify_links(&path) {
        Ok(ConfigurationCheck::Correct) => {
            println!("Everything looks properly configured!");
            Ok(())
        }
        Ok(ConfigurationCheck::Incorrect(i)) => {
            println!("{}", i);
            process::exit(1);
        }
        Err(e) => Err(e.into()),
    }
}

fn remove_override() -> CLIResult {
    nodeup::remove_override().map_err(|e| e.into())
}

fn remove_default_override() -> CLIResult {
    nodeup::remove_default_override().map_err(|e| e.into())
}

fn which() -> CLIResult {
    let cwd = env::current_dir()?;
    let active_target = nodeup::which(&cwd)?;

    println!("{}", active_target);

    Ok(())
}
