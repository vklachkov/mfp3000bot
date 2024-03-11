use std::process::Command;

fn main() {
    let Ok(output) = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
    else {
        println!("cargo:rustc-env=GIT_COMMIT_HASH=unknown");
        return;
    };

    let git_hash = String::from_utf8(output.stdout).expect("git output should be valid UTF-8");

    println!("cargo:rustc-env=GIT_COMMIT_HASH={git_hash}");
}
