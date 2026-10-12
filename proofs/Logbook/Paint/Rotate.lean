import Logbook.Paint.Basic
open Aeneas Aeneas.Std Result logbook_paint

/-! Rotate 90° clockwise (spec 7.6): P6, with P10 (no panic) built in. -/

namespace Logbook.Paint

/-- The source index of output index `j` when a `w × h` picture turns 90°. -/
def rot (w h j : Nat) : Nat := (h - 1 - j % h) * w + j / h

/-- Turning twice: the source index of `j` under a 180° turn of a `w × h` picture. -/
def rot2 (w h j : Nat) : Nat := (h - 1 - j / w) * w + (w - 1 - j % w)

theorem div_lt_of_lt_mul' {j w h : Nat} (hj : j < w * h) : j / h < w := by
  apply Nat.div_lt_of_lt_mul; rwa [Nat.mul_comm]

theorem rot_lt {w h j : Nat} (hj : j < w * h) : rot w h j < w * h := by
  have hh : 0 < h := Nat.pos_of_ne_zero (by rintro rfl; simp at hj)
  unfold rot
  exact idx_lt (div_lt_of_lt_mul' hj) (by have := Nat.mod_lt j hh; omega)

theorem rot_rot {w h j : Nat} (hj : j < h * w) : rot w h (rot h w j) = rot2 w h j := by
  have hw : 0 < w := Nat.pos_of_ne_zero (by rintro rfl; simp at hj)
  have hq : j / w < h := div_lt_of_lt_mul' hj
  obtain ⟨h1, h2⟩ := div_mod_of_lt (w := h) (b := w - 1 - j % w) hq
  unfold rot rot2
  rw [h1, h2]

theorem rot2_lt {w h j : Nat} (hj : j < w * h) : rot2 w h j < w * h := by
  have hw : 0 < w := Nat.pos_of_ne_zero (by rintro rfl; simp at hj)
  have hq : j / w < h := Nat.div_lt_of_lt_mul hj
  have hh : 0 < h := Nat.pos_of_ne_zero (by rintro rfl; simp at hj)
  unfold rot2
  exact idx_lt (a := w - 1 - j % w) (b := h - 1 - j / w) (by have := Nat.mod_lt j hw; omega)
    (by have := Nat.sub_le (h - 1) (j / w); omega)

theorem rot2_rot2 {w h j : Nat} (hj : j < w * h) : rot2 w h (rot2 w h j) = j := by
  have hw : 0 < w := Nat.pos_of_ne_zero (by rintro rfl; simp at hj)
  have hq : j / w < h := Nat.div_lt_of_lt_mul hj
  have hr := Nat.mod_lt j hw
  obtain ⟨h1, h2⟩ := div_mod_of_lt (w := w) (b := h - 1 - j / w) (a := w - 1 - j % w) (by omega)
  unfold rot2
  rw [h1, h2]
  have : h - 1 - (h - 1 - j / w) = j / w := Nat.sub_sub_self (by omega)
  rw [this, show w - 1 - (w - 1 - j % w) = j % w by omega, Nat.mul_comm, Nat.div_add_mod]

/-! The loops. -/

/-- Row `y` of the output is done up to column `x` (exclusive): `Done y x j`. -/
def RotDone (h y x j : Nat) : Prop := h - 1 - j % h < y ∨ (h - 1 - j % h = y ∧ j / h < x)

theorem rotate90_inner_spec (W H : Std.Usize) (v v1 : alloc.vec.Vec Std.U32) (y x : Std.Usize)
    (hW : 1 ≤ W.val ∧ W.val ≤ 4096) (hH : 1 ≤ H.val ∧ H.val ≤ 4096)
    (hv : v.val.length = W.val * H.val) (hv1 : v1.val.length = W.val * H.val)
    (hy : y.val < H.val) (hx : x.val ≤ W.val)
    (hdone : ∀ j < W.val * H.val, RotDone H.val y.val x.val j →
      v1.val[j]! = v.val[rot W.val H.val j]!) :
    rotate90_loop0_loop0 W H v H v1 y x ⦃ r => r.val.length = W.val * H.val ∧
      ∀ j < W.val * H.val, RotDone H.val y.val W.val j →
        r.val[j]! = v.val[rot W.val H.val j]! ⦄ := by
  unfold rotate90_loop0_loop0
  apply loop.spec_decr_nat
    (measure := fun (p : alloc.vec.Vec Std.U32 × Std.Usize) => W.val - p.2.val)
    (inv := fun p => p.1.val.length = W.val * H.val ∧ p.2.val ≤ W.val ∧
      ∀ j < W.val * H.val, RotDone H.val y.val p.2.val j →
        p.1.val[j]! = v.val[rot W.val H.val j]!)
  · rintro ⟨v1, x⟩ ⟨hl, hx, hd⟩
    simp only at hl hx hd
    unfold rotate90_loop0_loop0.body
    by_cases hxW : x.val < W.val
    · have hk1 : y.val * W.val + x.val < W.val * H.val := idx_lt (by scalar_tac) hy
      have hk2 : x.val * H.val + (H.val - 1 - y.val) < W.val * H.val := by
        rw [Nat.mul_comm W.val]; exact idx_lt (by omega) (by scalar_tac)
      step*
      refine ⟨by simp [index_mut_back_post, hl], by scalar_tac, ?_, by scalar_tac⟩
      have hi6 : i6.val = x.val * H.val + (H.val - 1 - y.val) := by scalar_tac
      have hi1 : i1.val = y.val * W.val + x.val := by scalar_tac
      obtain ⟨hq, hr⟩ := div_mod_of_lt (w := H.val) (b := x.val) (a := H.val - 1 - y.val) (by omega)
      intro j hj hdj
      simp only [index_mut_back_post, alloc.vec.Vec.set_val_eq]
      by_cases hjk : i6.val = j
      · subst hjk
        rw [set_eq _ _ (by scalar_tac), i2_post]
        unfold rot
        rw [hi6, hq, hr]
        have : (H.val - 1 - (H.val - 1 - y.val)) * W.val + x.val = i1.val := by
          rw [hi1, Nat.sub_sub_self (by omega)]
        rw [this, getElem!_pos v.val i1.val (by scalar_tac)]
      · rw [set_ne _ _ hjk]
        apply hd j hj
        unfold RotDone at hdj ⊢
        rcases hdj with h | ⟨h1, h2⟩
        · exact Or.inl h
        · refine Or.inr ⟨h1, ?_⟩
          by_contra hc
          have : j / H.val = x.val := by scalar_tac
          have hm : j % H.val = H.val - 1 - y.val := by
            have := Nat.mod_lt j (show 0 < H.val by omega); omega
          exact hjk (by rw [hi6, eq_of_div_mod this hm])
    · have : ¬ x < W := by scalar_tac
      simp only [this, if_false, WP.spec_ok]
      have : x.val = W.val := by omega
      exact ⟨hl, fun j hj hdj => hd j hj (by rw [this]; exact hdj)⟩
  · exact ⟨hv1, hx, hdone⟩

theorem rotate90_outer_spec (W H : Std.Usize) (v v1 : alloc.vec.Vec Std.U32) (y : Std.Usize)
    (hW : 1 ≤ W.val ∧ W.val ≤ 4096) (hH : 1 ≤ H.val ∧ H.val ≤ 4096)
    (hv : v.val.length = W.val * H.val) (hv1 : v1.val.length = W.val * H.val)
    (hy : y.val ≤ H.val)
    (hdone : ∀ j < W.val * H.val, H.val - 1 - j % H.val < y.val →
      v1.val[j]! = v.val[rot W.val H.val j]!) :
    rotate90_loop0 W H v H v1 y ⦃ r => r.val.length = W.val * H.val ∧
      ∀ j < W.val * H.val, r.val[j]! = v.val[rot W.val H.val j]! ⦄ := by
  unfold rotate90_loop0
  apply loop.spec_decr_nat
    (measure := fun (p : alloc.vec.Vec Std.U32 × Std.Usize) => H.val - p.2.val)
    (inv := fun p => p.1.val.length = W.val * H.val ∧ p.2.val ≤ H.val ∧
      ∀ j < W.val * H.val, H.val - 1 - j % H.val < p.2.val →
        p.1.val[j]! = v.val[rot W.val H.val j]!)
  · rintro ⟨v1, y⟩ ⟨hl, hy, hd⟩
    simp only at hl hy hd
    unfold rotate90_loop0.body
    by_cases hyH : y.val < H.val
    · have := rotate90_inner_spec W H v v1 y 0#usize hW hH hv hl hyH (by simp)
        (fun j hj h => hd j hj (by unfold RotDone at h; simp at h; omega))
      step*
      refine ⟨v2_post, by scalar_tac, fun j hj h => v2_post1 j hj ?_, by scalar_tac⟩
      unfold RotDone
      have : j / H.val < W.val := div_lt_of_lt_mul' hj
      omega
    · have : ¬ y < H := by scalar_tac
      simp only [this, if_false, WP.spec_ok]
      refine ⟨hl, fun j hj => hd j hj ?_⟩
      have := Nat.mod_lt j (show 0 < H.val by omega)
      omega
  · exact ⟨hv1, hy, hdone⟩

/-- P6: on a valid canvas, `rotate90` returns a valid canvas with the sides swapped.
Pixel `j` of the result is pixel `rot w h j` of the input. -/
@[step]
theorem rotate90_valid_spec (c : Canvas) (hc : Valid c) :
    rotate90 c ⦃ r => r.w = c.h ∧ r.h = c.w ∧ Valid r ∧
      ∀ j < c.w.val * c.h.val, r.px.val[j]! = c.px.val[rot c.w.val c.h.val j]! ⦄ := by
  obtain ⟨h1, h2, h3, h4, h5⟩ := hc
  unfold rotate90
  have hv : valid c ⦃ b => b = true ⦄ := by
    apply WP.spec_mono (valid_spec c); intro b hb; exact hb.mpr ⟨h1, h2, h3, h4, h5⟩
  step*
  have hlen : v.val.length = c.w.val * c.h.val := by
    rw [v_post, List.length_replicate, i_post, Nat.mul_comm]
  have := rotate90_outer_spec c.w c.h c.px v 0#usize ⟨h1, h3⟩ ⟨h2, h4⟩ h5 hlen (by simp)
    (fun j _ h => by simp at h)
  step*
  refine ⟨⟨h2, h1, h4, h3, by rw [v1_post, Nat.mul_comm]⟩, v1_post1⟩

/-- On a canvas that is not valid, `rotate90` gives the same canvas (P10). -/
theorem rotate90_invalid (c : Canvas) (hc : ¬ Valid c) : rotate90 c = ok c := by
  unfold rotate90
  have hv : valid c = ok false := by
    obtain ⟨b, hb, hb2⟩ := spec_ok_exists (valid_spec c)
    rw [hb]; cases b <;> simp_all
  obtain ⟨r, hr, hr2⟩ := spec_ok_exists (clone_spec c)
  simp [hv, hr, hr2]

/-- Two turns: a valid canvas with the same sides, turned 180°. -/
theorem rotate90_twice (c : Canvas) (hc : Valid c) :
    ∃ c1 c2, rotate90 c = ok c1 ∧ rotate90 c1 = ok c2 ∧ Valid c2 ∧ c2.w = c.w ∧ c2.h = c.h ∧
      ∀ j < c.w.val * c.h.val, c2.px.val[j]! = c.px.val[rot2 c.w.val c.h.val j]! := by
  obtain ⟨c1, e1, hw1, hh1, hv1, hp1⟩ := spec_ok_exists (rotate90_valid_spec c hc)
  obtain ⟨c2, e2, hw2, hh2, hv2, hp2⟩ := spec_ok_exists (rotate90_valid_spec c1 hv1)
  refine ⟨c1, c2, e1, e2, hv2, by rw [hw2, hh1], by rw [hh2, hw1], fun j hj => ?_⟩
  rw [hw1, hh1] at hp2
  have hj' : j < c.h.val * c.w.val := by rw [Nat.mul_comm]; exact hj
  rw [hp2 j hj', hp1 _ (by rw [Nat.mul_comm]; exact rot_lt hj'), rot_rot hj']

/-- P6: four turns give back the canvas, for any canvas. -/
theorem rotate90_four (c : Canvas) :
    (do let c1 ← rotate90 c; let c2 ← rotate90 c1; let c3 ← rotate90 c2; rotate90 c3) = ok c := by
  by_cases hc : Valid c
  · obtain ⟨c1, c2, e1, e2, hv2, hw2, hh2, hp2⟩ := rotate90_twice c hc
    obtain ⟨c3, c4, e3, e4, hv4, hw4, hh4, hp4⟩ := rotate90_twice c2 hv2
    simp only [e1, e2, e3, e4, bind_ok, ok.injEq]
    apply canvas_ext (by rw [hw4, hw2]) (by rw [hh4, hh2])
    obtain ⟨-, -, -, -, hl4⟩ := hv4
    obtain ⟨-, -, -, -, hl⟩ := hc
    rw [hw2, hh2] at hp4
    have hl4' : c4.px.val.length = c.w.val * c.h.val := by rw [hl4, hw4, hh4, hw2, hh2]
    apply list_ext! _ _ (by rw [hl4', hl])
    intro j hj
    rw [hl4'] at hj
    rw [hp4 j hj, hp2 _ (rot2_lt hj), rot2_rot2 hj]
  · simp [rotate90_invalid c hc]

/-- P6: one turn of a valid canvas swaps its sides. The result is valid. -/
theorem rotate90_swap (c : Canvas) (hc : Valid c) :
    rotate90 c ⦃ r => r.w = c.h ∧ r.h = c.w ∧ Valid r ⦄ := by
  apply WP.spec_mono (rotate90_valid_spec c hc)
  rintro r ⟨h1, h2, h3, -⟩
  exact ⟨h1, h2, h3⟩

/-- P10: `rotate90` returns `ok` for every canvas. -/
theorem rotate90_total (c : Canvas) : rotate90 c ⦃ _ => True ⦄ := by
  by_cases hc : Valid c
  · exact WP.spec_mono (rotate90_valid_spec c hc) (fun _ _ => trivial)
  · rw [rotate90_invalid c hc]; simp

end Logbook.Paint
