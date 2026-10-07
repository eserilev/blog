import Logbook.Policy
import Logbook.Media
import Logbook.Slug
import Logbook.SlugExact
import Logbook.Post
import Logbook.Csrf
import Logbook.Html
import Logbook.Ip
import Logbook.Access
import Logbook.Session
import Logbook.Video

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

/-- info: 'Logbook.Video.youtube_id_spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.Video.youtube_id_spec

/-- info: 'Logbook.Video.youtube_id_plain' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.Video.youtube_id_plain

/-- info: 'Logbook.make_slug_spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.make_slug_spec

/-- info: 'Logbook.slug_charset' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.slug_charset

/-- info: 'Logbook.slug_plain' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.slug_plain

/-- info: 'Logbook.slug_fallback' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.slug_fallback

/-- info: 'Logbook.slug_lower' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.slug_lower

/-- info: 'Logbook.slug_idempotent' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.slug_idempotent

/-- info: 'Logbook.write_allowed_spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.write_allowed_spec

/-- info: 'Logbook.csrf_cross_origin' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.csrf_cross_origin

/-- info: 'Logbook.csrf_body' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.csrf_body

/-- info: 'Logbook.csrf_read' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.csrf_read

/-- info: 'Logbook.escape_html_spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.escape_html_spec

/-- info: 'Logbook.escape_no_special' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.escape_no_special

/-- info: 'Logbook.escape_amp' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.escape_amp

/-- info: 'Logbook.unescape_escape' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.unescape_escape

/-- info: 'Logbook.client_addr_spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.client_addr_spec

/-- info: 'Logbook.client_addr_untrusted_peer' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.client_addr_untrusted_peer

/-- info: 'Logbook.authorize_spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.authorize_spec

/-- info: 'Logbook.authorize_owner' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.authorize_owner

/-- info: 'Logbook.authorize_not_owner' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.authorize_not_owner

/-- info: 'Logbook.session_valid_spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.session_valid_spec

/-- info: 'Logbook.setup_token_usable_spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.setup_token_usable_spec

/-- info: 'Logbook.setup_token_expiry_spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Logbook.setup_token_expiry_spec
