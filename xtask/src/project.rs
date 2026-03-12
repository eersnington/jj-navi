use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use semver::Version;
use serde_json::{Map, Value};
use time::OffsetDateTime;

use crate::error::ToolError;

pub(crate) fn find_repo_root(start: &Path) -> Result<PathBuf, ToolError> {
    for candidate in start.ancestors() {
        if is_repo_root(candidate) {
            return Ok(candidate.to_path_buf());
        }
    }

    Err(ToolError::message(
        "navi-release must run inside the jj-navi repo",
    ))
}

pub(crate) fn changelog_path(repo_root: &Path) -> PathBuf {
    repo_root.join("CHANGELOG.md")
}

pub(crate) fn today() -> String {
    let today = OffsetDateTime::now_utc().date();
    format!(
        "{:04}-{:02}-{:02}",
        today.year(),
        u8::from(today.month()),
        today.day()
    )
}

pub(crate) fn prepend_changelog(repo_root: &Path, section: &str) -> Result<(), ToolError> {
    let current = read_text(&changelog_path(repo_root))?;
    let first_entry_index = current.find("\n## ");
    let header = first_entry_index
        .map_or(current.trim_end(), |index| current[..index].trim_end())
        .to_owned();
    let rest = first_entry_index.map_or(String::new(), |index| {
        current[index..].trim_start().to_owned()
    });

    let next = if rest.is_empty() {
        format!("{header}\n\n{section}\n")
    } else {
        format!("{header}\n\n{section}\n\n{rest}\n")
    };

    write_text(&changelog_path(repo_root), &next)
}

pub(crate) fn changelog_has_version(repo_root: &Path, version: &str) -> Result<bool, ToolError> {
    Ok(read_text(&changelog_path(repo_root))?.contains(&format!("## v{version} - ")))
}

pub(crate) fn release_notes(repo_root: &Path, version: &str) -> Result<String, ToolError> {
    let changelog = read_text(&changelog_path(repo_root))?;
    let marker = format!("## v{version} - ");
    let Some(start) = changelog.find(&marker) else {
        return Err(ToolError::message(format!(
            "CHANGELOG entry for {version} not found"
        )));
    };

    let rest = &changelog[start..];
    let next_section = rest[marker.len()..]
        .find("\n## ")
        .map(|offset| offset + marker.len());
    let entry = next_section.map_or(rest, |end| &rest[..end]);
    Ok(format!("{}\n", entry.trim_end()))
}

pub(crate) fn current_release_version(repo_root: &Path) -> Result<Version, ToolError> {
    let cargo_version = current_cargo_version(repo_root)?;
    let package_version = current_package_version(repo_root)?;
    if cargo_version != package_version {
        return Err(ToolError::message(format!(
            "version drift before release: Cargo={cargo_version}, npm={package_version}"
        )));
    }
    Ok(cargo_version)
}

pub(crate) fn parse_version(value: &str) -> Result<Version, ToolError> {
    Ok(Version::parse(value)?)
}

