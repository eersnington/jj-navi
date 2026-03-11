use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use semver::Version;
use serde_json::{Map, Value};
use thiserror::Error;
use time::OffsetDateTime;
use time::macros::format_description;

const HELP_TEXT: &str = "navi-release

Usage:
  navi-release
  navi-release fragment [patch|minor|major] <summary> [-s <scope>] [--dry-run]
  navi-release prepare <version>
  navi-release validate [version]
  navi-release notes <version>

Commands:
  fragment, new  Create a release fragment. Default command.
  prepare        Roll fragments into changelog and sync versions.
  validate       Verify synced release files.
  notes          Print release notes for a version.
";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bump {
    Patch,
    Minor,
    Major,
}

impl Bump {
    fn as_str(self) -> &'static str {
        match self {
            Self::Patch => "patch",
            Self::Minor => "minor",
            Self::Major => "major",
        }
    }
}

impl TryFrom<&str> for Bump {
    type Error = ToolError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "patch" => Ok(Self::Patch),
            "minor" => Ok(Self::Minor),
            "major" => Ok(Self::Major),
            _ => Err(ToolError::Message(format!("invalid bump: {value}"))),
        }
    }
}

#[derive(Debug)]
struct FragmentCommand {
    bump: Bump,
    scope: String,
    summary: Option<String>,
    dry_run: bool,
}

#[derive(Debug)]
enum Command {
    Fragment(FragmentCommand),
    Prepare { version: String },
    Validate { version: Option<String> },
    Notes { version: String },
    Help,
    Version,
}

#[derive(Debug)]
struct Fragment {
    path: PathBuf,
    scope: String,
    entries: Vec<String>,
}

