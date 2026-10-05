import Lake
open Lake DSL

-- The Aeneas Lean library comes from the pinned Aeneas build (tools.sh).
require aeneas from "./.tools/aeneas/backends/lean"

package logbook

-- The model generated from crates/logbook-core (extract.sh). Do not edit.
lean_lib LogbookCore where
  globs := #[.submodules `LogbookCore]

-- The theorems (spec 7.6).
@[default_target]
lean_lib Logbook where
  globs := #[.submodules `Logbook]
