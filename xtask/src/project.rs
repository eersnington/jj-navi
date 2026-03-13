use std::fs;
use std::path::{Path, PathBuf};

use semver::Version;
use serde::Deserialize;
use serde_json::{Map, Value};
use toml_edit::{DocumentMut, Item, value};

use crate::error::ToolError;

pub(crate) fn find_repo_root(start: &Path) -> Result<PathBuf, ToolError> {
    for candidate in start.ancestors() {
        if is_repo_root(candidate) {
            return Ok(candidate.to_path_buf());
        }
    }

    Err(ToolError::Message(
        "navi-release must run inside the jj-navi repo".to_owned(),
    ))
}

pub(crate) fn parse_version(value: &str) -> Result<Version, ToolError> {
    Ok(Version::parse(value)?)
}

struct CargoManifest {
    path: PathBuf,
    document: DocumentMut,
}

impl CargoManifest {
    fn load(repo_root: &Path) -> Result<Self, ToolError> {
        let path = repo_root.join("Cargo.toml");
        let document = read_text(&path)?.parse::<DocumentMut>()?;
        Ok(Self { path, document })
    }

    fn version(&self) -> Result<Version, ToolError> {
        let version = self
            .document
            .get("package")
            .and_then(Item::as_table_like)
            .and_then(|package| package.get("version"))
            .and_then(Item::as_str)
            .ok_or_else(|| {
                ToolError::Message("could not read Cargo.toml package version".to_owned())
            })?;
        parse_version(version)
    }

    fn set_version(&mut self, version: &str) -> Result<(), ToolError> {
        parse_version(version)?;

        let package = self
            .document
            .get_mut("package")
            .and_then(Item::as_table_like_mut)
            .ok_or_else(|| {
                ToolError::Message("could not update Cargo.toml package version".to_owned())
            })?;

        package.insert("version", value(version));
        Ok(())
    }

    fn write(&self) -> Result<(), ToolError> {
        write_text(&self.path, &self.document.to_string())
    }
}

pub(crate) fn read_json_typed<T>(path: &Path) -> Result<T, ToolError>
where
    T: for<'de> Deserialize<'de>,
{
    Ok(serde_json::from_str(&read_text(path)?)?)
}

pub(crate) fn current_cargo_version(repo_root: &Path) -> Result<Version, ToolError> {
    CargoManifest::load(repo_root)?.version()
}

pub(crate) fn current_package_version(repo_root: &Path) -> Result<Version, ToolError> {
    let package_json = read_json_value(&repo_root.join("npm/jj-navi/package.json"))?;
    let version = package_json
        .get("version")
        .and_then(Value::as_str)
        .ok_or_else(|| ToolError::Message("could not read npm package version".to_owned()))?;
    parse_version(version)
}

pub(crate) fn sync_versions(repo_root: &Path, version: &str) -> Result<(), ToolError> {
    parse_version(version)?;

    let mut cargo_manifest = CargoManifest::load(repo_root)?;
    cargo_manifest.set_version(version)?;
    cargo_manifest.write()?;

    let readme_path = repo_root.join("README.md");
    let readme = read_text(&readme_path)?;
    let next_readme = update_root_readme_version(&readme, version)?;
    write_text(&readme_path, &next_readme)?;
    write_text(&repo_root.join("npm/jj-navi/README.md"), &next_readme)?;

    let package_path = repo_root.join("npm/jj-navi/package.json");
    let mut package_json = read_json_value(&package_path)?;
    let package_object = package_json
        .as_object_mut()
        .ok_or_else(|| ToolError::Message("npm package.json must be an object".to_owned()))?;
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
        .ok_or_else(|| ToolError::Message("publishConfig must be an object".to_owned()))?;
    publish_object.insert("access".to_owned(), Value::String("public".to_owned()));
    publish_object.insert("provenance".to_owned(), Value::Bool(true));
    write_json_value(&package_path, &package_json)
}