#[derive(Debug, Error)]
enum ToolError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Semver(#[from] semver::Error),
    #[error(transparent)]
    Time(#[from] time::error::Format),
}

fn main() -> ExitCode {
    match run(env::args_os().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<OsString>) -> Result<(), ToolError> {
    let repo_root = find_repo_root(&env::current_dir()?)?;
    match parse_command(args)? {
        Command::Fragment(command) => run_fragment_command(&repo_root, command),
        Command::Prepare { version } => run_prepare(&repo_root, &version),
        Command::Validate { version } => run_validate(&repo_root, version.as_deref()),
        Command::Notes { version } => run_notes(&repo_root, &version),
        Command::Help => {
            print!("{HELP_TEXT}");
            Ok(())
        }
        Command::Version => {
            println!("navi-release {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}

fn parse_command(args: Vec<OsString>) -> Result<Command, ToolError> {
    let args = args
        .into_iter()
        .map(|value| {
            value
                .into_string()
                .map_err(|_| ToolError::Message("non-utf8 arguments are not supported".to_owned()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    if args.is_empty() {
        return Ok(Command::Fragment(FragmentCommand {
            bump: Bump::Patch,
            scope: "general".to_owned(),
            summary: None,
            dry_run: false,
        }));
    }

    match args[0].as_str() {
        "-h" | "--help" => Ok(Command::Help),
        "-V" | "--version" => Ok(Command::Version),
        "fragment" | "new" => Ok(Command::Fragment(parse_fragment_args(&args[1..])?)),
        "prepare" => {
            let version = args
                .get(1)
                .ok_or_else(|| ToolError::Message("prepare requires <version>".to_owned()))?;
            Ok(Command::Prepare {
                version: version.clone(),
            })
        }
        "validate" => Ok(Command::Validate {
            version: args.get(1).cloned(),
        }),
        "notes" => {
            let version = args
                .get(1)
                .ok_or_else(|| ToolError::Message("notes requires <version>".to_owned()))?;
            Ok(Command::Notes {
                version: version.clone(),
            })
        }
        _ => Ok(Command::Fragment(parse_fragment_args(&args)?)),
    }
}

fn parse_fragment_args(args: &[String]) -> Result<FragmentCommand, ToolError> {
    let mut bump = Bump::Patch;
    let mut scope = "general".to_owned();
    let mut dry_run = false;
    let mut positionals = Vec::new();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "-h" | "--help" => {
                return Ok(FragmentCommand {
                    bump,
                    scope,
                    summary: Some(String::new()),
                    dry_run,
                });
            }
            "-s" | "--scope" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| ToolError::Message("-s/--scope requires a value".to_owned()))?;
                scope = value.clone();
                index += 2;
            }
            "--dry-run" => {
                dry_run = true;
                index += 1;
            }
            value => {
                positionals.push(value.to_owned());
                index += 1;
            }
        }
    }

    if let Some(first) = positionals.first() {
        if let Ok(parsed_bump) = Bump::try_from(first.as_str()) {
            bump = parsed_bump;
            positionals.remove(0);
        }
    }

    let summary = if positionals.is_empty() {
        None
    } else {
        Some(positionals.join(" ").trim().to_owned())
    };

    Ok(FragmentCommand {
        bump,
        scope,
        summary,
        dry_run,
    })
}

fn run_fragment_command(repo_root: &Path, command: FragmentCommand) -> Result<(), ToolError> {
    if command.summary.as_deref() == Some("") {
        print!("{}", HELP_TEXT.replace("navi-release\n\n", ""));
        return Ok(());
    }

    let (bump, scope, entries) = if let Some(summary) = command.summary {
        let summary = summary.trim().to_owned();
        if summary.is_empty() {
            return Err(ToolError::Message("summary required".to_owned()));
        }
        (command.bump, command.scope, vec![summary])
    } else {
        prompt_for_fragment(command.bump, &command.scope)?
    };

    let fragment_path = next_fragment_path(repo_root, &entries[0])?;
    let contents = render_fragment(bump, &scope, &entries);

    if command.dry_run {
        print!("{}\n\n{}", fragment_path.display(), contents);
        return Ok(());
    }

    write_text(&fragment_path, &contents)?;
    println!("{}", fragment_path.display());
    Ok(())
}

fn run_prepare(repo_root: &Path, version: &str) -> Result<(), ToolError> {
    let version = parse_version(version)?;
    let cargo_version = current_cargo_version(repo_root)?;
    let package_version = current_package_version(repo_root)?;

    if cargo_version != package_version {
        return Err(ToolError::Message(format!(
            "version drift before release: Cargo={cargo_version}, npm={package_version}",
        )));
    }

    if version <= cargo_version {
        return Err(ToolError::Message(format!(
            "release version must be greater than current version {cargo_version}",
        )));
    }

    let fragments = load_fragments(repo_root)?;
    if fragments.is_empty() {
        return Err(ToolError::Message(
            "no release fragments found in .release/".to_owned(),
        ));
    }

    let version_text = version.to_string();
    sync_versions(repo_root, &version_text)?;
    prepend_changelog(repo_root, &version_text, &today(), &fragments)?;
    for fragment in &fragments {
        fs::remove_file(&fragment.path)?;
    }

    println!(
        "Prepared release {} from {} fragment(s).",
        version,
        fragments.len()
    );
    Ok(())
}

fn run_validate(repo_root: &Path, version: Option<&str>) -> Result<(), ToolError> {
    let target = match version {
        Some(value) => parse_version(value)?,
        None => current_cargo_version(repo_root)?,
    };

    ensure_versions_match(repo_root, &target)?;
    if !changelog_has_version(repo_root, target.to_string().as_str())? {
        return Err(ToolError::Message(format!(
            "CHANGELOG entry for {target} not found",
        )));
    }

    println!("Validated release files for {target}.");
    Ok(())
}

fn run_notes(repo_root: &Path, version: &str) -> Result<(), ToolError> {
    print!("{}", release_notes(repo_root, version)?);
    Ok(())
}

fn prompt_for_fragment(
    default_bump: Bump,
    default_scope: &str,
) -> Result<(Bump, String, Vec<String>), ToolError> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(ToolError::Message(
            "interactive fragment creation requires a terminal".to_owned(),
        ));
    }

    let bump = loop {
        let input = prompt(&format!(
            "Bump [patch/minor/major] ({default}): ",
            default = default_bump.as_str()
        ))?;
        let trimmed = input.trim();
        if trimmed.is_empty() {
            break default_bump;
        }

        match Bump::try_from(trimmed) {
            Ok(value) => break value,
            Err(_) => eprintln!("invalid bump: {trimmed}"),
        }
    };

    let scope_input = prompt(&format!("Scope ({default_scope}): "))?;
    let scope = if scope_input.trim().is_empty() {
        default_scope.to_owned()
    } else {
        scope_input.trim().to_owned()
    };

    let summary = loop {
        let input = prompt("Summary: ")?;
        let trimmed = input.trim();
        if trimmed.is_empty() {
            continue;
        }
        break trimmed.to_owned();
    };

    let mut entries = vec![summary];
    loop {
        let input = prompt("Additional bullet (blank to finish): ")?;
        let trimmed = input.trim();
        if trimmed.is_empty() {
            break;
        }
        entries.push(trimmed.to_owned());
    }

    Ok((bump, scope, entries))
}

fn prompt(message: &str) -> Result<String, ToolError> {
    print!("{message}");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input)
}

fn find_repo_root(start: &Path) -> Result<PathBuf, ToolError> {
    for candidate in start.ancestors() {
        if candidate.join(".jj").exists() {
            return Ok(candidate.to_path_buf());
        }
    }

    Err(ToolError::Message(
        "navi-release must run inside the jj-navi repo".to_owned(),
    ))
}

fn release_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(".release")
}

fn changelog_path(repo_root: &Path) -> PathBuf {
    repo_root.join("CHANGELOG.md")
}

fn next_fragment_path(repo_root: &Path, summary: &str) -> Result<PathBuf, ToolError> {
    let stamp = OffsetDateTime::now_utc().format(&format_description!(
        "[year][month][day][hour][minute][second]"
    ))?;
    let slug = slugify(summary);
    let path = release_dir(repo_root).join(format!("{stamp}-{slug}.md"));
    if path.exists() {
        return Err(ToolError::Message(format!(
            "fragment already exists: {}",
            path.display()
        )));
    }
    Ok(path)
}

fn slugify(summary: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;

    for character in summary.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            last_dash = false;
        } else if !last_dash && !slug.is_empty() {
            slug.push('-');
            last_dash = true;
        }
        if slug.len() >= 48 {
            break;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        "change".to_owned()
    } else {
        slug
    }
}

fn render_fragment(bump: Bump, scope: &str, entries: &[String]) -> String {
    let mut output = String::new();
    output.push_str("---\n");
    output.push_str(&format!("bump: {}\n", bump.as_str()));
    output.push_str(&format!("scope: {scope}\n"));
    output.push_str("---\n");
    for entry in entries {
        output.push_str("- ");
        output.push_str(entry);
        output.push('\n');
    }
    output
}

fn load_fragments(repo_root: &Path) -> Result<Vec<Fragment>, ToolError> {
    let mut fragments = Vec::new();

    for entry in fs::read_dir(release_dir(repo_root))? {
        let entry = entry?;
        let path = entry.path();
        if path.file_name().and_then(|name| name.to_str()) == Some("README.md") {
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }

        fragments.push(parse_fragment(&path)?);
    }

    fragments.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(fragments)
}

fn parse_fragment(path: &Path) -> Result<Fragment, ToolError> {
    let text = read_text(path)?;
    let Some(frontmatter_end) = text.find("\n---\n") else {
        return Err(ToolError::Message(format!(
            "{}: missing frontmatter end",
            path.display()
        )));
    };

    if !text.starts_with("---\n") {
        return Err(ToolError::Message(format!(
            "{}: missing frontmatter start",
            path.display()
        )));
    }

    let frontmatter_text = &text[4..frontmatter_end];
    let body = text[(frontmatter_end + 5)..].trim();
    let mut saw_bump = false;
    let mut scope = None;

    for line in frontmatter_text
        .lines()
        .filter(|line| !line.trim().is_empty())
    {
        let Some((key, value)) = line.split_once(':') else {
            return Err(ToolError::Message(format!(
                "{}: invalid frontmatter line '{line}'",
                path.display()
            )));
        };

        match key.trim() {
            "bump" => {
                Bump::try_from(value.trim())?;
                saw_bump = true;
            }
            "scope" => scope = Some(value.trim().to_owned()),
            other => {
                return Err(ToolError::Message(format!(
                    "{}: unknown frontmatter key '{other}'",
                    path.display()
                )));
            }
        }
    }

    let entries = body
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.trim_start_matches("- ").to_owned())
        .collect::<Vec<_>>();

    if entries.is_empty() {
        return Err(ToolError::Message(format!(
            "{}: fragment body must include at least one bullet",
            path.display()
        )));
    }

    if !saw_bump {
        return Err(ToolError::Message(format!(
            "{}: missing bump",
            path.display()
        )));
    }

    Ok(Fragment {
        path: path.to_path_buf(),
        scope: scope.unwrap_or_else(|| "general".to_owned()),
        entries,
    })
}

