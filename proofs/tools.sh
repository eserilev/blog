#!/usr/bin/env bash
# Downloads the pinned Charon and Aeneas builds into proofs/.tools (spec 7.6).
# The Aeneas build includes its Lean library. Checksums are checked.
set -euo pipefail
cd "$(dirname "$0")"

AENEAS_TAG=nightly-2026.10.04-557eff8
AENEAS_SHA256=5b2d33e41498b44a20cc4ddc852b368b4edd3f3ed2d6a55a176ac79ea3b29d58
# Aeneas pins Charon v0.1.279 (charon-pin in the Aeneas repo). This nightly is v0.1.279.
CHARON_TAG=nightly-2026.10.02
CHARON_SHA256=98c1771deff46d213a35d677746e85d68a0efa14cdbc883feb7c7f5d1b9d149e

fetch() { # url sha256 dest-dir
  local tmp; tmp=$(mktemp)
  curl -fsSL -o "$tmp" "$1"
  echo "$2  $tmp" | sha256sum -c --quiet -
  rm -rf "$3" && mkdir -p "$3" && tar -xzf "$tmp" -C "$3"
  rm -f "$tmp"
}

if [ "$(cat .tools/aeneas.tag 2>/dev/null)" != "$AENEAS_TAG" ]; then
  fetch "https://github.com/AeneasVerif/aeneas/releases/download/$AENEAS_TAG/aeneas-linux-x86_64.tar.gz" "$AENEAS_SHA256" .tools/aeneas
  echo "$AENEAS_TAG" > .tools/aeneas.tag
fi
if [ "$(cat .tools/charon.tag 2>/dev/null)" != "$CHARON_TAG" ]; then
  fetch "https://github.com/AeneasVerif/charon/releases/download/$CHARON_TAG/charon-linux-x86_64.tar.gz" "$CHARON_SHA256" .tools/charon
  echo "$CHARON_TAG" > .tools/charon.tag
fi
# Charon drives this exact nightly toolchain.
rustup toolchain install "$(sed -n 's/^channel = "\(.*\)"/\1/p' .tools/charon/rust-toolchain)" \
  --profile minimal --component rustc-dev,llvm-tools,rust-src >/dev/null
echo "tools: aeneas $AENEAS_TAG, charon $CHARON_TAG"
