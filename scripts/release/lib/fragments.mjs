import { readdirSync, rmSync } from "fs";
import { readText, repoPath } from "./files.mjs";

const allowedBumps = new Set(["patch", "minor", "major"]);

function parseFrontmatter(text, path) {
  if (!text.startsWith("---\n")) {
    throw new Error(`${path}: missing frontmatter start`);
  }

  const endIndex = text.indexOf("\n---\n", 4);
  if (endIndex === -1) {
    throw new Error(`${path}: missing frontmatter end`);
  }

  const frontmatterText = text.slice(4, endIndex);
  const body = text.slice(endIndex + 5).trim();
  const frontmatter = Object.fromEntries(
    frontmatterText
      .split("\n")
      .filter(Boolean)
      .map((line) => {
        const separator = line.indexOf(":");
        if (separator === -1) {
          throw new Error(`${path}: invalid frontmatter line '${line}'`);
        }

        const key = line.slice(0, separator).trim();
        const value = line.slice(separator + 1).trim();
        return [key, value];
      }),
  );

  return { body, frontmatter };
}

export function releaseDirectory() {
  return repoPath(".release");
}

export function fragmentPaths() {
  return readdirSync(releaseDirectory())
    .filter((entry) => entry.endsWith(".md") && entry !== "README.md")
    .sort()
    .map((entry) => repoPath(".release", entry));
}

export function loadFragments() {
  return fragmentPaths().map((path) => {
    const { body, frontmatter } = parseFrontmatter(readText(path), path);
    const bump = frontmatter.bump;
    if (!allowedBumps.has(bump)) {
      throw new Error(`${path}: invalid bump '${bump}'`);
    }

    const entries = body
      .split("\n")
      .map((line) => line.trim())
      .filter(Boolean)
      .map((line) => line.replace(/^-\s*/, ""));

    if (entries.length === 0) {
      throw new Error(`${path}: fragment body must include at least one bullet`);
    }

    return {
      path,
      bump,
      scope: frontmatter.scope || "general",
      entries,
    };
  });
}

export function deleteFragments(paths) {
  for (const path of paths) {
    rmSync(path);
  }
}