fn today() -> String {
    let today = OffsetDateTime::now_utc().date();
    format!(
        "{:04}-{:02}-{:02}",
        today.year(),
        u8::from(today.month()),
        today.day()
    )
}

fn prepend_changelog(
    repo_root: &Path,
    version: &str,
    date: &str,
    fragments: &[Fragment],
) -> Result<(), ToolError> {
    let current = read_text(&changelog_path(repo_root))?;
    let section = build_changelog_section(version, date, fragments);
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

fn build_changelog_section(version: &str, date: &str, fragments: &[Fragment]) -> String {
    let mut grouped: Vec<(String, Vec<String>)> = Vec::new();

    for fragment in fragments {
        let scope = normalize_scope(&fragment.scope);
        if let Some((_, entries)) = grouped.iter_mut().find(|(name, _)| name == &scope) {
            entries.extend(fragment.entries.iter().cloned());
        } else {
            grouped.push((scope, fragment.entries.clone()));
        }
    }

    let mut lines = vec![format!("## v{version} - {date}"), String::new()];
    for (scope, entries) in grouped {
        lines.push(format!("### {scope}"));
        lines.push(String::new());
        for entry in entries {
            lines.push(format!("- {entry}"));
        }
        lines.push(String::new());
    }

    lines.join("\n").trim_end().to_owned()
}

fn normalize_scope(scope: &str) -> String {
    if scope.is_empty() || scope == "general" {
        return "General".to_owned();
    }

    scope
        .split(['-', '_', ' '])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut characters = part.chars();
            let Some(first) = characters.next() else {
                return String::new();
            };
            first.to_uppercase().collect::<String>() + characters.as_str()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn changelog_has_version(repo_root: &Path, version: &str) -> Result<bool, ToolError> {
    Ok(read_text(&changelog_path(repo_root))?.contains(&format!("## v{version} - ")))
}

fn release_notes(repo_root: &Path, version: &str) -> Result<String, ToolError> {
    let changelog = read_text(&changelog_path(repo_root))?;
    let marker = format!("## v{version} - ");
    let Some(start) = changelog.find(&marker) else {
        return Err(ToolError::Message(format!(
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

fn current_cargo_version(repo_root: &Path) -> Result<Version, ToolError> {
    let cargo_toml = read_text(&repo_root.join("Cargo.toml"))?;
    let version = cargo_toml
        .lines()
        .find_map(|line| {
            line.strip_prefix("version = \"")
                .and_then(|rest| rest.strip_suffix('"'))
        })
        .ok_or_else(|| ToolError::Message("could not read Cargo.toml version".to_owned()))?;
    parse_version(version)
}

fn current_package_version(repo_root: &Path) -> Result<Version, ToolError> {
    let package_json = read_json(&repo_root.join("npm/jj-navi/package.json"))?;
    let version = package_json
        .get("version")
        .and_then(Value::as_str)
        .ok_or_else(|| ToolError::Message("could not read npm package version".to_owned()))?;
    parse_version(version)
}

fn parse_version(value: &str) -> Result<Version, ToolError> {
    Ok(Version::parse(value)?)
}

fn supported_platforms(repo_root: &Path) -> Result<Vec<String>, ToolError> {
    let platforms = read_json(&repo_root.join("npm/scripts/platforms.json"))?;
    let object = platforms
        .as_object()
        .ok_or_else(|| ToolError::Message("platforms.json must be an object".to_owned()))?;
    Ok(object.keys().cloned().collect())
}

fn sync_versions(repo_root: &Path, version: &str) -> Result<(), ToolError> {
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
    write_text(&readme_path, &next_readme)?;

    let package_path = repo_root.join("npm/jj-navi/package.json");
    let mut package_json = read_json(&package_path)?;
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
    write_json(&package_path, &package_json)
}

fn ensure_versions_match(repo_root: &Path, version: &Version) -> Result<(), ToolError> {
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

    let package_json = read_json(&repo_root.join("npm/jj-navi/package.json"))?;
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
    if provenance != Some(true) {
        return Err(ToolError::Message(
            "npm publishConfig.provenance must be true".to_owned(),
        ));
    }

    Ok(())
}

fn read_text(path: &Path) -> Result<String, ToolError> {
    Ok(fs::read_to_string(path)?)
}

fn write_text(path: &Path, value: &str) -> Result<(), ToolError> {
    fs::write(path, value.replace("\r\n", "\n"))?;
    Ok(())
}

fn read_json(path: &Path) -> Result<Value, ToolError> {
    Ok(serde_json::from_str(&read_text(path)?)?)
}

fn write_json(path: &Path, value: &Value) -> Result<(), ToolError> {
    write_text(path, &(serde_json::to_string_pretty(value)? + "\n"))
}
