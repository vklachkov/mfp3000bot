use std::process::Command;

fn main() {
    let Ok(output) = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
    else {
        println!("cargo:rustc-env=GIT_COMMIT_HASH=unknown");
        return;
    };

    if !output.stdout.is_empty() {
        let hash = std::str::from_utf8(&output.stdout).unwrap();
        println!("cargo:rustc-env=GIT_COMMIT_HASH={hash}");
    } else {
        let error = std::str::from_utf8(&output.stderr).unwrap();
        panic!("Git error: {error}");
    }
}
