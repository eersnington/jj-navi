import { readJson, readText, repoPath, writeJson, writeText } from "./files.mjs";

const semverPattern = /^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?$/;

export function parseVersion(version) {
  const match = semverPattern.exec(version);
  if (!match) {
    throw new Error(`Invalid semver: ${version}`);
  }

  const prerelease = match[4]
    ? match[4].split(".").map((identifier) =>
        /^\d+$/.test(identifier) ? Number(identifier) : identifier,
      )
    : [];

  return {
    raw: version,
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
    prerelease,
  };
}

function compareIdentifier(left, right) {
  const leftIsNumber = typeof left === "number";
  const rightIsNumber = typeof right === "number";

  if (leftIsNumber && rightIsNumber) {
    return left - right;
  }

  if (leftIsNumber) {
    return -1;
  }

  if (rightIsNumber) {
    return 1;
  }

  return left.localeCompare(right);
}

export function compareVersions(leftRaw, rightRaw) {
  const left = parseVersion(leftRaw);
  const right = parseVersion(rightRaw);

  for (const key of ["major", "minor", "patch"]) {
    const diff = left[key] - right[key];
    if (diff !== 0) {
      return diff;
    }
  }

  if (left.prerelease.length === 0 && right.prerelease.length === 0) {
    return 0;
  }

  if (left.prerelease.length === 0) {
    return 1;
  }

  if (right.prerelease.length === 0) {
    return -1;
  }

  const length = Math.max(left.prerelease.length, right.prerelease.length);
  for (let index = 0; index < length; index += 1) {
    const leftIdentifier = left.prerelease[index];
    const rightIdentifier = right.prerelease[index];

    if (leftIdentifier === undefined) {
      return -1;
    }

    if (rightIdentifier === undefined) {
      return 1;
    }

    const diff = compareIdentifier(leftIdentifier, rightIdentifier);
    if (diff !== 0) {
      return diff;
    }
  }

  return 0;
}

export function isPrerelease(version) {
  return parseVersion(version).prerelease.length > 0;
}

export function currentCargoVersion() {
  const cargoToml = readText(repoPath("Cargo.toml"));
  const match = cargoToml.match(/^version = "([^"]+)"/m);
  if (!match) {
    throw new Error("Could not read Cargo.toml version");
  }

  return match[1];
}

export function currentPackageVersion() {
  return readJson(repoPath("npm", "jj-navi", "package.json")).version;
}

export function supportedPlatforms() {
  return Object.keys(readJson(repoPath("npm", "scripts", "platforms.json")));
}

export function expectedOptionalDependencies(version) {
  return Object.fromEntries(
    supportedPlatforms().map((platform) => [`jj-navi-${platform}`, version]),
  );
}

export function syncVersions(version) {
  parseVersion(version);

  const cargoPath = repoPath("Cargo.toml");
  const cargoToml = readText(cargoPath);
  writeText(
    cargoPath,
    cargoToml.replace(/^version = ".*"/m, `version = "${version}"`),
  );

  const readmePath = repoPath("README.md");
  const readme = readText(readmePath);
  writeText(
    readmePath,
    readme.replace(
      /cargo install jj-navi --version [^\n]+/,
      `cargo install jj-navi --version ${version}`,
    ),
  );

  const packagePath = repoPath("npm", "jj-navi", "package.json");
  const packageJson = readJson(packagePath);
  packageJson.version = version;
  packageJson.optionalDependencies = expectedOptionalDependencies(version);
  packageJson.publishConfig = {
    access: "public",
    provenance: true,
  };
  writeJson(packagePath, packageJson);
}

export function ensureVersionsMatch(version) {
  parseVersion(version);

  const cargoVersion = currentCargoVersion();
  const packageVersion = currentPackageVersion();
  if (cargoVersion !== version) {
    throw new Error(`Cargo.toml version mismatch: expected ${version}, got ${cargoVersion}`);
  }

  if (packageVersion !== version) {
    throw new Error(
      `npm/jj-navi/package.json version mismatch: expected ${version}, got ${packageVersion}`,
    );
  }

  const readme = readText(repoPath("README.md"));
  const installLine = readme.match(/cargo install jj-navi --version ([^\n]+)/);
  if (!installLine) {
    throw new Error("README install version not found");
  }

  if (installLine[1].trim() !== version) {
    throw new Error(`README version mismatch: expected ${version}, got ${installLine[1].trim()}`);
  }

  const packageJson = readJson(repoPath("npm", "jj-navi", "package.json"));
  const expectedDependencies = expectedOptionalDependencies(version);
  const actualDependencies = packageJson.optionalDependencies ?? {};
  const expectedKeys = Object.keys(expectedDependencies);
  const actualKeys = Object.keys(actualDependencies);

  if (
    expectedKeys.length !== actualKeys.length ||
    expectedKeys.some((key) => actualDependencies[key] !== expectedDependencies[key])
  ) {
    throw new Error("npm optionalDependencies do not match supported platforms");
  }

  if (packageJson.publishConfig?.provenance !== true) {
    throw new Error("npm publishConfig.provenance must be true");
  }
}
