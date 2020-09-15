use nodeup::local::*;
use std::{env, path::PathBuf};

#[test]
fn find_download_dir() {
    env::set_var("NODEUP_DOWNLOADS", "/tmp/nodeup");
    let actual = download_dir().unwrap();
    let expected = PathBuf::from("/tmp/nodeup");
    env::remove_var("NODEUP_DOWNLOADS");
    assert_eq!(actual, expected);

    #[cfg(target_os = "linux")]
    {
        env::set_var("XDG_DATA_HOME", "/tmp/other-nodeup");
        let actual = download_dir().unwrap();
        let expected = PathBuf::from("/tmp/other-nodeup/nodeup");
        env::remove_var("XDG_DATA_HOME");
        assert_eq!(actual, expected);
    }

    let actual = download_dir().unwrap();

    #[cfg(target_os = "linux")]
    let expected = dirs::home_dir()
        .map(|dir| dir.join(".local").join("share").join("nodeup"))
        .unwrap();

    #[cfg(target_os = "macos")]
    let expected = dirs::home_dir()
        .map(|dir| {
            dir.join("Library")
                .join("Application Support")
                .join("nodeup")
        })
        .unwrap();

    assert_eq!(actual, expected);
}
