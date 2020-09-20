use nodeup::local::*;
use std::{env, path::PathBuf};

#[test]
fn linking() {
    env::set_var("NODEUP_LINKS", "/tmp/links");
    let actual = links().unwrap();
    let expected = PathBuf::from("/tmp/links");
    env::remove_var("NODEUP_LINKS");
    assert_eq!(actual, expected);

    let home = dirs::home_dir().unwrap();
    env::set_var("HOME", "/tmp/home");
    let actual = links().unwrap();
    let expected = PathBuf::from("/tmp/home/.local/bin/");
    env::set_var("HOME", home);
    assert_eq!(actual, expected);
}
