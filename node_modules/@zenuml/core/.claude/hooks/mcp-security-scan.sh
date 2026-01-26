#!/bin/bash
# MCP Security Scanner Hook
# Scans MCP requests for sensitive data before sending to external services
#
# This hook protects against accidental exposure of secrets, API keys, and other
# sensitive information when using MCP servers like Gemini or Context7.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PATTERNS_FILE="$SCRIPT_DIR/config/sensitive-patterns.json"
LOG_FILE="$SCRIPT_DIR/../logs/security-scan.log"

# Ensure log directory exists
mkdir -p "$(dirname "$LOG_FILE")"

# Read input from stdin
INPUT_JSON=$(cat)

# Function to log security events
log_security_event() {
    local event_type="$1"
    local details="$2"
    local tool_name="${3:-unknown}"
    local timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    echo "{\"timestamp\": \"$timestamp\", \"tool\": \"$tool_name\", \"event\": \"$event_type\", \"details\": \"$details\"}" >> "$LOG_FILE"
}

# Function to check if content matches sensitive patterns
check_sensitive_content() {
    local content="$1"
    local pattern_type="$2"
    
    # Get patterns from JSON config
    local patterns=$(jq -r ".patterns.$pattern_type[]" "$PATTERNS_FILE" 2>/dev/null || echo "")
    
    for pattern in $patterns; do
        if echo "$content" | grep -qiE "$pattern"; then
            # Check whitelist
            local whitelisted=false
            local whitelist_patterns=$(jq -r '.whitelist.allowed_mentions[]' "$PATTERNS_FILE" 2>/dev/null || echo "")
            
            for whitelist in $whitelist_patterns; do
                if echo "$content" | grep -qF "$whitelist"; then
                    whitelisted=true
                    break
                fi
            done
            
            if [[ "$whitelisted" == "false" ]]; then
                return 0  # Found sensitive data
            fi
        fi
    done
    
    return 1  # No sensitive data found
}

# Function to scan file content
scan_file_content() {
    local file_path="$1"
    
    # Check if file name itself is sensitive
    local filename=$(basename "$file_path")
    if check_sensitive_content "$filename" "sensitive_files"; then
        return 0  # Sensitive file
    fi
    
    # Don't scan files that don't exist or are too large
    if [[ ! -f "$file_path" ]] || [[ $(stat -f%z "$file_path" 2>/dev/null || stat -c%s "$file_path" 2>/dev/null || echo "999999999") -gt 1048576 ]]; then
        return 1
    fi
    
    # Read and scan file content
    local content=$(cat "$file_path" 2>/dev/null || echo "")
    
    # Check all pattern types
    for pattern_type in api_keys credentials regex_patterns; do
        if check_sensitive_content "$content" "$pattern_type"; then
            return 0  # Found sensitive data
        fi
    done
    
    return 1
}

# Main scanning logic
main() {
    # Extract tool information from stdin
    local tool_name=$(echo "$INPUT_JSON" | jq -r '.tool_name // ""')
    local tool_args=$(echo "$INPUT_JSON" | jq -r '.tool_input // "{}"')
    
    log_security_event "scan_started" "$tool_name" "$tool_name"
    
    # Check code_context for sensitive data
    local code_context=$(echo "$tool_args" | jq -r '.code_context // ""' 2>/dev/null || echo "")
    if [[ -n "$code_context" ]]; then
        for pattern_type in api_keys credentials regex_patterns; do
            if check_sensitive_content "$code_context" "$pattern_type"; then
                log_security_event "blocked" "sensitive_data_in_code_context" "$tool_name"
                echo '{"decision": "block", "reason": "Security Alert: Detected sensitive data in code_context. Found patterns matching actual credentials (API keys, passwords, or secrets with values). For discussions about security topics, use placeholders like YOUR_API_KEY, <password>, or example values instead of real credentials."}'
                exit 2
            fi
        done
    fi
    
    # Check problem_description for sensitive data
    local problem_desc=$(echo "$tool_args" | jq -r '.problem_description // ""' 2>/dev/null || echo "")
    if [[ -n "$problem_desc" ]]; then
        for pattern_type in api_keys credentials regex_patterns; do
            if check_sensitive_content "$problem_desc" "$pattern_type"; then
                log_security_event "blocked" "sensitive_data_in_problem_description" "$tool_name"
                echo '{"decision": "block", "reason": "Security Alert: Detected sensitive data in problem description. Found patterns matching actual credentials (API keys, passwords, connection strings, or tokens with values). For security discussions, use placeholders: YOUR_API_KEY, <password>, postgres://user:password@localhost, or example-token-here."}'
                exit 2
            fi
        done
    fi
    
    # Check attached files
    local attached_files=$(echo "$tool_args" | jq -r '.attached_files[]?' 2>/dev/null || echo "")
    for file in $attached_files; do
        if scan_file_content "$file"; then
            log_security_event "blocked" "sensitive_file_attached:$file" "$tool_name"
            echo "{\"decision\": \"block\", \"reason\": \"Security Alert: Detected sensitive content in attached file $file. Found credentials, private keys, or environment files. Remove actual secrets and use placeholders like YOUR_SECRET_HERE or example values for demonstrations.\"}"
            exit 2
        fi
    done
    
    # Check specific question for Context7
    if [[ "$tool_name" == "mcp__context7__get-library-docs" ]]; then
        local library_id=$(echo "$tool_args" | jq -r '.context7CompatibleLibraryID // ""' 2>/dev/null || echo "")
        # Basic check to prevent injection attacks
        if echo "$library_id" | grep -qE '(\$|`|;|&&|\|\||>|<)'; then
            log_security_event "blocked" "suspicious_library_id" "$tool_name"
            echo '{"decision": "block", "reason": "Security Alert: Detected suspicious characters in library ID that could indicate command injection. Please use only alphanumeric characters, hyphens, underscores, and forward slashes."}'
            exit 2
        fi
    fi
    
    log_security_event "scan_completed" "no_sensitive_data_found" "$tool_name"
    
    # All checks passed, allow the tool to continue
    # No output needed when allowing - just exit 0
}

# Run main function
main