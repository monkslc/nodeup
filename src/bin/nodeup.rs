use clap::load_yaml;
use clap::App;
use std::env;

use nodeup::{local, registry, Target, Version};

type CLIResult = Result<(), Box<dyn std::error::Error>>;

fn main() {
    let mut args = env::args();
    match args.next() {
        Some(cmd) if cmd == "nodeup" => {
            if let Err(e) = nodeup_command() {
                println!("{}", e);
            }
        }
        Some(cmd) if cmd == "node" => {
            if let Err(e) = node_command(args) {
                println!("{}", e);
            }
        }
        Some(cmd) if cmd == "npm" => {
            if let Err(e) = npm_command(args) {
                println!("{}", e);
            }
        }
        _ => panic!("Unrecognized command"),
    }
}

fn nodeup_command() -> CLIResult {
    let yaml = load_yaml!("cli.yaml");
    let args = App::from_yaml(yaml).get_matches();
    match args.subcommand() {
        ("override", args) => match args.unwrap().subcommand() {
            ("default", args) => {
                let version = args.unwrap().value_of("version").expect("Version required");
                let version = nodeup::Version::parse(version)?;
                let target = Target::from_version(version);
                println!("Changing the default node version to {}...", version);
                nodeup::change_default_target(target)?;
            }
            ("set", args) => {
                let version = args.unwrap().value_of("version").expect("Version required");
                let version = nodeup::Version::parse(version)?;
                let target = Target::from_version(version);
                nodeup::override_cwd(target)?;
            }
            ("list", _) => {
                print_active_versions()?;
            }
            _ => panic!("Subcommand not recognized"),
        },
        ("versions", args) => match args.unwrap().subcommand() {
            ("install", args) => {
                let version = args.unwrap().value_of("version").expect("Version required");
                let version = Version::parse(version)?;
                let target = Target::from_version(version);
                println!("Installing {}...", target);
                download_node_toolchain(target)?;
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
            _ => panic!("Subcommand not recognized"),
        },
        ("control", args) => match args.unwrap().subcommand() {
            ("link", _) => {
                link_command()?;
            }
            _ => panic!("Subcommand not recognized"),
        },
        _ => panic!("Subcommand not recognized"),
    }
    Ok(())
}

fn node_command<I: std::iter::Iterator<Item = String>>(args: I) -> CLIResult {
    nodeup::execute_bin("node", args).map_err(|e| e.into())
}

fn npm_command<I: std::iter::Iterator<Item = String>>(args: I) -> CLIResult {
    nodeup::execute_bin("npm", args).map_err(|e| e.into())
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

fn print_versions() -> CLIResult {
    let download_dir = local::download_dir()?;
    let targets = nodeup::installed_versions(&download_dir)?;
    targets
        .iter()
        .for_each(|target| println!("{}", target.to_string()));
    Ok(())
}

fn print_active_versions() -> CLIResult {
    nodeup::active_versions()?
        .into_iter()
        .for_each(|(dir, target)| {
            println!("({}) {}", dir.display(), target);
        });

    Ok(())
}
