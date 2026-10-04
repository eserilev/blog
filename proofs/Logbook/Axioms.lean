import Logbook.Policy
import Logbook.Media
import Logbook.Slug
import Logbook.Post

/-! The build fails if a theorem below depends on any axiom other than Lean's three
standard ones: no `sorry`, and none of the generated `Option` axioms in
`LogbookCore/FunsExternal.lean`. -/

/-- info: 'Logbook.reveal_spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.reveal_spec

/-- info: 'Logbook.reveal_iff' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.reveal_iff

/-- info: 'Logbook.filter_public_spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.filter_public_spec

/-- info: 'Logbook.reading_spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.reading_spec

/-- info: 'Logbook.media_key_spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.media_key_spec

/-- info: 'Logbook.make_slug_spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.make_slug_spec

/-- info: 'Logbook.slug_charset' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.slug_charset
