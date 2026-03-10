#!/usr/bin/env node

import { existsSync } from "fs";
import { basename, join } from "path";
import { ensureDir, repoPath, writeText } from "./lib/files.mjs";

const args = process.argv.slice(2);
let scope = "general";
let dryRun = false;
const positionals = [];

for (let index = 0; index < args.length; index += 1) {
  const value = args[index];
  if (value === "-s" || value === "--scope") {
    scope = args[index + 1] ?? "general";
    index += 1;
    continue;
  }

  if (value === "--dry-run") {
    dryRun = true;
    continue;
  }

  positionals.push(value);
}

if (positionals.length === 0) {
  console.error(
    "Usage: ./scripts/release/new [patch|minor|major] <summary> [-s <scope>] [--dry-run]",
  );
  process.exit(1);
}

const allowedBumps = new Set(["patch", "minor", "major"]);
const hasExplicitBump = allowedBumps.has(positionals[0]);
const bump = hasExplicitBump ? positionals[0] : "patch";
const summaryParts = hasExplicitBump ? positionals.slice(1) : positionals;

if (summaryParts.length === 0) {
  console.error("Summary required");
  process.exit(1);
}

const summary = summaryParts.join(" ").trim();
const slug = summary
  .toLowerCase()
  .replace(/[^a-z0-9]+/g, "-")
  .replace(/^-+|-+$/g, "")
  .slice(0, 48);

const now = new Date();
const stamp = [
  now.getUTCFullYear(),
  String(now.getUTCMonth() + 1).padStart(2, "0"),
  String(now.getUTCDate()).padStart(2, "0"),
  String(now.getUTCHours()).padStart(2, "0"),
  String(now.getUTCMinutes()).padStart(2, "0"),
  String(now.getUTCSeconds()).padStart(2, "0"),
].join("");

const fragmentsDir = repoPath(".release");
ensureDir(fragmentsDir);

const filePath = join(fragmentsDir, `${stamp}-${slug || "change"}.md`);
if (existsSync(filePath)) {
  console.error(`Fragment already exists: ${basename(filePath)}`);
  process.exit(1);
}

const body = ["---", `bump: ${bump}`, `scope: ${scope}`, "---", `- ${summary}`, ""].join("\n");

if (dryRun) {
  process.stdout.write(`${filePath}\n\n${body}`);
  process.exit(0);
}

writeText(filePath, body);
process.stdout.write(`${filePath}\n`);
