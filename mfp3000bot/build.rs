use std::process::Command;

fn main() {
    let output = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .expect("git rev-parse should be successful");

    let git_hash = String::from_utf8(output.stdout).expect("git output should be valid UTF-8");

    println!("cargo:rustc-env=GIT_COMMIT_HASH={git_hash}");
}
