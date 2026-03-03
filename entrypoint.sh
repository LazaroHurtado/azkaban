#!/bin/bash
set -e

CONFIG_FILE="/workspace/azkaban/config.yaml"

if [ -f "$CONFIG_FILE" ]; then
    tool_count=$(yq '.cli_tools | length' "$CONFIG_FILE")
    for i in $(seq 0 $((tool_count - 1))); do
        cli_cmd=$(yq ".cli_tools[$i].cli_cmd" "$CONFIG_FILE")
        install_cmd=$(yq ".cli_tools[$i].install_cmd" "$CONFIG_FILE")
        binary=$(echo "$cli_cmd" | awk '{print $1}')

        if [ "$install_cmd" = "null" ] || [ -z "$install_cmd" ]; then
            continue
        fi

        if command -v "$binary" >/dev/null 2>&1; then
            echo "✅ $binary already installed"
        else
            echo "📦 Installing $binary..."
            sudo bash -c "$install_cmd" || echo "⚠️  Failed to install $binary"
        fi
    done
else
    echo "⚠️  No config.yaml found at $CONFIG_FILE"
fi

echo "🚀 Azkaban sandbox ready"
exec sleep infinity
