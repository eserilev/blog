import Logbook.Paint.Rect
import Logbook.Paint.Line
open Aeneas Aeneas.Std Result logbook_paint

/-! `line` (spec 7.6): P10 (no panic, an invalid canvas does not change). -/

namespace Logbook.Paint

/-- A canvas with the same fields as `c0`, where each pixel is the old one or `color`. -/
def Painted (c0 : Canvas) (color : Std.U32) (c : Canvas) : Prop :=
  c.w = c0.w ∧ c.h = c0.h ∧ c.px.val.length = c0.px.val.length ∧
    (∀ k < c0.px.val.length, c.px.val[k]! = c0.px.val[k]! ∨ c.px.val[k]! = color) ∧
    (¬ Valid c0 → c = c0)

theorem painted_fill {c0 c r : Canvas} {color : Std.U32} {x y w h : Std.I32}
    (hc : Painted c0 color c) (hr : r.w = c.w ∧ r.h = c.h ∧ r.px.val.length = c.px.val.length ∧
      (Valid c → ∀ k < c.px.val.length, r.px.val[k]! =
        if InClip c.w.val x.val y.val w.val h.val k then color else c.px.val[k]!) ∧
      (¬ Valid c → r = c)) : Painted c0 color r := by
  obtain ⟨hw, hh, hl, hp, hi⟩ := hc
  obtain ⟨rw', rh, rl, rp, ri⟩ := hr
  have hv := valid_of_same hw hh hl
  refine ⟨by rw [rw', hw], by rw [rh, hh], by rw [rl, hl], fun k hk => ?_, fun h => ?_⟩
  · by_cases hvc : Valid c
    · rw [rp hvc k (by omega)]
      split_ifs
      · exact Or.inr rfl
      · exact hp k hk
    · rw [ri hvc]; exact hp k hk
  · rw [ri (fun h' => h (hv.mp h')), hi h]

theorem line_loop_spec (c0 c : Canvas) (color : Std.U32) (wd off : Std.I32)
    (pts : alloc.vec.Vec (Std.I32 × Std.I32)) (i : Std.Usize) (hoff : 0 ≤ off.val ∧ off.val ≤ 64)
    (hpts : ∀ j < pts.val.length, -65536 ≤ (pts.val[j]!).1.val ∧ (pts.val[j]!).1.val ≤ 65536 ∧
      -65536 ≤ (pts.val[j]!).2.val ∧ (pts.val[j]!).2.val ≤ 65536)
    (hi : i.val ≤ pts.val.length) (hc : Painted c0 color c) :
    line_loop c color wd off pts i ⦃ r => Painted c0 color r ⦄ := by
  unfold line_loop
  apply loop.spec_decr_nat
    (measure := fun (p : Canvas × Std.Usize) => pts.val.length - p.2.val)
    (inv := fun p => p.2.val ≤ pts.val.length ∧ Painted c0 color p.1)
  · rintro ⟨d, j⟩ ⟨hj, hd⟩
    simp only at hj hd
    unfold line_loop.body
    step*
    all_goals
      have hjl : j.val < pts.val.length := by scalar_tac
      have hb := hpts j.val hjl
      rw [getElem!_pos pts.val j.val hjl, ← px_post] at hb
      simp only at hb
    · scalar_tac
    · scalar_tac
    · exact ⟨by scalar_tac, painted_fill hd ⟨c1_post, c1_post1, c1_post2, c1_post3, c1_post4⟩,
        by scalar_tac⟩
  · exact ⟨hi, hc⟩

/-- P10: `line` returns `ok` for every input. Each pixel of the result is the old pixel
or `color`. A canvas that is not valid does not change. -/
@[step]
theorem line_spec (c : Canvas) (x0 y0 x1 y1 width : Std.I32) (color : Std.U32) :
    line c x0 y0 x1 y1 width color ⦃ r => Painted c color r ⦄ := by
  unfold line
  have := clampW_bounds width.val
  step*
  · have hoff : 0 ≤ off.val ∧ off.val ≤ 64 := by
      rw [off_post, Int.tdiv_eq_ediv_of_nonneg (by omega)]; omega
    have := line_loop_spec c c color wd off pts 0#usize hoff pts_post2 (by simp)
      ⟨rfl, rfl, rfl, fun k _ => Or.inl rfl, fun _ => rfl⟩
    step*
  · exact ⟨rfl, rfl, rfl, fun k _ => Or.inl rfl, fun _ => rfl⟩

end Logbook.Paint
