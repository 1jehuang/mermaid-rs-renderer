#!/bin/bash
# Claude Code notification hook script
# Plays pleasant sounds when Claude needs input or completes tasks

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOUNDS_DIR="$SCRIPT_DIR/sounds"

# Function to play a sound file with cross-platform support
play_sound_file() {
    local sound_file="$1"
    
    # Check if file exists
    if [[ ! -f "$sound_file" ]]; then
        echo "Warning: Sound file not found: $sound_file" >&2
        return 1
    fi
    
    # Detect OS and use appropriate command-line audio player
    local os_type="$(uname -s)"
    
    case "$os_type" in
        Darwin*)  # macOS
            if command -v afplay &> /dev/null; then
                afplay "$sound_file" 2>/dev/null &
                return 0  # Exit immediately after starting playback
            fi
            ;;
            
        Linux*)   # Linux
            # Try PulseAudio first (most common on modern desktop Linux)
            if command -v paplay &> /dev/null; then
                paplay "$sound_file" 2>/dev/null &
                return 0  # Exit immediately after starting playback
            fi
            
            # Try ALSA
            if command -v aplay &> /dev/null; then
                aplay -q "$sound_file" 2>/dev/null &
                return 0  # Exit immediately after starting playback
            fi
            
            # Try PipeWire (newer systems)
            if command -v pw-play &> /dev/null; then
                pw-play "$sound_file" 2>/dev/null &
                return 0  # Exit immediately after starting playback
            fi
            
            # Try sox play command
            if command -v play &> /dev/null; then
                play -q "$sound_file" 2>/dev/null &
                return 0  # Exit immediately after starting playback
            fi
            ;;
            
        MINGW*|CYGWIN*|MSYS*)  # Windows (Git Bash, WSL, etc.)
            # Try PowerShell
            if command -v powershell.exe &> /dev/null; then
                # Use Windows Media Player COM object for better compatibility
                # Run in background and exit immediately
                powershell.exe -NoProfile -Command "
                    Start-Job -ScriptBlock {
                        \$player = New-Object -ComObject WMPlayer.OCX
                        \$player.URL = '$sound_file'
                        \$player.controls.play()
                        Start-Sleep -Milliseconds 1000
                        \$player.close()
                    }
                " 2>/dev/null
                return 0  # Exit immediately after starting playback
            fi
            ;;
    esac
    
    # If we have ffplay (cross-platform)
    if command -v ffplay &> /dev/null; then
        ffplay -nodisp -autoexit -loglevel quiet "$sound_file" 2>/dev/null &
        return 0  # Exit immediately after starting playback
    fi
    
    # No audio player found - fail silently
    return 1
}

# Main script logic
case "$1" in
    "input")
        play_sound_file "$SOUNDS_DIR/input-needed.wav"
        ;;
        
    "complete")
        play_sound_file "$SOUNDS_DIR/complete.wav"
        ;;
        
    *)
        echo "Usage: $0 {input|complete}" >&2
        echo "  input    - Play sound when Claude needs user input" >&2
        echo "  complete - Play sound when Claude completes tasks" >&2
        exit 1
        ;;
esac

exit 0