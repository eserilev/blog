import LogbookCore.Funs
open Aeneas Aeneas.Std Result logbook_core

/-! Reading time (spec 7.6): T11, with T12 (no panic) built in. -/

namespace Logbook

/-- T11 + T12: `reading_minutes w = max 1 ⌈w / 220⌉`, and it never fails. -/
theorem reading_spec (w : Std.U32) :
    post.reading_minutes w ⦃ m => m.val = max 1 ((w.val + 219) / 220) ⦄ := by
  unfold post.reading_minutes post.WORDS_PER_MINUTE
  step*
  split
  · step* <;> scalar_tac
  · step* <;> scalar_tac

end Logbook
