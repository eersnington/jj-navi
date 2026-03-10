import { readText, repoPath, writeText } from "./files.mjs";

const changelogPath = repoPath("CHANGELOG.md");

function normalizeScope(scope) {
  if (!scope || scope === "general") {
    return "General";
  }

  return scope
    .split(/[-_\s]+/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function buildSection(version, date, fragments) {
  const grouped = new Map();
  for (const fragment of fragments) {
    const scope = normalizeScope(fragment.scope);
    if (!grouped.has(scope)) {
      grouped.set(scope, []);
    }

    grouped.get(scope).push(...fragment.entries);
  }

  const lines = [`## v${version} - ${date}`, ""];
  for (const [scope, entries] of grouped) {
    lines.push(`### ${scope}`, "");
    for (const entry of entries) {
      lines.push(`- ${entry}`);
    }
    lines.push("");
  }

  return lines.join("\n").trimEnd();
}

export function changelogHasVersion(version) {
  return readText(changelogPath).includes(`## v${version} - `);
}

export function prependChangelog(version, date, fragments) {
  const current = readText(changelogPath).trimEnd();
  const section = buildSection(version, date, fragments);
  const firstEntryIndex = current.indexOf("\n## ");
  const header = firstEntryIndex === -1 ? current : current.slice(0, firstEntryIndex).trimEnd();
  const rest = firstEntryIndex === -1 ? "" : current.slice(firstEntryIndex).trimStart();
  const next = [header, "", section, rest ? `\n${rest}` : ""]
    .join("\n")
    .replace(/\n{3,}/g, "\n\n")
    .trimEnd() + "\n";

  writeText(changelogPath, next);
}

export function releaseNotes(version) {
  const changelog = readText(changelogPath);
  const marker = `## v${version} - `;
  const start = changelog.indexOf(marker);
  if (start === -1) {
    throw new Error(`CHANGELOG entry for ${version} not found`);
  }

  const rest = changelog.slice(start);
  const nextSection = rest.indexOf("\n## ", marker.length);
  const entry = nextSection === -1 ? rest : rest.slice(0, nextSection);
  return entry.trimEnd() + "\n";
}
