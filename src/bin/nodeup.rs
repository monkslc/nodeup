use clap::{App, Arg};

use nodeup;

fn main() -> anyhow::Result<()> {
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
        _ => todo!(),
    }
    Ok(())
}