pub(crate) fn sync_versions(repo_root: &Path, version: &str) -> Result<(), ToolError> {
    parse_version(version)?;

    let cargo_path = repo_root.join("Cargo.toml");
    let cargo_toml = read_text(&cargo_path)?;
    let updated_cargo_toml = cargo_toml
        .lines()
        .map(|line| {
            if line.starts_with("version = \"") {
                format!("version = \"{version}\"")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    write_text(&cargo_path, &updated_cargo_toml)?;

    let readme_path = repo_root.join("README.md");
    let readme = read_text(&readme_path)?;
    let Some(start) = readme.find("cargo install jj-navi --version ") else {
        return Err(ToolError::message("README install version not found"));
    };
    let end = readme[start..]
        .find('\n')
        .map(|offset| start + offset)
        .unwrap_or(readme.len());
    let mut next_readme = String::new();
    next_readme.push_str(&readme[..start]);
    next_readme.push_str(&format!("cargo install jj-navi --version {version}"));
    next_readme.push_str(&readme[end..]);
    write_text(&readme_path, &next_readme)?;

    let package_path = repo_root.join("npm/jj-navi/package.json");
    let mut package_json = read_json(&package_path)?;
    let package_object = package_json
        .as_object_mut()
        .ok_or_else(|| ToolError::message("npm package.json must be an object"))?;
    package_object.insert("version".to_owned(), Value::String(version.to_owned()));

    let platforms = supported_platforms(repo_root)?;
    let optional_dependencies = platforms.iter().fold(Map::new(), |mut map, platform| {
        map.insert(
            format!("jj-navi-{platform}"),
            Value::String(version.to_owned()),
        );
        map
    });
    package_object.insert(
        "optionalDependencies".to_owned(),
        Value::Object(optional_dependencies),
    );

    let publish_config = package_object
        .entry("publishConfig".to_owned())
        .or_insert_with(|| Value::Object(Map::new()));
    let publish_object = publish_config
        .as_object_mut()
        .ok_or_else(|| ToolError::message("publishConfig must be an object"))?;
    publish_object.insert("access".to_owned(), Value::String("public".to_owned()));
    publish_object.insert("provenance".to_owned(), Value::Bool(true));
    write_json(&package_path, &package_json)
}

pub(crate) fn ensure_versions_match(repo_root: &Path, version: &Version) -> Result<(), ToolError> {
    let cargo_version = current_cargo_version(repo_root)?;
    if &cargo_version != version {
        return Err(ToolError::message(format!(
            "Cargo.toml version mismatch: expected {version}, got {cargo_version}"
        )));
    }

    let package_version = current_package_version(repo_root)?;
    if &package_version != version {
        return Err(ToolError::message(format!(
            "npm/jj-navi/package.json version mismatch: expected {version}, got {package_version}"
        )));
    }

    let readme = read_text(&repo_root.join("README.md"))?;
    let expected = format!("cargo install jj-navi --version {version}");
    if !readme.contains(&expected) {
        return Err(ToolError::message(format!(
            "README version mismatch: expected line '{expected}'"
        )));
    }

    let package_json = read_json(&repo_root.join("npm/jj-navi/package.json"))?;
    let package_object = package_json
        .as_object()
        .ok_or_else(|| ToolError::message("npm package.json must be an object"))?;
    let optional_dependencies = package_object
        .get("optionalDependencies")
        .and_then(Value::as_object)
        .ok_or_else(|| ToolError::message("optionalDependencies missing"))?;

    let expected_platforms = supported_platforms(repo_root)?;
    if optional_dependencies.len() != expected_platforms.len() {
        return Err(ToolError::message(
            "npm optionalDependencies do not match supported platforms",
        ));
    }

    for platform in expected_platforms {
        let dependency_name = format!("jj-navi-{platform}");
        let dependency_version = optional_dependencies
            .get(&dependency_name)
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ToolError::message(format!("missing optional dependency {dependency_name}"))
            })?;

        if dependency_version != version.to_string() {
            return Err(ToolError::message(format!(
                "optional dependency mismatch for {dependency_name}: expected {version}, got {dependency_version}"
            )));
        }
    }

    let provenance = package_object
        .get("publishConfig")
        .and_then(Value::as_object)
        .and_then(|publish| publish.get("provenance"))
        .and_then(Value::as_bool);
    if provenance != Some(true) {
        return Err(ToolError::message(
            "npm publishConfig.provenance must be true",
        ));
    }

    Ok(())
}

pub(crate) fn read_text(path: &Path) -> Result<String, ToolError> {
    Ok(fs::read_to_string(path)?)
}

pub(crate) fn write_text(path: &Path, value: &str) -> Result<(), ToolError> {
    fs::write(path, value.replace("\r\n", "\n"))?;
    Ok(())
}

fn read_json(path: &Path) -> Result<Value, ToolError> {
    Ok(serde_json::from_str(&read_text(path)?)?)
}

fn write_json(path: &Path, value: &Value) -> Result<(), ToolError> {
    write_text(path, &(serde_json::to_string_pretty(value)? + "\n"))
}

fn is_repo_root(candidate: &Path) -> bool {
    has_vcs_marker(candidate) && has_repo_markers(candidate)
}

fn has_vcs_marker(candidate: &Path) -> bool {
    candidate.join(".jj").exists() || candidate.join(".git").exists()
}

fn has_repo_markers(candidate: &Path) -> bool {
    candidate.join("Cargo.toml").is_file()
        && candidate.join("CHANGELOG.md").is_file()
        && candidate.join("xtask").join("Cargo.toml").is_file()
}

fn current_cargo_version(repo_root: &Path) -> Result<Version, ToolError> {
    let cargo_toml = read_text(&repo_root.join("Cargo.toml"))?;
    let version = cargo_toml
        .lines()
        .find_map(|line| {
            line.strip_prefix("version = \"")
                .and_then(|rest| rest.strip_suffix('"'))
        })
        .ok_or_else(|| ToolError::message("could not read Cargo.toml version"))?;
    parse_version(version)
}

fn current_package_version(repo_root: &Path) -> Result<Version, ToolError> {
    let package_json = read_json(&repo_root.join("npm/jj-navi/package.json"))?;
    let version = package_json
        .get("version")
        .and_then(Value::as_str)
        .ok_or_else(|| ToolError::message("could not read npm package version"))?;
    parse_version(version)
}

fn supported_platforms(repo_root: &Path) -> Result<Vec<String>, ToolError> {
    let platforms = read_json(&repo_root.join("npm/scripts/platforms.json"))?;
    let object = platforms
        .as_object()
        .ok_or_else(|| ToolError::message("platforms.json must be an object"))?;
    let ordered = object
        .keys()
        .fold(BTreeMap::new(), |mut acc, key| {
            acc.insert(key.clone(), ());
            acc
        })
        .into_keys()
        .collect();
    Ok(ordered)
}

#[cfg(test)]
mod tests {
    use super::find_repo_root;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(name: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock before unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("jj-navi-{name}-{unique}"));
            fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent dirs");
        }
        fs::write(path, "").expect("write file");
    }

    fn make_repo_markers(root: &Path) {
        touch(&root.join("Cargo.toml"));
        touch(&root.join("CHANGELOG.md"));
        touch(&root.join("xtask").join("Cargo.toml"));
    }

    #[test]
    fn detects_jj_repo_root() {
        let temp = TempDir::new("jj-root");
        let root = temp.path().join("repo");
        fs::create_dir_all(root.join(".jj")).expect("create .jj dir");
        make_repo_markers(&root);

        assert_eq!(
            find_repo_root(&root.join("xtask")).expect("find repo root"),
            root
        );
    }

    #[test]
    fn detects_git_repo_root_from_nested_dir() {
        let temp = TempDir::new("git-root");
        let root = temp.path().join("repo");
        fs::create_dir_all(root.join(".git")).expect("create .git dir");
        make_repo_markers(&root);
        let nested = root.join("nested").join("deeper");
        fs::create_dir_all(&nested).expect("create nested dir");

        assert_eq!(find_repo_root(&nested).expect("find repo root"), root);
    }

    #[test]
    fn rejects_random_git_repo_without_project_markers() {
        let temp = TempDir::new("random-git-root");
        let root = temp.path().join("repo");
        fs::create_dir_all(root.join(".git")).expect("create .git dir");

        assert!(find_repo_root(&root).is_err());
    }
}
