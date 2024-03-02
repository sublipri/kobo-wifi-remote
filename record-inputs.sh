#!/bin/sh
# Useful for testing or as a way to quickly record actions without the web UI
set -e
echo "Recording Next Page"
curl -s --json '{"name": "Next", "sort_value": "%00", "path_segment": "next-page", "keyboard_shortcut": "ArrowRight", "new_event_timeout": 100 }' "$KOBO_HOST"/actions | jq .
echo "Recording Prev Page"
curl -s --json '{"name": "Prev", "sort_value": "%01", "path_segment": "prev-page", "keyboard_shortcut": "ArrowLeft", "new_event_timeout": 100 }' "$KOBO_HOST"/actions | jq .
echo "Brightness Up"
curl -s --json '{"name": "🔆", "keyboard_shortcut": "PageUp", "new_event_timeout": 100 }' "$KOBO_HOST"/actions | jq .
echo "Brightness Down"
curl -s --json '{"name": "🔅", "keyboard_shortcut": "PageDown", "new_event_timeout": 100 }' "$KOBO_HOST"/actions | jq .
