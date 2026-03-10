#!/usr/bin/env node

import { deleteFragments, loadFragments } from "./lib/fragments.mjs";
import { prependChangelog } from "./lib/changelog.mjs";
import {
  compareVersions,
  currentCargoVersion,
  currentPackageVersion,
  syncVersions,
} from "./lib/version.mjs";

const version = process.argv[2];
if (!version) {
  console.error("Usage: node scripts/release/plan.mjs <version>");
  process.exit(1);
}

const cargoVersion = currentCargoVersion();
const packageVersion = currentPackageVersion();
if (cargoVersion !== packageVersion) {
  console.error(`Version drift before release: Cargo=${cargoVersion}, npm=${packageVersion}`);
  process.exit(1);
}

if (compareVersions(version, cargoVersion) <= 0) {
  console.error(`Release version must be greater than current version ${cargoVersion}`);
  process.exit(1);
}

const fragments = loadFragments();
if (fragments.length === 0) {
  console.error("No release fragments found in .release/");
  process.exit(1);
}

syncVersions(version);
prependChangelog(version, new Date().toISOString().slice(0, 10), fragments);
deleteFragments(fragments.map((fragment) => fragment.path));

process.stdout.write(`Prepared release ${version} from ${fragments.length} fragment(s).\n`);
