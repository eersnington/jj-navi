#!/usr/bin/env node

import { mkdirSync, readFileSync, writeFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const platforms = JSON.parse(readFileSync(join(__dirname, "platforms.json"), "utf8"));
const [, , platform, version] = process.argv;

if (!platform || !version) {
  console.error("Usage: node generate-platform-package.mjs <platform> <version>");
  process.exit(1);
}

const config = platforms[platform];
if (!config) {
  console.error(`Unknown platform: ${platform}`);
  process.exit(1);
}

const packageJson = {
  name: `jj-navi-${platform}`,
  version,
  description: `navi binary for ${config.os} ${config.cpu}${config.libc ? ` (${config.libc})` : ""}`,
  license: "MIT",
  os: [config.os],
  cpu: [config.cpu],
  preferUnplugged: true,
  publishConfig: {
    access: "public",
    provenance: true
  }
};

if (config.libc) {
  packageJson.libc = [config.libc];
}

const platformDir = join(__dirname, "..", platform);
mkdirSync(platformDir, { recursive: true });

writeFileSync(
  join(platformDir, "package.json"),
  JSON.stringify(packageJson, null, 2) + "\n"
);
