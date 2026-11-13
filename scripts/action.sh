#!/bin/sh
set -eu

binary=${ALREADY_TESTED_BINARY:-}
if [ -z "$binary" ]; then
  if directory=$(mktemp -d "${RUNNER_TEMP:?RUNNER_TEMP is required}/already-tested.XXXXXX") &&
    sh "${GITHUB_ACTION_PATH:?GITHUB_ACTION_PATH is required}/scripts/install.sh" \
      --version "${INPUT_VERSION:-v0.3.1}" --bin-dir "$directory"; then
    binary="$directory/already-tested"
    [ ! -f "$directory/already-tested.exe" ] || binary="$directory/already-tested.exe"
  else
    printf '%s\n' 'already-tested: Could not install verifier; running CI normally.'
    printf 'skip=false\n' >> "${GITHUB_OUTPUT:?GITHUB_OUTPUT is required}"
    if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
      if ! printf '\n### Already Tested\n\n**Run CI normally.**\n\nCould not install verifier.\n\nClaims are trusted statements, not proof that tests ran.\n' >> "$GITHUB_STEP_SUMMARY"; then
        printf '%s\n' 'already-tested: Could not write job summary.' >&2
      fi
    fi
    exit 0
  fi
fi
exec "$binary" action
