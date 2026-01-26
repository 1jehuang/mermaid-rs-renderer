#!/usr/bin/env node

/**
 * Script to update Cypress image snapshots
 *
 * This script:
 * 1. Removes all .diff.png files
 * 2. Replaces .png files with their .actual.png counterparts
 * 3. Removes all .actual.png files
 */

const fs = require("fs");
const path = require("path");
const { promisify } = require("util");
const { exec } = require("child_process");

const execAsync = promisify(exec);

// Path to the image snapshots directory
const SNAPSHOTS_DIR = path.resolve(
  __dirname,
  "../cypress/e2e/__image_snapshots__",
);

async function updateSnapshots() {
  try {
    console.log("Starting snapshot update process...");

    // Check if the snapshots directory exists
    if (!fs.existsSync(SNAPSHOTS_DIR)) {
      console.error(`Error: Snapshots directory not found at ${SNAPSHOTS_DIR}`);
      process.exit(1);
    }

    console.log("1. Removing .diff.png files...");
    await execAsync(`find "${SNAPSHOTS_DIR}" -name "*.diff.png" -delete`);

    console.log(
      "2. Replacing .png files with their .actual.png counterparts...",
    );
    // Find all .actual.png files and replace their corresponding .png files
    const files = await execAsync(
      `find "${SNAPSHOTS_DIR}" -name "*.actual.png"`,
    );
    const actualFiles = files.stdout.trim().split("\n").filter(Boolean);

    for (const actualFile of actualFiles) {
      const targetFile = actualFile.replace(".actual.png", ".png");
      console.log(
        `   Replacing ${path.basename(targetFile)} with ${path.basename(
          actualFile,
        )}`,
      );
      fs.copyFileSync(actualFile, targetFile);
    }

    console.log("3. Removing .actual.png files...");
    await execAsync(`find "${SNAPSHOTS_DIR}" -name "*.actual.png" -delete`);

    console.log("Snapshot update completed successfully!");
    console.log(
      "Remember to commit and push these changes to update the reference snapshots.",
    );
  } catch (error) {
    console.error("Error updating snapshots:", error);
    process.exit(1);
  }
}

updateSnapshots();
