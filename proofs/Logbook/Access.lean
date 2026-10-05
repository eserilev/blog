import LogbookCore.Funs
open Aeneas Aeneas.Std Result logbook_core

/-! Route access (spec 7.6): T16, with T12 (no panic) built in. -/

namespace Logbook

/-- T16 + T12: `authorize` gives 401 exactly for an owner route without a valid
session. -/
theorem authorize_spec (a : access.Access) (s : access.SessionState) :
    access.authorize a s ⦃ d => (d = .Unauthorized ↔ a = .Owner ∧ s ≠ .Valid) ⦄ := by
  unfold access.authorize
  cases a <;> cases s <;> simp

/-- T16: an owner route allows a request only with a valid session. -/
theorem authorize_owner (s : access.SessionState) :
    access.authorize .Owner s ⦃ d => (d = .Allow ↔ s = .Valid) ⦄ := by
  unfold access.authorize
  cases s <;> simp

/-- T16: a public, session, or sign-in route never gives 401, for every session state. -/
theorem authorize_not_owner (a : access.Access) (s : access.SessionState)
    (h : a ≠ .Owner) : access.authorize a s ⦃ d => d = .Allow ⦄ := by
  unfold access.authorize
  cases a <;> cases s <;> simp_all

end Logbook
