# Running Cypress Tests Locally

This document explains how to run Cypress tests locally on your machine, with a focus on ensuring color profiles match those used in GitHub Actions for consistent visual testing results.

## Running Tests

You can run Cypress tests using the following commands:

- Run all tests headlessly:

```bash
pnpm cy
```

- Run a specific test file:

```bash
pnpm cypress run --spec "cypress/e2e/smoke.spec.js"
```

## Color Profile Configuration

### Why Color Profiles Matter

Visual regression tests in Cypress compare screenshots pixel-by-pixel. Different color profiles can cause the same UI to render with slightly different colors, leading to false test failures. To match the GitHub Actions environment, we use the sRGB color profile.

### Automatic Configuration

Our Cypress configuration automatically forces the sRGB color profile when running tests. This is configured in `cypress/plugins/index.js` with the following code:

```javascript
on("before:browser:launch", (browser = {}, launchOptions) => {
  if (browser.family === "chromium" && browser.name !== "electron") {
    launchOptions.args.push("--force-color-profile=srgb");
  }
});
```

### Configuring macOS Color Profile

For the most consistent results, you should also configure your macOS system to use the sRGB color profile:

1. Open System Preferences (or System Settings on newer macOS versions)
2. Go to "Displays"
3. Select the "Color" tab
4. Click "Calibrate" to open the Display Calibrator Assistant
5. Follow the wizard and when prompted for a color profile, select "sRGB"
6. Complete the calibration and save the profile

Alternatively, you can directly select an existing sRGB profile:

1. Open System Preferences > Displays > Color
2. From the list of color profiles, select "sRGB IEC61966-2.1" or a similar sRGB profile
3. Click "Apply"

## Troubleshooting Visual Test Failures

If you encounter visual test failures despite using the sRGB profile:

1. Check that your display brightness is set to a consistent level
2. Ensure you're using the same browser version as GitHub Actions (Chrome)
3. Verify that your OS theme/appearance settings match the CI environment (light mode is recommended)
4. If necessary, adjust the threshold in the image snapshot configuration:

```javascript
cy.document().toMatchImageSnapshot({
  imageConfig: { threshold: 0.012 }, // Increase this value if needed
  capture: "viewport",
});
```

## Additional Resources

- [Cypress Image Snapshot Documentation](https://github.com/jaredpalmer/cypress-image-snapshot)
- [Chrome Command Line Switches](https://peter.sh/experiments/chromium-command-line-switches/)
- [macOS Color Management Guide](https://support.apple.com/guide/mac-help/calibrate-your-display-mchlp2920/mac)
