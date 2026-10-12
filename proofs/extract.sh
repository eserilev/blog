#!/usr/bin/env bash
# Regenerates the Lean models of logbook-core and logbook-paint (spec 7.6):
# Rust → Charon → Aeneas → Lean.
# CI runs this and fails if the committed files in proofs/LogbookCore or
# proofs/LogbookPaint differ.
set -euo pipefail
cd "$(dirname "$0")"
tools=$PWD/.tools
llbc=$(mktemp -d)/logbook_core.llbc
(cd ../crates/logbook-core && "$tools/charon/charon" cargo --preset=aeneas --dest-file "$llbc" >/dev/null)
rm -f LogbookCore/Types.lean LogbookCore/Funs.lean LogbookCore/FunsExternal_Template.lean
"$tools/aeneas/aeneas" -backend lean -split-files -dest LogbookCore "$llbc" >/dev/null 2>&1 || { echo "aeneas failed"; "$tools/aeneas/aeneas" -backend lean -split-files -dest LogbookCore "$llbc"; exit 1; }
# The external functions are the derived Debug and PartialEq of Option. No verified
# function calls them, so the template's axioms are used as they are.
mv LogbookCore/FunsExternal_Template.lean LogbookCore/FunsExternal.lean
sed -i 's/^-- This is a template file: rename it to "FunsExternal.lean" and fill the holes.$/-- Generated from the template by proofs\/extract.sh. No verified function uses these./' LogbookCore/FunsExternal.lean
echo "extracted: proofs/LogbookCore/{Types,Funs,FunsExternal}.lean"

# logbook-paint calls no external function, so Aeneas writes no template for it.
llbc=$(mktemp -d)/logbook_paint.llbc
(cd ../crates/logbook-paint && "$tools/charon/charon" cargo --preset=aeneas --dest-file "$llbc" >/dev/null)
rm -f LogbookPaint/Types.lean LogbookPaint/Funs.lean
"$tools/aeneas/aeneas" -backend lean -split-files -dest LogbookPaint "$llbc" >/dev/null 2>&1 || { echo "aeneas failed"; "$tools/aeneas/aeneas" -backend lean -split-files -dest LogbookPaint "$llbc"; exit 1; }
test ! -e LogbookPaint/FunsExternal_Template.lean || { echo "logbook-paint has external functions"; exit 1; }
echo "extracted: proofs/LogbookPaint/{Types,Funs}.lean"