pub(crate) fn ensure_versions_match(repo_root: &Path, version: &Version) -> Result<(), ToolError> {
    let cargo_version = current_cargo_version(repo_root)?;
    if &cargo_version != version {
        return Err(ToolError::Message(format!(
            "Cargo.toml version mismatch: expected {version}, got {cargo_version}",
        )));
    }

    let package_version = current_package_version(repo_root)?;
    if &package_version != version {
        return Err(ToolError::Message(format!(
            "npm/jj-navi/package.json version mismatch: expected {version}, got {package_version}",
        )));
    }

    let readme = read_text(&repo_root.join("README.md"))?;
    let expected = format!("cargo install jj-navi --version {version}");
    if !readme.contains(&expected) {
        return Err(ToolError::Message(format!(
            "README version mismatch: expected line '{expected}'",
        )));
    }

    let wrapper_readme = read_text(&repo_root.join("npm/jj-navi/README.md"))?;
    if wrapper_readme != readme {
        return Err(ToolError::Message(
            "npm/jj-navi/README.md must stay in sync with README.md".to_owned(),
        ));
    }

    let package_json = read_json_value(&repo_root.join("npm/jj-navi/package.json"))?;
    let package_object = package_json
        .as_object()
        .ok_or_else(|| ToolError::Message("npm package.json must be an object".to_owned()))?;
    let optional_dependencies = package_object
        .get("optionalDependencies")
        .and_then(Value::as_object)
        .ok_or_else(|| ToolError::Message("optionalDependencies missing".to_owned()))?;

    let expected_platforms = supported_platforms(repo_root)?;
    if optional_dependencies.len() != expected_platforms.len() {
        return Err(ToolError::Message(
            "npm optionalDependencies do not match supported platforms".to_owned(),
        ));
    }

    for platform in expected_platforms {
        let dependency_name = format!("jj-navi-{platform}");
        let dependency_version = optional_dependencies
            .get(&dependency_name)
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ToolError::Message(format!("missing optional dependency {dependency_name}"))
            })?;

        if dependency_version != version.to_string() {
            return Err(ToolError::Message(format!(
                "optional dependency mismatch for {dependency_name}: expected {version}, got {dependency_version}",
            )));
        }
    }

    let provenance = package_object
        .get("publishConfig")
        .and_then(Value::as_object)
        .and_then(|publish| publish.get("provenance"))
        .and_then(Value::as_bool);
    let access = package_object
        .get("publishConfig")
        .and_then(Value::as_object)
        .and_then(|publish| publish.get("access"))
        .and_then(Value::as_str);
    if access != Some("public") {
        return Err(ToolError::Message(
            "npm publishConfig.access must be public".to_owned(),
        ));
    }
    if provenance != Some(true) {
        return Err(ToolError::Message(
            "npm publishConfig.provenance must be true".to_owned(),
        ));
    }

    Ok(())
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
        && candidate.join(".github").join("workflows").is_dir()
        && candidate
            .join("npm")
            .join("jj-navi")
            .join("package.json")
            .is_file()
        && candidate.join("xtask").join("Cargo.toml").is_file()
}

fn supported_platforms(repo_root: &Path) -> Result<Vec<String>, ToolError> {
    let platforms = read_json_value(&repo_root.join("npm/scripts/platforms.json"))?;
    let object = platforms
        .as_object()
        .ok_or_else(|| ToolError::Message("platforms.json must be an object".to_owned()))?;
    Ok(object.keys().cloned().collect())
}

fn update_root_readme_version(readme: &str, version: &str) -> Result<String, ToolError> {
    let Some(start) = readme.find("cargo install jj-navi --version ") else {
        return Err(ToolError::Message(
            "README install version not found".to_owned(),
        ));
    };
    let end = readme[start..]
        .find('\n')
        .map(|offset| start + offset)
        .unwrap_or(readme.len());
    let mut next_readme = String::new();
    next_readme.push_str(&readme[..start]);
    next_readme.push_str(&format!("cargo install jj-navi --version {version}"));
    next_readme.push_str(&readme[end..]);
    Ok(next_readme)
}

fn read_text(path: &Path) -> Result<String, ToolError> {
    Ok(fs::read_to_string(path)?)
}

fn write_text(path: &Path, value: &str) -> Result<(), ToolError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, value.replace("\r\n", "\n"))?;
    Ok(())
}

fn read_json_value(path: &Path) -> Result<Value, ToolError> {
    Ok(serde_json::from_str(&read_text(path)?)?)
}

