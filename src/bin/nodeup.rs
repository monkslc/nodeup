use clap::{App, Arg};
use std::env;

use nodeup;

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
        .get_matches();

    match args.subcommand() {
        ("install", args) => {
            // safe to unwrap because version is required
            let version = args.unwrap().value_of("version").expect("Version required");
            let version = nodeup::Version::parse(version)?;
            println!("Installing node {}...", version);
            nodeup::download_node(version)?;
        }
        ("list", _) => {
            nodeup::list_versions()?;
        }
        ("default", args) => {
            // safe to unwrap because version is required
            let version = args.unwrap().value_of("version").expect("Version required");
            let version = nodeup::Version::parse(version)?;
            println!("Changing the default node version to {}", version);
            nodeup::change_default_version(version)?;
        }
        ("active", _) => {
            nodeup::active_versions()?;
        }
        ("link", _) => {
            nodeup::link()?;
            println!("Add the following to your .bashrc:\nexport PATH=\"$HOME/.nodeup/bin/\":$PATH")
        }
        ("remove", args) => {
            // safe to unwrap because version is required
            let version = args.unwrap().value_of("version").expect("Version required");
            let version = nodeup::Version::parse(version)?;
            nodeup::remove_node(version)?;
            println!("{} successfully removed", version);
        }
        ("lts", _) => {
            let version = nodeup::get_latest_lts()?;
            println!("{}", version)
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
