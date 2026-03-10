#!/usr/bin/env node

import { execSync } from "child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const npmDir = join(__dirname, "..");
const platforms = JSON.parse(readFileSync(join(__dirname, "platforms.json"), "utf8"));
const [, , version] = process.argv;

if (!version) {
  console.error("Usage: node update-versions.mjs <version>");
  process.exit(1);
}

try {
  execSync(`npx --yes semver "${version}"`, { stdio: "pipe" });
} catch {
  console.error(`Invalid semver: ${version}`);
  process.exit(1);
}

const packagePath = join(npmDir, "jj-navi", "package.json");
const packageJson = JSON.parse(readFileSync(packagePath, "utf8"));
packageJson.version = version;
packageJson.optionalDependencies = Object.fromEntries(
  Object.keys(platforms).map((platform) => [`jj-navi-${platform}`, version])
);

writeFileSync(packagePath, JSON.stringify(packageJson, null, 2) + "\n");

for (const platform of Object.keys(platforms)) {
  const platformDir = join(npmDir, platform);
  if (!existsSync(platformDir)) {
    mkdirSync(platformDir, { recursive: true });
  }

  execSync(`node ${join(__dirname, "generate-platform-package.mjs")} ${platform} ${version}`, {
    stdio: "inherit"
  });
}
