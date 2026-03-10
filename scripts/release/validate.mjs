#!/usr/bin/env node

import { changelogHasVersion } from "./lib/changelog.mjs";
import { currentCargoVersion, ensureVersionsMatch } from "./lib/version.mjs";

const version = process.argv[2] ?? currentCargoVersion();

try {
  ensureVersionsMatch(version);
  if (!changelogHasVersion(version)) {
    throw new Error(`CHANGELOG entry for ${version} not found`);
  }
} catch (error) {
  console.error(error.message);
  process.exit(1);
}

process.stdout.write(`Validated release files for ${version}.\n`);
