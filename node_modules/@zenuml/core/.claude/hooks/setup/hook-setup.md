## Description

This command uses specialized agents to verify, configure, and test your Claude Code hooks installation. It ensures everything is properly set up and working correctly.

## Process

### Phase 1: Multi-Agent Setup Verification

The command spawns specialized agents to handle different aspects:

1. **Installation Agent**
   - Verifies `.claude/hooks/` directory exists
   - Checks all hook scripts are present
   - Ensures executable permissions (`chmod +x`)
   - Validates sound files and configuration files

2. **Configuration Agent**
   - Locates Claude Code settings.json for your OS
   - Verifies hook configurations in settings
   - Checks WORKSPACE environment variable
   - Validates MCP server configurations

3. **Documentation Agent**
   - Ensures project structure documentation exists
   - Verifies paths used by context injector
   - Checks log directory setup

### Phase 2: Comprehensive Testing

After setup verification, the main agent runs comprehensive tests:

1. **Security Scanner Tests**
   - API key detection patterns
   - Password and secret detection
   - Whitelist functionality
   - Command injection protection
   - File scanning capabilities

2. **Context Injector Tests**
   - New session detection
   - File attachment logic
   - Path resolution
   - Error handling scenarios

3. **Notification Tests**
   - Audio playback on current platform
   - Fallback mechanism verification
   - Both input and complete sounds

## Expected Output

```
Starting multi-agent hook setup verification...

[Installation Agent]
✓ Hooks directory found: .claude/hooks/
✓ All hook scripts present and executable
✓ Configuration files valid
✓ Sound files present

[Configuration Agent]
✓ Project settings found: .claude/settings.json
✓ Hook configurations verified
✓ WORKSPACE environment variable set correctly

[Documentation Agent]
✓ Project structure documentation found
✓ Log directories configured

Running comprehensive tests...

[Security Scanner]
✓ Detected: sk-1234567890abcdef (API key)
✓ Detected: password=mysecret123
✓ Allowed: YOUR_API_KEY (whitelisted)
✓ Blocked: $(malicious) (injection attempt)

[Context Injector]
✓ New session handling correct
✓ File attachment working
✓ Error handling graceful

[Notifications]
✓ Audio playback successful
✓ Platform: darwin (macOS)

All hooks configured and tested successfully!
```

## Troubleshooting

The command provides specific guidance for any issues found:
- Missing files or permissions
- Configuration problems
- Test failures with debugging steps
- Platform-specific audio issues