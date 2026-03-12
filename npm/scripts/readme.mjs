import { mkdirSync, readFileSync, writeFileSync } from "fs";
import { dirname, join, resolve } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "..", "..");
const repositoryUrl = "https://github.com/eersnington/jj-navi";
const wrapperPackageUrl = "https://www.npmjs.com/package/jj-navi";
const rootReadmePath = join(repoRoot, "README.md");
const wrapperReadmePath = join(repoRoot, "npm", "jj-navi", "README.md");

export function syncWrapperReadme(outputPath = wrapperReadmePath) {
  const targetPath = resolve(outputPath);
  writeText(targetPath, readRootReadme());
  return targetPath;
}

export function writePlatformReadme(platform, config, outputDir) {
  const targetPath = join(resolve(outputDir), "README.md");
  writeText(targetPath, renderPlatformReadme(platform, config));
  return targetPath;
}

function readRootReadme() {
  return readFileSync(rootReadmePath, "utf8");
}

function renderPlatformReadme(platform, config) {
  const platformLabel = renderPlatformLabel(config);
  return `# jj-navi-${platform}\n\nPrebuilt \`navi\` binary for ${platformLabel}.\n\nMost users should install [\`jj-navi\`](${wrapperPackageUrl}) instead.\n\nThis package is published as an implementation detail of the main \`jj-navi\` npm package and provides the executable used during install.\n\nProject docs live in the main repo: ${repositoryUrl}\n`;
}

function renderPlatformLabel(config) {
  const osLabel = config.os === "darwin" ? "macOS" : config.os === "linux" ? "Linux" : config.os;
  const parts = [osLabel, config.cpu];
  if (config.libc) {
    parts.push(`(${config.libc})`);
  }
  return parts.join(" ");
}

function writeText(path, text) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, text.replace(/\r\n/g, "\n"));
}
