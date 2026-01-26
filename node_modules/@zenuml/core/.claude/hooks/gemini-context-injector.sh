#!/bin/bash
# Gemini Context Injector Hook
# Automatically adds project context files to new Gemini consultation sessions:
# - docs/ai-context/project-structure.md
# - MCP-ASSISTANT-RULES.md
#
# This hook enhances Gemini consultations by automatically including your project's
# structure documentation and assistant rules, ensuring the AI has complete context.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
PROJECT_STRUCTURE_FILE="$PROJECT_ROOT/docs/ai-context/project-structure.md"
MCP_RULES_FILE="$PROJECT_ROOT/MCP-ASSISTANT-RULES.md"
LOG_FILE="$SCRIPT_DIR/../logs/context-injection.log"

# Ensure log directory exists
mkdir -p "$(dirname "$LOG_FILE")"

# Read input from stdin
INPUT_JSON=$(cat)

# Function to log injection events
log_injection_event() {
    local event_type="$1"
    local details="$2"
    local timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    echo "{\"timestamp\": \"$timestamp\", \"event\": \"$event_type\", \"details\": \"$details\"}" >> "$LOG_FILE"
}

# Main logic
main() {
    # Extract tool information from stdin
    local tool_name=$(echo "$INPUT_JSON" | jq -r '.tool_name // ""')
    
    # Only process Gemini consultation requests
    if [[ "$tool_name" != "mcp__gemini__consult_gemini" ]]; then
        echo '{"continue": true}'
        exit 0
    fi
    
    # Extract tool arguments
    local tool_args=$(echo "$INPUT_JSON" | jq -r '.tool_input // "{}"')
    
    # Check if this is a new session (no session_id provided)
    local session_id=$(echo "$tool_args" | jq -r '.session_id // ""' 2>/dev/null || echo "")
    
    if [[ -z "$session_id" || "$session_id" == "null" ]]; then
        log_injection_event "new_session_detected" "preparing_context_injection"
        
        # Check if required files exist
        local missing_files=""
        if [[ ! -f "$PROJECT_STRUCTURE_FILE" ]]; then
            missing_files="$missing_files project_structure.md"
        fi
        if [[ ! -f "$MCP_RULES_FILE" ]]; then
            missing_files="$missing_files MCP-ASSISTANT-RULES.md"
        fi
        
        # If either file is missing, log warning but continue
        if [[ -n "$missing_files" ]]; then
            log_injection_event "warning" "missing_files:$missing_files"
        fi
        
        # If both files are missing, exit early
        if [[ ! -f "$PROJECT_STRUCTURE_FILE" ]] && [[ ! -f "$MCP_RULES_FILE" ]]; then
            echo '{"continue": true}'
            exit 0
        fi
        
        # Extract current attached_files if any
        local current_files=$(echo "$tool_args" | jq -c '.attached_files // []' 2>/dev/null || echo "[]")
        
        # Check if files are already included
        local has_project_structure=$(echo "$current_files" | jq -e ".[] | select(. == \"$PROJECT_STRUCTURE_FILE\")" > /dev/null 2>&1 && echo "true" || echo "false")
        local has_mcp_rules=$(echo "$current_files" | jq -e ".[] | select(. == \"$MCP_RULES_FILE\")" > /dev/null 2>&1 && echo "true" || echo "false")
        
        # If both files exist and are already included, skip
        if [[ -f "$PROJECT_STRUCTURE_FILE" ]] && [[ "$has_project_structure" == "true" ]] && \
           [[ -f "$MCP_RULES_FILE" ]] && [[ "$has_mcp_rules" == "true" ]]; then
            log_injection_event "skipped" "all_required_files_already_included"
            echo '{"continue": true}'
            exit 0
        fi
        
        # Add missing files to attached_files
        local modified_args="$tool_args"
        local files_added=""
        
        if [[ -f "$PROJECT_STRUCTURE_FILE" ]] && [[ "$has_project_structure" == "false" ]]; then
            modified_args=$(echo "$modified_args" | jq --arg file "$PROJECT_STRUCTURE_FILE" '
                .attached_files = ((.attached_files // []) + [$file])
            ' 2>/dev/null)
            files_added="$files_added project_structure.md"
        fi
        
        if [[ -f "$MCP_RULES_FILE" ]] && [[ "$has_mcp_rules" == "false" ]]; then
            modified_args=$(echo "$modified_args" | jq --arg file "$MCP_RULES_FILE" '
                .attached_files = ((.attached_files // []) + [$file])
            ' 2>/dev/null)
            files_added="$files_added MCP-ASSISTANT-RULES.md"
        fi
        
        if [[ -n "$modified_args" ]] && [[ "$modified_args" != "$tool_args" ]]; then
            log_injection_event "context_injected" "added_files:$files_added"
            
            # Update the input JSON with modified tool_input
            local output_json=$(echo "$INPUT_JSON" | jq --argjson new_args "$modified_args" '.tool_input = $new_args')
            
            # Return the modified input to stdout
            echo "$output_json"
            exit 0
        else
            log_injection_event "error" "failed_to_modify_arguments"
            # Continue without modification on error
            echo '{"continue": true}'
            exit 0
        fi
    else
        log_injection_event "existing_session" "session_id:$session_id"
        # For existing sessions, continue without modification
        echo '{"continue": true}'
        exit 0
    fi
}

# Run main function
main