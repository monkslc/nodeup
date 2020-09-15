use nodeup::local::*;
use std::{env, path::PathBuf};

#[test]
fn find_config_dir() {
    env::set_var("NODEUP_CONFIG", "/tmp/config");
    let actual = config_file().unwrap();
    let expected = PathBuf::from("/tmp/config/settings.toml");
    env::remove_var("NODEUP_CONFIG");
    assert_eq!(actual, expected);

    #[cfg(target_os = "linux")]
    {
        env::set_var("XDG_CONFIG_HOME", "/tmp/xdg-config");
        let actual = config_file().unwrap();
        let expected = PathBuf::from("/tmp/xdg-config/nodeup/settings.toml");
        env::remove_var("XDG_CONFIG_HOME");
        assert_eq!(actual, expected);
    }

    let actual = config_file().unwrap();

    #[cfg(target_os = "linux")]
    let expected = dirs::home_dir()
        .map(|dir| dir.join(".config").join("nodeup").join("settings.toml"))
        .unwrap();

    #[cfg(target_os = "macos")]
    let expected = dirs::home_dir()
        .map(|dir| {
            dir.join("Library")
                .join("Application Support")
                .join("nodeup")
                .join("settings.toml")
        })
        .unwrap();

    assert_eq!(actual, expected);
}
