# Kiro Agent Hooks

This directory contains custom hooks that automatically execute when specific events occur in Kiro.

## Session Sound Notification Hook

**File:** `session-sound-notification.json` and `session-sound-notification.js`

**Purpose:** Plays system sounds to notify you when:

- An AI session completes
- User input is required to continue
- An error occurs

**Sounds Used (macOS):**

- **Session Complete:** Glass.aiff - A pleasant chime when tasks finish
- **Input Required:** Sosumi.aiff - An attention-getting sound when you need to respond
- **Error:** Basso.aiff - A distinctive sound for errors
- **Default:** Ping.aiff - Fallback sound

**Configuration:**
The hook is enabled by default and will auto-execute. You can disable it by setting `"enabled": false` in the JSON configuration file.

**Platform Notes:**

- Currently optimized for macOS using the `afplay` command
- Uses built-in system sounds located in `/System/Library/Sounds/`
- For other platforms, the sound files and playback command would need to be adjusted

## Managing Hooks

You can:

1. View current hooks in the Kiro Explorer under "Agent Hooks"
2. Use Command Palette → "Open Kiro Hook UI" to create new hooks
3. Edit hook files directly in this directory
4. Enable/disable hooks by modifying the `enabled` property
