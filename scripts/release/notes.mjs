#!/usr/bin/env node

import { releaseNotes } from "./lib/changelog.mjs";

const version = process.argv[2];
if (!version) {
  console.error("Usage: node scripts/release/notes.mjs <version>");
  process.exit(1);
}

process.stdout.write(releaseNotes(version));
