import { mkdirSync, readFileSync, writeFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(here, "..", "..", "..");

export function repoPath(...parts) {
  return join(repoRoot, ...parts);
}

export function ensureDir(path) {
  mkdirSync(path, { recursive: true });
}

export function readText(path) {
  return readFileSync(path, "utf8");
}

export function writeText(path, value) {
  writeFileSync(path, value.replace(/\r\n/g, "\n"));
}

export function readJson(path) {
  return JSON.parse(readText(path));
}

export function writeJson(path, value) {
  writeText(path, JSON.stringify(value, null, 2) + "\n");
}
