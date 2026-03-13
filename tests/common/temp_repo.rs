use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::{Builder, TempDir};

pub struct TempJjRepo {
    dir: TempDir,
}

impl TempJjRepo {
    pub fn new() -> Self {
        Self::new_with_prefix(".tmp")
    }

    pub fn new_with_prefix(prefix: &str) -> Self {
        let dir = Builder::new()
            .prefix(prefix)
            .tempdir()
            .expect("create temp dir");

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
        self.create_workspace_at(name, &path)
    }

    pub fn create_workspace_at(&self, name: &str, path: &Path) -> PathBuf {
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
        path.to_path_buf()
    }

    pub fn run(&self, args: &[&str]) -> String {
        run_jj(self.path(), args)
    }

    pub fn navi_config_path(&self) -> PathBuf {
        self.path()
            .join(".jj")
            .join("repo")
            .join("navi")
            .join("config.toml")
    }

    pub fn navi_metadata_path(&self) -> PathBuf {
        self.path()
            .join(".jj")
            .join("repo")
            .join("navi")
            .join("workspaces.toml")
    }

    pub fn write_navi_config(&self, contents: &str) {
        let path = self.navi_config_path();
        let parent = path.parent().expect("config parent");
        fs::create_dir_all(parent).expect("create navi config dir");
        fs::write(path, contents).expect("write navi config");
    }

    pub fn write_navi_metadata(&self, contents: &str) {
        let path = self.navi_metadata_path();
        let parent = path.parent().expect("metadata parent");
        fs::create_dir_all(parent).expect("create navi metadata dir");
        fs::write(path, contents).expect("write navi metadata");
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
