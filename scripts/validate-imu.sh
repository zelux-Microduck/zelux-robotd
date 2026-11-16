#!/usr/bin/env bash
# Host-only IMU regression gate. No production service, deployment or real serial I/O.
# robotd integration tests spawn --fake children with private sockets.
# Run with: bash scripts/validate-imu.sh
set -Eeuo pipefail

repo="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd -- "$repo"
if [[ "$(uname -s)" != Linux ]]; then
    printf '%s\n' 'This gate requires Linux to cover IPC, permissions and board-facing code.' >&2
    exit 2
fi
for tool in cargo rustc git sha256sum sort xargs tee mktemp sed; do
    command -v "$tool" >/dev/null || { printf 'Missing tool: %s\n' "$tool" >&2; exit 2; }
done
export CARGO_TARGET_DIR="$repo/target"
mkdir -p target/imu-validation
evidence="$(mktemp -d target/imu-validation/run-XXXXXXXX)"
exec > >(tee "$evidence/run.log") 2>&1
trap 'status=$?; printf "exit_code=%s\n" "$status" > "$evidence/result.txt"; if ((status == 0)); then printf "PASS: host checks only; hardware not tested. Evidence: %s\n" "$evidence"; else printf "FAIL: host gate incomplete (exit %s). Evidence: %s\n" "$status" "$evidence"; fi' EXIT
printf 'Host-only IMU validation. Evidence: %s\n' "$evidence"
date -u '+%Y-%m-%dT%H:%M:%SZ' > "$evidence/started-at.txt"
git rev-parse HEAD > "$evidence/base-commit.txt"
git status --short > "$evidence/worktree-status.txt"
git diff --binary HEAD > "$evidence/tracked-changes.patch"
# Include untracked source too: HEAD and a tracked diff alone cannot identify this build.
git ls-files -z --cached --others --exclude-standard \
    | sort -zu | xargs -0 sha256sum > "$evidence/source-files.sha256"
cargo --version
rustc -vV
host="$(rustc -vV | sed -n 's/^host: //p')"
[[ -n "$host" ]] || { printf '%s\n' 'Cannot determine native Rust target' >&2; exit 2; }
cargo fmt --all --check
cargo test --locked --target "$host" -p robotd-params
cargo test --locked --target "$host" -p duck-control
cargo test --locked --target "$host" -p robotd
cargo test --locked --target "$host" -p robotctl
cargo clippy --locked --target "$host" -p duck-control -p robotd-params -p robotd -p robotctl --all-targets -- -D warnings
# Build only here; the tests above may spawn --fake daemons.
# Never run a production daemon or label a host binary a board release.
cargo build --locked --target "$host" -p robotd -p robotctl --bins
sha256sum "target/$host/debug/robotd" "target/$host/debug/robotctl" > "$evidence/host-binary.sha256"
