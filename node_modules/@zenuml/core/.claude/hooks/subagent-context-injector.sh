#!/bin/bash
# Sub-Agent Context Auto-Loader
# Automatically enhances Task tool prompts with essential project context
#
# This hook ensures every sub-agent spawned via the Task tool automatically
# receives core project documentation, eliminating the need to manually
# include context in each Task prompt.
#
# IMPLEMENTATION OVERVIEW:
# - Registered as a PreToolUse hook in .claude/settings.json
# - Intercepts all Task tool calls before execution
# - Injects references to CLAUDE.md, project-structure.md, and docs-overview.md
# - Preserves original prompt by prepending context, not replacing
# - Passes through non-Task tools unchanged with {"continue": true}


set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Read input from stdin
INPUT_JSON=$(cat)

# Extract tool information
tool_name=$(echo "$INPUT_JSON" | jq -r '.tool_name // ""')

# Only process Task tool calls - pass through all other tools unchanged
if [[ "$tool_name" != "Task" ]]; then
    echo '{"continue": true}'
    exit 0
fi

# Extract current prompt from the Task tool input
current_prompt=$(echo "$INPUT_JSON" | jq -r '.tool_input.prompt // ""')

# Build context injection header with project documentation references
# These files are automatically available to all sub-agents via @ references
context_injection="## Auto-Loaded Project Context

This sub-agent has automatic access to the following project documentation:
- @$PROJECT_ROOT/docs/CLAUDE.md (Project overview, coding standards, and AI instructions)
- @$PROJECT_ROOT/docs/ai-context/project-structure.md (Complete file tree and tech stack)
- @$PROJECT_ROOT/docs/ai-context/docs-overview.md (Documentation architecture)

These files provide essential context about the project structure, 
conventions, and development patterns. Reference them as needed for your task.

---

## Your Task

"

# Combine context injection with original prompt
# The context is prepended to preserve the original task instructions
modified_prompt="${context_injection}${current_prompt}"

# Update the input JSON with the modified prompt
# This maintains all other tool input fields unchanged
output_json=$(echo "$INPUT_JSON" | jq --arg new_prompt "$modified_prompt" '.tool_input.prompt = $new_prompt')

# Output the modified JSON for Claude Code to process
# The Task tool will receive the enhanced prompt with context
echo "$output_json"