fn write_json_value(path: &Path, value: &Value) -> Result<(), ToolError> {
    write_text(path, &(serde_json::to_string_pretty(value)? + "\n"))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        CargoManifest, ensure_versions_match, find_repo_root, is_repo_root, sync_versions,
    };

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

    fn write(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent dirs");
        }
        fs::write(path, contents).expect("write file");
    }

    fn make_repo_markers(root: &Path) {
        write(&root.join("Cargo.toml"), "");
        write(&root.join("CHANGELOG.md"), "");
        write(&root.join("xtask").join("Cargo.toml"), "");
        write(&root.join("npm").join("jj-navi").join("package.json"), "");
        fs::create_dir_all(root.join(".github").join("workflows")).expect("create workflows dir");
    }

    fn make_release_repo(root: &Path, wrapper_readme: &str) {
        fs::create_dir_all(root.join(".jj")).expect("create .jj dir");
        write(
            &root.join("Cargo.toml"),
            "[package]\nname = \"jj-navi\"\nversion = \"1.2.3\"\n",
        );
        write(
            &root.join("README.md"),
            "# jj-navi\n\n```sh\ncargo install jj-navi --version 1.2.3\n```\n",
        );
        write(
            &root.join("npm").join("jj-navi").join("README.md"),
            wrapper_readme,
        );
        write(
            &root.join("npm").join("jj-navi").join("package.json"),
            concat!(
                "{\n",
                "  \"name\": \"jj-navi\",\n",
                "  \"version\": \"1.2.3\",\n",
                "  \"optionalDependencies\": {\n",
                "    \"jj-navi-linux-x64\": \"1.2.3\"\n",
                "  },\n",
                "  \"publishConfig\": {\n",
                "    \"access\": \"public\",\n",
                "    \"provenance\": true\n",
                "  }\n",
                "}\n"
            ),
        );
        write(
            &root.join("npm").join("scripts").join("platforms.json"),
            "{\n  \"linux-x64\": {}\n}\n",
        );
        write(
            &root.join("CHANGELOG.md"),
            "# Changelog\n\nAll notable changes to `jj-navi` live here.\n\n## v1.2.3 - 2026-03-13\n",
        );
        write(&root.join("xtask").join("Cargo.toml"), "");
        fs::create_dir_all(root.join(".github").join("workflows")).expect("create workflows dir");
    }

    #[test]
    fn cargo_manifest_updates_package_version_without_touching_dependencies() {
        let temp = TempDir::new("cargo-manifest-version-update");
        let root = temp.path().join("repo");
        fs::create_dir_all(root.join(".jj")).expect("create .jj dir");
        write(
            &root.join("Cargo.toml"),
            concat!(
                "[package]\n",
                "name = \"jj-navi\"\n",
                "version = \"0.1.0\" # keep comment\n\n",
                "[dependencies]\n",
                "serde = { version = \"1\" }\n"
            ),
        );

        let mut manifest = CargoManifest::load(&root).expect("load manifest");
        manifest.set_version("0.2.0").expect("set version");
        manifest.write().expect("write manifest");

        let updated = fs::read_to_string(root.join("Cargo.toml")).expect("read updated manifest");

        assert!(updated.contains("version = \"0.2.0\""));
        assert!(updated.contains("serde = { version = \"1\" }"));
    }

    #[test]
    fn sync_versions_updates_wrapper_readme() {
        let temp = TempDir::new("wrapper-readme-sync");
        let root = temp.path().join("repo");
        make_release_repo(&root, "stale wrapper readme\n");

        sync_versions(&root, "1.2.4").expect("sync versions");

        ensure_versions_match(&root, &"1.2.4".parse().expect("parse version"))
            .expect("versions should match");
    }

    #[test]
    fn rejects_wrapper_readme_drift() {
        let temp = TempDir::new("wrapper-readme-drift");
        let root = temp.path().join("repo");
        make_release_repo(&root, "stale wrapper readme\n");

        let error = ensure_versions_match(&root, &"1.2.3".parse().expect("parse version"))
            .expect_err("versions should fail");
        assert!(
            error
                .to_string()
                .contains("npm/jj-navi/README.md must stay in sync with README.md")
        );
    }

    #[test]
    fn rejects_non_public_publish_access() {
        let temp = TempDir::new("publish-access-drift");
        let root = temp.path().join("repo");
        make_release_repo(
            &root,
            "# jj-navi\n\n```sh\ncargo install jj-navi --version 1.2.3\n```\n",
        );
        write(
            &root.join("npm").join("jj-navi").join("package.json"),
            concat!(
                "{\n",
                "  \"name\": \"jj-navi\",\n",
                "  \"version\": \"1.2.3\",\n",
                "  \"optionalDependencies\": {\n",
                "    \"jj-navi-linux-x64\": \"1.2.3\"\n",
                "  },\n",
                "  \"publishConfig\": {\n",
                "    \"access\": \"restricted\",\n",
                "    \"provenance\": true\n",
                "  }\n",
                "}\n"
            ),
        );

        let error = ensure_versions_match(&root, &"1.2.3".parse().expect("parse version"))
            .expect_err("versions should fail");
        assert!(
            error
                .to_string()
                .contains("npm publishConfig.access must be public")
        );
    }

    #[test]
    fn detects_jj_repo_root() {
        let temp = TempDir::new("jj-root");
        let root = temp.path().join("repo");
        fs::create_dir_all(root.join(".jj")).expect("create .jj dir");
        make_repo_markers(&root);

        assert!(is_repo_root(&root));
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

        assert!(!is_repo_root(&root));
        assert!(find_repo_root(&root).is_err());
    }
}
