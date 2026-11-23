//! The folder a launch with no remembered workspace opens on (#166): a
//! terminal launch keeps its cwd; a Dock / Start-menu launch, which inherits
//! `/` or Orbit's install folder, falls back to the home folder.

use super::first_launch_workspace;
use std::path::PathBuf;

fn p(path: &str) -> Option<PathBuf> {
    Some(PathBuf::from(path))
}

#[test]
fn a_terminal_launch_keeps_the_folder_it_was_started_in() {
    assert_eq!(
        first_launch_workspace(p("/work/project"), p("/opt/orbit/orbit-pi"), p("/home/me")),
        PathBuf::from("/work/project"),
    );
}

#[test]
fn the_install_folder_is_never_the_default_project() {
    assert_eq!(
        first_launch_workspace(p("/opt/orbit"), p("/opt/orbit/orbit-pi"), p("/home/me")),
        PathBuf::from("/home/me"),
    );
    assert_eq!(
        first_launch_workspace(
            p("/opt/orbit/resources"),
            p("/opt/orbit/orbit-pi"),
            p("/home/me")
        ),
        PathBuf::from("/home/me"),
    );
}

#[test]
fn a_launch_from_the_filesystem_root_falls_back_to_home() {
    assert_eq!(
        first_launch_workspace(p("/"), p("/Applications/Orbit.app/Contents/MacOS/orbit-pi"), p("/Users/me")),
        PathBuf::from("/Users/me"),
    );
}

#[test]
fn with_nothing_known_it_still_returns_a_path() {
    assert_eq!(first_launch_workspace(None, None, None), PathBuf::from("."));
    assert_eq!(first_launch_workspace(p("/"), None, None), PathBuf::from("."));
}
