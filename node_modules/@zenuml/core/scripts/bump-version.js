#!/usr/bin/env node
const fs = require("fs");
const { execSync } = require("child_process");
const path = require("path");

const getCurrentVersion = () => {
  try {
    const pkg = require("../package.json");
    const npmVersion = execSync(`npm view ${pkg.name} version`, {
      stdio: ["pipe", "pipe", "pipe"],
    })
      .toString()
      .trim();
    console.log(`Latest version on npm: ${npmVersion}`);
    return npmVersion;
  } catch (error) {
    console.error(
      "Failed to get npm version. Please ensure you have npm registry access.",
    );
    console.error("Error:", error.message);
    process.exit(1);
  }
};

const getCommitMessages = (currentVersion) => {
  try {
    // Try to fetch tags first
    try {
      execSync("git fetch --tags");
    } catch (e) {
      console.log("Warning: Could not fetch tags:", e.message);
    }

    // Get commits since the last version tag
    const messages = execSync(
      `git log v${currentVersion}..HEAD --pretty=format:%s%n%b`,
    ).toString();
    console.log("\nCommit messages since", `v${currentVersion}:`);
    console.log(messages || "(no commits)");
    return messages;
  } catch (e) {
    console.log(
      "Warning: Could not get commits since last version, falling back to recent commits",
    );
    // Fallback to recent commits if tag not found
    const messages = execSync("git log -10 --pretty=format:%s%n%b").toString();
    console.log("\nRecent commit messages:");
    console.log(messages || "(no commits)");
    return messages;
  }
};

const determineVersionBump = (messages) => {
  const lines = messages.split("\n").filter(Boolean);
  if (
    lines.some((msg) => msg.includes("BREAKING CHANGE") || msg.includes("!:"))
  ) {
    return "major";
  }
  if (lines.some((msg) => msg.toLowerCase().startsWith("feat"))) {
    return "minor";
  }
  return "patch";
};

const getNewVersion = (currentVersion, bump) => {
  const [major, minor, patch] = currentVersion.split(".").map(Number);
  switch (bump) {
    case "major":
      return `${major + 1}.0.0`;
    case "minor":
      return `${major}.${minor + 1}.0`;
    case "patch":
      return `${major}.${minor}.${patch + 1}`;
    default:
      throw new Error(`Invalid version bump type: ${bump}`);
  }
};

const run = async () => {
  const dryRun = process.argv.includes("--dry-run");
  if (dryRun) {
    console.log("DRY RUN: No changes will be made\n");
  }

  const currentVersion = getCurrentVersion();
  console.log(`Current version: ${currentVersion}`);

  const messages = getCommitMessages(currentVersion);
  const versionBump = determineVersionBump(messages);
  console.log(`\nDetermined version bump: ${versionBump}`);

  const newVersion = getNewVersion(currentVersion, versionBump);
  console.log(`New version will be: ${newVersion}`);

  if (!dryRun) {
    // Update package.json
    const pkg = require("../package.json");
    pkg.version = newVersion;
    fs.writeFileSync(
      path.join(__dirname, "../package.json"),
      JSON.stringify(pkg, null, 2) + "\n",
    );

    // Set output for GitHub Actions
    if (process.env.GITHUB_ACTIONS) {
      fs.appendFileSync(process.env.GITHUB_OUTPUT, `version=${newVersion}\n`);
    }
  } else {
    console.log("\nDRY RUN: No changes were made");
  }
};

run().catch((err) => {
  console.error("Error:", err.message);
  process.exit(1);
});
