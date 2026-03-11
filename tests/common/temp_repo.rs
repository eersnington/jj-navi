use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

pub struct TempJjRepo {
    dir: TempDir,
}

impl TempJjRepo {
    pub fn new() -> Self {
        let dir = TempDir::new().expect("create temp dir");

        run_jj(dir.path(), &["git", "init"]);
        run_jj(
            dir.path(),
            &["config", "set", "--repo", "user.name", "Test User"],
        );
        run_jj(
            dir.path(),
            &["config", "set", "--repo", "user.email", "test@example.com"],
        );
        run_jj(dir.path(), &["commit", "-m", "Initial commit"]);

        Self { dir }
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn repo_name(&self) -> String {
        self.path()
            .file_name()
            .and_then(|name| name.to_str())
            .expect("repo basename")
            .to_owned()
    }

    pub fn create_workspace(&self, name: &str) -> PathBuf {
        let path = self
            .path()
            .with_file_name(format!("{}.{}", self.repo_name(), name));
        run_jj(
            self.path(),
            &[
                "workspace",
                "add",
                "--name",
                name,
                path.to_str().expect("workspace path"),
            ],
        );
        path
    }

    pub fn run(&self, args: &[&str]) -> String {
        run_jj(self.path(), args)
    }

    pub fn run_at(path: &Path, args: &[&str]) -> String {
        run_jj(path, args)
    }

    pub fn rev_id(&self, revset: &str) -> String {
        self.run(&["log", "-r", revset, "--no-graph", "-T", "commit_id"])
            .trim()
            .to_owned()
    }
}

impl Default for TempJjRepo {
    fn default() -> Self {
        Self::new()
    }
}

fn run_jj(path: &Path, args: &[&str]) -> String {
    let output = Command::new("jj")
        .args(args)
        .current_dir(path)
        .output()
        .expect("run jj");

    assert!(
        output.status.success(),
        "jj {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8_lossy(&output.stdout).into_owned()
}
