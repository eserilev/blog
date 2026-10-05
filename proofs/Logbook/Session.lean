import LogbookCore.Funs
open Aeneas Aeneas.Std Result logbook_core

/-! Session and setup token expiry (spec 7.6): T17, with T12 (no panic) built in.
Times are Unix seconds. Single use of a setup token relies on the atomic SQL
`UPDATE` in the server. These theorems do not cover it. -/

namespace Logbook

/-- T17 + T12: a session is valid if and only if `now` is before its expiry time. -/
theorem session_valid_spec (now exp : Std.I64) :
    session.session_valid now exp ⦃ b => (b ↔ now.val < exp.val) ⦄ := by
  unfold session.session_valid
  simp

/-- T17 + T12: a setup token works if and only if it is not used and `now` is before
its expiry time. -/
theorem setup_token_usable_spec (now exp : Std.I64) (used : Bool) :
    session.setup_token_usable now exp used ⦃ b => (b ↔ used = false ∧ now.val < exp.val) ⦄ := by
  unfold session.setup_token_usable
  cases used <;> simp

/-- T17 + T12: a new setup token expires at most 15 minutes after `now`, and exactly
15 minutes after it when that time fits in an `i64`. -/
theorem setup_token_expiry_spec (now : Std.I64) :
    session.setup_token_expiry now ⦃ e =>
      now.val ≤ e.val ∧ e.val - now.val ≤ 900 ∧
      (now.val + 900 ≤ I64.max → e.val = now.val + 900) ⦄ := by
  unfold session.setup_token_expiry session.SETUP_TOKEN_SECONDS core.num.I64.MAX
  step*
  all_goals simp_all [IScalar.ofInt, I64.max, I64.rMax]
  all_goals scalar_tac

end Logbook
