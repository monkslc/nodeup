use clap::{App, Arg};
use std::env;

use nodeup::{Target, Version};

fn main() -> anyhow::Result<()> {
    let mut args = env::args();
    match args.next() {
        Some(cmd) if cmd == String::from("nodeup") => nodeup_command(),
        Some(cmd) if cmd == String::from("node") => node_command(args),
        Some(cmd) if cmd == String::from("npm") => npm_command(args),
        _ => todo!(),
    }
}

fn nodeup_command() -> anyhow::Result<()> {
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
            nodeup::download_node(target)?;
        }
        ("list", _) => {
            nodeup::list_versions()?;
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
            nodeup::active_versions()?;
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

fn node_command<I: std::iter::Iterator<Item = String>>(args: I) -> anyhow::Result<()> {
    nodeup::execute_bin("node", args)
}

fn npm_command<I: std::iter::Iterator<Item = String>>(args: I) -> anyhow::Result<()> {
    nodeup::execute_bin("npm", args)
}

fn link_command() -> anyhow::Result<()> {
    let links_path = nodeup::nodeup_files::links()?;
    match nodeup::link_node_bins(&links_path) {
        Ok(path) => {
            println!("Symlinks crated for node, npm, and npx. Make sure {} is in your PATH environment variable.", path.to_str().unwrap_or("[not_found]"));
            Ok(())
        }
        Err(e) => Err(e),
    }
}
