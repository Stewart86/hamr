#!/usr/bin/env bash
# Generate SHA256 checksums for all plugin files
# Output: plugins/checksums.json
#
# Usage:
#   ./scripts/generate-plugin-checksums.sh
#   ./scripts/generate-plugin-checksums.sh --verify  # Verify existing checksums

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PLUGINS_DIR="$PROJECT_ROOT/plugins"
OUTPUT_FILE="$PLUGINS_DIR/checksums.json"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

verify_mode=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --verify)
            verify_mode=true
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [--verify]"
            echo ""
            echo "Generate SHA256 checksums for all plugin files."
            echo ""
            echo "Options:"
            echo "  --verify    Verify existing checksums instead of generating new ones"
            echo "  -h, --help  Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Find all plugin directories (those with manifest.json)
get_plugin_dirs() {
    find "$PLUGINS_DIR" -mindepth 1 -maxdepth 1 -type d \
        -exec test -f '{}/manifest.json' \; -print | sort
}

# Get checksum for a single file
get_checksum() {
    local file="$1"
    sha256sum "$file" | cut -d' ' -f1
}

# Generate checksums for all plugins
generate_checksums() {
    local first=true
    
    echo "{"
    echo "  \"version\": 1,"
    echo "  \"generated\": \"$(date -Iseconds)\","
    echo "  \"plugins\": {"
    
    for plugin_dir in $(get_plugin_dirs); do
        local plugin_name
        plugin_name=$(basename "$plugin_dir")
        
        # Skip SDK directory
        if [[ "$plugin_name" == "sdk" ]]; then
            continue
        fi
        
        if [[ "$first" != true ]]; then
            echo ","
        fi
        first=false
        
        printf "    \"%s\": {" "$plugin_name"
        
        local file_first=true
        echo ""
        
        # Checksum manifest.json
        if [[ -f "$plugin_dir/manifest.json" ]]; then
            printf "      \"manifest.json\": \"%s\"" "$(get_checksum "$plugin_dir/manifest.json")"
            file_first=false
        fi
        
        # Checksum handler.py
        if [[ -f "$plugin_dir/handler.py" ]]; then
            if [[ "$file_first" != true ]]; then
                echo ","
            fi
            printf "      \"handler.py\": \"%s\"" "$(get_checksum "$plugin_dir/handler.py")"
            file_first=false
        fi
        
        # Checksum any additional tracked files (data files used by plugins)
        for extra_file in "$plugin_dir"/*.tsv "$plugin_dir"/launch-*; do
            if [[ -f "$extra_file" ]]; then
                local filename
                filename=$(basename "$extra_file")
                if [[ "$file_first" != true ]]; then
                    echo ","
                fi
                printf "      \"%s\": \"%s\"" "$filename" "$(get_checksum "$extra_file")"
                file_first=false
            fi
        done
        
        echo ""
        printf "    }"
    done
    
    echo ""
    echo "  }"
    echo "}"
}

# Verify checksums against existing file
verify_checksums() {
    if [[ ! -f "$OUTPUT_FILE" ]]; then
        echo -e "${RED}Error: No checksums.json found. Run without --verify first.${NC}"
        exit 1
    fi
    
    local exit_code=0
    local verified=0
    local modified=0
    local unknown=0
    
    echo "Verifying plugin checksums..."
    echo ""
    
    for plugin_dir in $(get_plugin_dirs); do
        local plugin_name
        plugin_name=$(basename "$plugin_dir")
        
        if [[ "$plugin_name" == "sdk" ]]; then
            continue
        fi
        
        # Check each file
        for file in manifest.json handler.py; do
            local filepath="$plugin_dir/$file"
            if [[ ! -f "$filepath" ]]; then
                continue
            fi
            
            local current_checksum
            current_checksum=$(get_checksum "$filepath")
            
            # Extract expected checksum from JSON
            local expected_checksum
            expected_checksum=$(grep -A 20 "\"$plugin_name\":" "$OUTPUT_FILE" | \
                grep "\"$file\":" | \
                sed -E 's/.*"[^"]+": "([a-f0-9]+)".*/\1/' | head -1 || true)
            
            if [[ -z "$expected_checksum" ]]; then
                echo -e "${YELLOW}[UNKNOWN]${NC} $plugin_name/$file"
                ((unknown++)) || true
            elif [[ "$current_checksum" == "$expected_checksum" ]]; then
                echo -e "${GREEN}[OK]${NC} $plugin_name/$file"
                ((verified++)) || true
            else
                echo -e "${RED}[MODIFIED]${NC} $plugin_name/$file"
                ((modified++)) || true
                exit_code=1
            fi
        done
    done
    
    echo ""
    echo "Summary: $verified verified, $modified modified, $unknown unknown"
    
    if [[ $modified -gt 0 ]]; then
        echo -e "${RED}Some plugins have been modified since checksums were generated.${NC}"
    fi
    
    exit $exit_code
}

# Main
if [[ "$verify_mode" == true ]]; then
    verify_checksums
else
    echo "Generating plugin checksums..."
    generate_checksums > "$OUTPUT_FILE"
    echo -e "${GREEN}Checksums written to: $OUTPUT_FILE${NC}"
fi
