#!/usr/bin/env bash
# uninstall-old-drovr.sh
#
# TEMPORARY migration script — delete this file once you've run it.
#
# Removes traces of the old `drovr` / `dv` install left behind before the
# rename to `radr` / `rd`:
#   - `dv` binary in ~/.local/bin
#   - `dv` binary installed via `cargo install` (package name `drovr`)
#   - old pi extension at ~/.pi/agent/extensions/drovr.ts
#   - old agent state directory ($XDG_DATA_HOME/drovr, default
#     ~/.local/share/drovr)
#   - old `dv set-state` / `dv clear-state` hook entries in
#     ~/.claude/settings.json (a backup is made before editing)
#
# After running this, re-install the new tool and hooks:
#   mise run install
#   rd hooks install pi
#   rd hooks install claude
#
# Usage:
#   ./scripts/uninstall-old-drovr.sh          # perform the cleanup
#   ./scripts/uninstall-old-drovr.sh --dry-run  # show what would happen

set -euo pipefail

DRY_RUN=0
if [[ "${1:-}" == "--dry-run" ]]; then
  DRY_RUN=1
  echo "== dry run: no changes will be made =="
fi

run() {
  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "would run: $*"
  else
    echo "+ $*"
    "$@"
  fi
}

XDG_DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"

echo "== 1. old ~/.local/bin/dv binary =="
if [[ -e "$HOME/.local/bin/dv" ]]; then
  run rm -f "$HOME/.local/bin/dv"
else
  echo "not found, skipping"
fi

echo
echo "== 2. old cargo-installed 'drovr' package (binary dv) =="
if command -v cargo >/dev/null 2>&1; then
  if cargo install --list 2>/dev/null | grep -q '^drovr '; then
    run cargo uninstall drovr
  else
    echo "not found, skipping"
  fi
else
  echo "cargo not found, skipping"
fi

echo
echo "== 3. old pi extension ~/.pi/agent/extensions/drovr.ts =="
PI_EXT="$HOME/.pi/agent/extensions/drovr.ts"
if [[ -e "$PI_EXT" ]]; then
  run rm -f "$PI_EXT"
else
  echo "not found, skipping"
fi

echo
echo "== 4. old agent state dir \$XDG_DATA_HOME/drovr =="
STATE_DIR="$XDG_DATA_HOME/drovr"
if [[ -d "$STATE_DIR" ]]; then
  run rm -rf "$STATE_DIR"
else
  echo "not found, skipping"
fi

echo
echo "== 5. old 'dv set-state' / 'dv clear-state' hooks in ~/.claude/settings.json =="
CLAUDE_SETTINGS="$HOME/.claude/settings.json"
if [[ -f "$CLAUDE_SETTINGS" ]]; then
  if ! command -v jq >/dev/null 2>&1; then
    echo "jq is required to safely edit $CLAUDE_SETTINGS — skipping this step."
    echo "You can remove old hooks manually, or install jq and re-run this script."
  elif jq -e '
      .hooks // {} | to_entries | any(.value[]?.hooks[]?.command? |
        (type == "string") and (startswith("dv set-state ") or . == "dv clear-state"))
    ' "$CLAUDE_SETTINGS" >/dev/null 2>&1; then
    BACKUP="$CLAUDE_SETTINGS.bak.$(date +%s)"
    run cp "$CLAUDE_SETTINGS" "$BACKUP"
    echo "backed up to $BACKUP"

    if [[ "$DRY_RUN" -eq 1 ]]; then
      echo "would remove hook entries whose command is 'dv set-state ...' or 'dv clear-state'"
    else
      TMP="$(mktemp)"
      jq '
        .hooks |= (
          . // {} | with_entries(
            .value |= map(
              select(
                (.hooks // []) | any(
                  (.command? // "") as $c
                  | ($c | startswith("dv set-state ") or . == "dv clear-state")
                ) | not
              )
            )
          )
        )
      ' "$CLAUDE_SETTINGS" > "$TMP"
      mv "$TMP" "$CLAUDE_SETTINGS"
      echo "removed old drovr hook entries from $CLAUDE_SETTINGS"
    fi
  else
    echo "no old 'dv' hook entries found, skipping"
  fi
else
  echo "$CLAUDE_SETTINGS not found, skipping"
fi

echo
echo "== done =="
echo "Next steps:"
echo "  mise run install        # build & install the new 'rd' binary"
echo "  rd hooks install pi     # (re)install the pi extension"
echo "  rd hooks install claude # (re)install claude hooks"
echo
echo "You can delete this script now: rm scripts/uninstall-old-drovr.sh"
