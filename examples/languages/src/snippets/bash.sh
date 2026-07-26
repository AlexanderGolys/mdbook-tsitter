#!/usr/bin/env bash
set -euo pipefail

declare -A checksums=()
declare -a artifacts=()
workdir="$(mktemp -d)"

cleanup() {
  local status=$?
  rm -rf -- "$workdir"
  exit "$status"
}
trap cleanup EXIT INT TERM

log() {
  printf '[%(%H:%M:%S)T] %s\n' -1 "$*"
}

discover() {
  local root=$1
  while IFS= read -r -d '' path; do
    artifacts+=("$path")
  done < <(
    find "$root" -type f \
      \( -name '*.tar.gz' -o -name '*.zip' \) \
      -print0
  )
}

verify() {
  local path=$1
  local name=${path##*/}
  local expected=${checksums[$name]:-}

  case "$name" in
    *.tar.gz) tar -tzf "$path" >/dev/null ;;
    *.zip) unzip -tq "$path" >/dev/null ;;
    *) return 2 ;;
  esac

  if [[ -n $expected ]]; then
    local actual
    actual=$(sha256sum "$path")
    [[ ${actual%% *} == "$expected" ]]
  fi
}

main() {
  checksums[release.tar.gz]=$(
    cut -d' ' -f1 checksums.txt
  )
  discover "${1:-dist}"

  local failed=0
  for artifact in "${artifacts[@]}"; do
    if verify "$artifact"; then
      log "ok: ${artifact##*/}"
    else
      log "failed: ${artifact##*/}"
      ((failed += 1))
    fi
  done
  return "$failed"
}

main "$@"
