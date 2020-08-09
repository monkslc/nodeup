use clap::{App, Arg};
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
    let args = App::new("Nodeup")
        .version("0.1")
        .author("Connor Monks")
        .about("Easily install and switch between multiple node versions")
        .subcommand(
            App::new("install")
                .about("install a new version of node")
                .arg(Arg::with_name("version").index(1).required(true)),
        )
        .subcommand(App::new("list").about("list all of the installed node versions"))
        .subcommand(
            App::new("default")
                .about("set the default node version")
                .arg(Arg::with_name("version").index(1).required(true)),
        )
        .subcommand(App::new("active").about("show active node versions for each override"))
        .subcommand(App::new("link").about("link node, npm and npx binaries"))
        .subcommand(
            App::new("remove")
                .about("set the default node version")
                .arg(Arg::with_name("version").index(1).required(true)),
        )
        .subcommand(App::new("lts").about("print the latest long term support version of node"))
        .subcommand(
            App::new("override")
                .about("override which node version gets used for the current directory and its descendents")
                .arg(Arg::with_name("version").index(1).required(true)),
        )
        .get_matches();

    match args.subcommand() {
        ("install", args) => {
            // safe to unwrap because version is required
            let version = args.unwrap().value_of("version").expect("Version required");
            let version = Version::parse(version)?;
            let target = Target::from_version(version);
            println!("Installing {}...", target);
            download_node_toolchain(target)?;
        }
        ("list", _) => {
            print_versions()?;
        }
        ("default", args) => {
            // safe to unwrap because version is required
            let version = args.unwrap().value_of("version").expect("Version required");
            let version = nodeup::Version::parse(version)?;
            let target = Target::from_version(version);
            println!("Changing the default node version to {}...", version);
            nodeup::change_default_target(target)?;
        }
        ("active", _) => {
            print_active_versions()?;
        }
        ("link", _) => {
            link_command()?;
        }
        ("remove", args) => {
            // safe to unwrap because version is required
            let version = args.unwrap().value_of("version").expect("Version required");
            let version = nodeup::Version::parse(version)?;
            let target = Target::from_version(version);
            nodeup::remove_node(target)?;
            println!("{} successfully removed", version);
        }
        ("lts", _) => {
            let version = nodeup::get_latest_lts()?;
            println!("{}", version)
        }
        ("override", args) => {
            let version = args.unwrap().value_of("version").expect("Version required");
            let version = nodeup::Version::parse(version)?;
            let target = Target::from_version(version);
            nodeup::override_cwd(target)?;
        }
        _ => todo!(),
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
