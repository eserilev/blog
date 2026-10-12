import Logbook.Paint.Basic
open Aeneas Aeneas.Std Result logbook_paint

/-! Rectangles (spec 7.6): `fill_rect`, P4 (`rect_outline`), P8 (`color_replace`), and
`line`. P10 (no panic) is built in. -/

namespace Logbook.Paint

/-- Pixel `k` of a `w`-wide picture is in columns `x0..x1` and rows `y0..y1`. -/
def InBox (w x0 x1 y0 y1 k : Nat) : Prop :=
  x0 ≤ k % w ∧ k % w < x1 ∧ y0 ≤ k / w ∧ k / w < y1

instance (w x0 x1 y0 y1 k : Nat) : Decidable (InBox w x0 x1 y0 y1 k) := by
  unfold InBox; infer_instance

/-- Pixel `k` is in the rectangle `x, y, w, h` after `clamp_coord` and `clamp_span`.
`clip_rect` keeps exactly these pixels of the canvas (P9). -/
def InClip (cw : Nat) (x y w h : Int) (k : Nat) : Prop :=
  clampC x ≤ (k % cw : Nat) ∧ ((k % cw : Nat) : Int) < clampC x + clampS w ∧
    clampC y ≤ (k / cw : Nat) ∧ ((k / cw : Nat) : Int) < clampC y + clampS h

instance (cw : Nat) (x y w h : Int) (k : Nat) : Decidable (InClip cw x y w h k) := by
  unfold InClip; infer_instance

theorem idx_div_mod {w y x : Nat} (hx : x < w) : (y * w + x) / w = y ∧ (y * w + x) % w = x :=
  div_mod_of_lt hx

/-- In a valid canvas, the rows of `k < w * h` are below `h`. -/
theorem div_lt_h {w h k : Nat} (hk : k < w * h) : k / w < h := by
  exact Nat.div_lt_of_lt_mul hk

/-! `fill_rect`. -/

theorem fill_rect_inner_spec (w : Std.Usize) (v : alloc.vec.Vec Std.U32) (color : Std.U32)
    (xx i1 yy xx1 : Std.Usize) (hw : 0 < w.val) (hx : xx.val + i1.val ≤ w.val)
    (hy : yy.val * w.val + w.val ≤ v.val.length) (hlen : v.val.length ≤ 4096 * 4096)
    (h2 : xx1.val ≤ xx.val + i1.val) :
    fill_rect_loop0_loop0 w v color xx i1 yy xx1 ⦃ r => r.val.length = v.val.length ∧
      ∀ k < v.val.length, r.val[k]! =
        if InBox w.val xx1.val (xx.val + i1.val) yy.val (yy.val + 1) k then color else v.val[k]! ⦄ := by
  obtain ⟨P, hP⟩ : ∃ P, yy.val * w.val = P := ⟨_, rfl⟩
  rw [hP] at hy
  unfold fill_rect_loop0_loop0
  apply loop.spec_decr_nat
    (measure := fun (p : alloc.vec.Vec Std.U32 × Std.Usize) => xx.val + i1.val - p.2.val)
    (inv := fun p => xx1.val ≤ p.2.val ∧ p.2.val ≤ xx.val + i1.val ∧
      p.1.val.length = v.val.length ∧ ∀ k < v.val.length, p.1.val[k]! =
        if InBox w.val xx1.val p.2.val yy.val (yy.val + 1) k then color else v.val[k]!)
  · rintro ⟨u, c⟩ ⟨hc1, hc2, hul, hu⟩
    simp only at hc1 hc2 hul hu
    unfold fill_rect_loop0_loop0.body
    step*
    · refine ⟨by scalar_tac, by scalar_tac, by simp [*], fun k hk => ?_, by scalar_tac⟩
      have hcw : c.val < w.val := by scalar_tac
      obtain ⟨hd, hm⟩ := idx_div_mod (w := w.val) (y := yy.val) (x := c.val) hcw
      rw [hP] at hd hm
      simp only [index_mut_back_post, alloc.vec.Vec.set_val_eq, i4_post, i3_post, hP]
      by_cases hk' : k = P + c.val
      · subst hk'
        rw [set_eq _ _ (by omega)]
        unfold InBox; simp only [hd, hm]; split_ifs with hb <;> first | rfl | omega
      · rw [set_ne _ _ (Ne.symm hk'), hu k hk]
        unfold InBox
        have : ¬ (k / w.val = yy.val ∧ k % w.val = c.val) := by
          rintro ⟨h1, h2⟩; exact hk' (hP ▸ eq_of_div_mod h1 h2)
        split_ifs <;> first | rfl | omega
    · refine ⟨hul, fun k hk => ?_⟩
      rw [hu k hk]
      have : c.val = xx.val + i1.val := by scalar_tac
      rw [this]
  · refine ⟨le_refl _, h2, rfl, fun k _ => ?_⟩
    simp only [InBox]; split_ifs <;> first | rfl | omega

theorem fill_rect_outer_spec (w : Std.Usize) (v : alloc.vec.Vec Std.U32) (color : Std.U32)
    (xx yy i1 i2 yy1 : Std.Usize) (hw : 0 < w.val) (hx : xx.val + i1.val ≤ w.val)
    (hy : (yy.val + i2.val) * w.val ≤ v.val.length) (hlen : v.val.length ≤ 4096 * 4096)
    (h2 : yy1.val ≤ yy.val + i2.val) :
    fill_rect_loop0 w v color xx yy i1 i2 yy1 ⦃ r => r.val.length = v.val.length ∧
      ∀ k < v.val.length, r.val[k]! =
        if InBox w.val xx.val (xx.val + i1.val) yy1.val (yy.val + i2.val) k then color
        else v.val[k]! ⦄ := by
  have hyb : yy.val + i2.val ≤ 4096 * 4096 :=
    le_trans (Nat.le_mul_of_pos_right _ hw) (le_trans hy hlen)
  unfold fill_rect_loop0
  apply loop.spec_decr_nat
    (measure := fun (p : alloc.vec.Vec Std.U32 × Std.Usize) => yy.val + i2.val - p.2.val)
    (inv := fun p => yy1.val ≤ p.2.val ∧ p.2.val ≤ yy.val + i2.val ∧
      p.1.val.length = v.val.length ∧ ∀ k < v.val.length, p.1.val[k]! =
        if InBox w.val xx.val (xx.val + i1.val) yy1.val p.2.val k then color else v.val[k]!)
  · rintro ⟨u, c⟩ ⟨hc1, hc2, hul, hu⟩
    simp only at hc1 hc2 hul hu
    unfold fill_rect_loop0.body
    step as ⟨i3, hi3⟩
    split
    · have hrow : c.val * w.val + w.val ≤ u.val.length := by
        have : (c.val + 1) * w.val ≤ (yy.val + i2.val) * w.val :=
          Nat.mul_le_mul_right _ (by scalar_tac)
        rw [Nat.add_mul, Nat.one_mul] at this
        omega
      have := fill_rect_inner_spec w u color xx i1 c xx hw hx hrow (by omega) (by omega)
      step*
      refine ⟨by scalar_tac, by scalar_tac, by omega, fun k hk => ?_, by scalar_tac⟩
      rw [v1_post1 k (by omega), hu k hk]
      simp only [InBox]
      split_ifs <;> first | rfl | omega
    · simp only [WP.spec_ok]
      refine ⟨hul, fun k hk => ?_⟩
      rw [hu k hk]
      have : c.val = yy.val + i2.val := by scalar_tac
      rw [this]
  · refine ⟨le_refl _, h2, rfl, fun k _ => ?_⟩
    simp only [InBox]; split_ifs <;> first | rfl | omega

/-- `InBox` of the clipped rectangle is `InClip`, for the pixels of a valid canvas. -/
theorem inBox_clip {c : Canvas} (hc : Valid c) {x y w h : Std.I32} {r : Rect}
    (hr : ∀ i : Nat, (r.x.val ≤ i ∧ i < r.x.val + r.w.val) ↔
        (clampC x.val ≤ i ∧ i < clampC x.val + clampS w.val ∧ i < c.w.val ∧ i < 4096))
    (hr' : ∀ i : Nat, (r.y.val ≤ i ∧ i < r.y.val + r.h.val) ↔
        (clampC y.val ≤ i ∧ i < clampC y.val + clampS h.val ∧ i < c.h.val ∧ i < 4096))
    {k : Nat} (hk : k < c.px.val.length) :
    InBox c.w.val r.x.val (r.x.val + r.w.val) r.y.val (r.y.val + r.h.val) k ↔
      InClip c.w.val x.val y.val w.val h.val k := by
  obtain ⟨hw1, -, hw4, hh4, hl⟩ := hc
  have hm := Nat.mod_lt k (by omega : 0 < c.w.val)
  have hd : k / c.w.val < c.h.val := div_lt_h (by rw [← hl]; exact hk)
  have h1 := hr (k % c.w.val)
  have h2 := hr' (k / c.w.val)
  unfold InBox InClip
  constructor
  · rintro ⟨a, b, d, e⟩
    have := h1.mp ⟨a, b⟩
    have := h2.mp ⟨d, e⟩
    omega
  · rintro ⟨a, b, d, e⟩
    have := h1.mpr ⟨a, b, by omega, by omega⟩
    have := h2.mpr ⟨d, e, by omega, by omega⟩
    omega

/-- `fill_rect` sets the pixels of the clipped rectangle to `color`. Other pixels stay.
A canvas that is not valid does not change (P10). -/
@[step]
theorem fill_rect_spec (c : Canvas) (x y w h : Std.I32) (color : Std.U32) :
    fill_rect c x y w h color ⦃ r => r.w = c.w ∧ r.h = c.h ∧
      r.px.val.length = c.px.val.length ∧
      (Valid c → ∀ k < c.px.val.length, r.px.val[k]! =
        if InClip c.w.val x.val y.val w.val h.val k then color else c.px.val[k]!) ∧
      (¬ Valid c → r = c) ⦄ := by
  unfold fill_rect
  step*
  · have hc : Valid c := b_post.mp (by assumption)
    obtain ⟨hw1, hh1, hw4, hh4, hl⟩ := id hc
    have hy : (r.y.val + r.h.val) * c.w.val ≤ c.px.val.length := by
      rw [hl, Nat.mul_comm c.w.val]; exact Nat.mul_le_mul_right _ (by omega)
    have := fill_rect_outer_spec c.w c.px color r.x r.y r.w r.h r.y (by omega) (by omega) hy
      (by rw [hl]; exact Nat.mul_le_mul hw4 hh4) (by omega)
    step*
    refine ⟨v_post, fun _ k hk => ?_, fun h => absurd hc h⟩
    rw [v_post1 k hk]
    have := inBox_clip hc r_post4 r_post5 hk
    simp only [this]

/-! `color_replace` (P8). -/

theorem color_replace_inner_spec (w : Std.Usize) (v : alloc.vec.Vec Std.U32) (f t : Std.U32)
    (xx i1 yy xx1 : Std.Usize) (hw : 0 < w.val) (hx : xx.val + i1.val ≤ w.val)
    (hy : yy.val * w.val + w.val ≤ v.val.length) (hlen : v.val.length ≤ 4096 * 4096)
    (h2 : xx1.val ≤ xx.val + i1.val) :
    color_replace_loop0_loop0 w v f t xx i1 yy xx1 ⦃ r => r.val.length = v.val.length ∧
      ∀ k < v.val.length, r.val[k]! =
        if InBox w.val xx1.val (xx.val + i1.val) yy.val (yy.val + 1) k ∧ v.val[k]! = f then t else v.val[k]! ⦄ := by
  obtain ⟨P, hP⟩ : ∃ P, yy.val * w.val = P := ⟨_, rfl⟩
  rw [hP] at hy
  unfold color_replace_loop0_loop0
  apply loop.spec_decr_nat
    (measure := fun (p : alloc.vec.Vec Std.U32 × Std.Usize) => xx.val + i1.val - p.2.val)
    (inv := fun p => xx1.val ≤ p.2.val ∧ p.2.val ≤ xx.val + i1.val ∧
      p.1.val.length = v.val.length ∧ ∀ k < v.val.length, p.1.val[k]! =
        if InBox w.val xx1.val p.2.val yy.val (yy.val + 1) k ∧ v.val[k]! = f then t else v.val[k]!)
  · rintro ⟨u, c⟩ ⟨hc1, hc2, hul, hu⟩
    simp only at hc1 hc2 hul hu
    unfold color_replace_loop0_loop0.body
    step as ⟨i2, hi2⟩
    split
    · have hcw : c.val < w.val := by scalar_tac
      obtain ⟨hd, hm⟩ := idx_div_mod (w := w.val) (y := yy.val) (x := c.val) hcw
      rw [hP] at hd hm
      have hidx : P + c.val < v.val.length := by omega
      -- The pixel at the index is still the original one.
      have hold : u.val[P + c.val]! = v.val[P + c.val]! := by
        rw [hu _ hidx]; unfold InBox; rw [if_neg (by omega)]
      step*
      have hi5 : i5 = v.val[P + c.val]! := by
        rw [i5_post, ← hold, getElem!_pos u.val (P + c.val) (by omega)]
        simp only [i4_post, i3_post, hP]
      split
      · rename_i hf
        step*
        obtain ⟨_, back⟩ := x
        simp only at x_post1
        step*
        have hi4 : i4.val = P + c.val := by rw [i4_post, i3_post, hP]
        refine ⟨by scalar_tac, by scalar_tac, by simp [*], fun k hk => ?_, by scalar_tac⟩
        rw [x_post1, alloc.vec.Vec.set_val_eq, hi4]
        by_cases hk' : k = P + c.val
        · rw [hk']
          rw [set_eq _ _ (by rw [hul]; exact hidx)]
          unfold InBox; simp only [hd, hm, ← hi5, hf, and_true]
          rw [if_pos (by scalar_tac)]
        · have : ¬ (k / w.val = yy.val ∧ k % w.val = c.val) := by
            rintro ⟨h1, h2⟩; exact hk' (hP ▸ eq_of_div_mod h1 h2)
          rw [set_ne _ _ (Ne.symm hk'), hu k hk]
          unfold InBox
          have e : (xx1.val ≤ k % w.val ∧ k % w.val < c.val ∧ yy.val ≤ k / w.val ∧
              k / w.val < yy.val + 1) ↔ (xx1.val ≤ k % w.val ∧ k % w.val < xx2.val ∧
              yy.val ≤ k / w.val ∧ k / w.val < yy.val + 1) := by
            constructor <;> intro h <;> scalar_tac
          simp only [e]; split_ifs <;> rfl
      · rename_i hf
        step*
        refine ⟨by scalar_tac, by scalar_tac, by simp [*], fun k hk => ?_, by scalar_tac⟩
        rw [hu k hk]
        by_cases hk' : k = P + c.val
        · rw [hk']
          unfold InBox; simp only [hd, hm, ← hi5]
          rw [if_neg (by omega), if_neg (by intro h; exact hf h.2)]
        · have : ¬ (k / w.val = yy.val ∧ k % w.val = c.val) := by
            rintro ⟨h1, h2⟩; exact hk' (hP ▸ eq_of_div_mod h1 h2)
          unfold InBox
          have e : (xx1.val ≤ k % w.val ∧ k % w.val < c.val ∧ yy.val ≤ k / w.val ∧
              k / w.val < yy.val + 1) ↔ (xx1.val ≤ k % w.val ∧ k % w.val < xx2.val ∧
              yy.val ≤ k / w.val ∧ k / w.val < yy.val + 1) := by
            constructor <;> intro h <;> scalar_tac
          simp only [e]; split_ifs <;> rfl
    · simp only [WP.spec_ok]
      refine ⟨hul, fun k hk => ?_⟩
      rw [hu k hk]
      have : c.val = xx.val + i1.val := by scalar_tac
      rw [this]
  · refine ⟨le_refl _, h2, rfl, fun k _ => ?_⟩
    simp only [InBox]; split_ifs <;> first | rfl | omega

/-- One more row of `color_replace`: the new row `A` and the old rows `B` do not meet,
and together they are `C`. -/
theorem replace_step {α : Type} [DecidableEq α] {A B C : Prop} [Decidable A] [Decidable B] [Decidable C] {v f t : α}
    (hAB : ¬ (A ∧ B)) (hC : C ↔ A ∨ B) :
    (if A ∧ (if B ∧ v = f then t else v) = f then t else (if B ∧ v = f then t else v)) =
      if C ∧ v = f then t else v := by
  by_cases hA : A <;> by_cases hB : B <;> by_cases hv : v = f <;> simp_all

theorem color_replace_outer_spec (w : Std.Usize) (v : alloc.vec.Vec Std.U32) (f t : Std.U32)
    (xx yy i1 i2 yy1 : Std.Usize) (hw : 0 < w.val) (hx : xx.val + i1.val ≤ w.val)
    (hy : (yy.val + i2.val) * w.val ≤ v.val.length) (hlen : v.val.length ≤ 4096 * 4096)
    (h2 : yy1.val ≤ yy.val + i2.val) :
    color_replace_loop0 w v f t xx yy i1 i2 yy1 ⦃ r => r.val.length = v.val.length ∧
      ∀ k < v.val.length, r.val[k]! =
        if InBox w.val xx.val (xx.val + i1.val) yy1.val (yy.val + i2.val) k ∧ v.val[k]! = f then t
        else v.val[k]! ⦄ := by
  have hyb : yy.val + i2.val ≤ 4096 * 4096 :=
    le_trans (Nat.le_mul_of_pos_right _ hw) (le_trans hy hlen)
  unfold color_replace_loop0
  apply loop.spec_decr_nat
    (measure := fun (p : alloc.vec.Vec Std.U32 × Std.Usize) => yy.val + i2.val - p.2.val)
    (inv := fun p => yy1.val ≤ p.2.val ∧ p.2.val ≤ yy.val + i2.val ∧
      p.1.val.length = v.val.length ∧ ∀ k < v.val.length, p.1.val[k]! =
        if InBox w.val xx.val (xx.val + i1.val) yy1.val p.2.val k ∧ v.val[k]! = f then t else v.val[k]!)
  · rintro ⟨u, c⟩ ⟨hc1, hc2, hul, hu⟩
    simp only at hc1 hc2 hul hu
    unfold color_replace_loop0.body
    step as ⟨i3, hi3⟩
    split
    · have hrow : c.val * w.val + w.val ≤ u.val.length := by
        have : (c.val + 1) * w.val ≤ (yy.val + i2.val) * w.val :=
          Nat.mul_le_mul_right _ (by scalar_tac)
        rw [Nat.add_mul, Nat.one_mul] at this
        omega
      have := color_replace_inner_spec w u f t xx i1 c xx hw hx hrow (by omega) (by omega)
      step*
      refine ⟨by scalar_tac, by scalar_tac, by omega, fun k hk => ?_, by scalar_tac⟩
      rw [v1_post1 k (by omega), hu k hk]
      apply replace_step
      · simp only [InBox]; omega
      · simp only [InBox]; constructor <;> intro h <;> scalar_tac
    · simp only [WP.spec_ok]
      refine ⟨hul, fun k hk => ?_⟩
      rw [hu k hk]
      have : c.val = yy.val + i2.val := by scalar_tac
      rw [this]
  · refine ⟨le_refl _, h2, rfl, fun k _ => ?_⟩
    simp only [InBox]; split_ifs <;> first | rfl | omega

/-- P8: `color_replace` changes a pixel only if it is in the clipped rectangle and has
color `from`. Then the pixel becomes `to`. A canvas that is not valid does not change (P10). -/
@[step]
theorem color_replace_spec (c : Canvas) (x y w h : Std.I32) (f t : Std.U32) :
    color_replace c x y w h f t ⦃ r => r.w = c.w ∧ r.h = c.h ∧
      r.px.val.length = c.px.val.length ∧
      (Valid c → ∀ k < c.px.val.length, r.px.val[k]! =
        if InClip c.w.val x.val y.val w.val h.val k ∧ c.px.val[k]! = f then t
        else c.px.val[k]!) ∧
      (¬ Valid c → r = c) ⦄ := by
  unfold color_replace
  step*
  · have hc : Valid c := b_post.mp (by assumption)
    obtain ⟨hw1, hh1, hw4, hh4, hl⟩ := id hc
    have hy : (r.y.val + r.h.val) * c.w.val ≤ c.px.val.length := by
      rw [hl, Nat.mul_comm c.w.val]; exact Nat.mul_le_mul_right _ (by omega)
    have := color_replace_outer_spec c.w c.px f t r.x r.y r.w r.h r.y (by omega) (by omega) hy
      (by rw [hl]; exact Nat.mul_le_mul hw4 hh4) (by omega)
    step*
    refine ⟨v_post, fun _ k hk => ?_, fun h => absurd hc h⟩
    rw [v_post1 k hk]
    have := inBox_clip hc r_post4 r_post5 hk
    simp only [this]

/-! `rect_outline` (P4). -/

theorem clampC_of {v : Int} (h1 : -65536 ≤ v) (h2 : v ≤ 65536) : clampC v = v := by
  unfold clampC; split_ifs <;> omega

theorem clampS_of {v : Int} (h1 : 0 ≤ v) (h2 : v ≤ 131072) : clampS v = v := by
  unfold clampS; split_ifs <;> omega

/-- A band at most 64 wide: clamping its start does not change which of the first
4096 rows (or columns) it covers. -/
theorem band_clamp {a t : Int} {i : Nat} (ht : 0 ≤ t ∧ t ≤ 64) (hi : i < 4096) :
    (clampC a ≤ i ∧ (i : Int) < clampC a + t) ↔ (a ≤ i ∧ (i : Int) < a + t) := by
  unfold clampC; split_ifs <;> omega

/-- Pixel `k` is on the border of the rectangle `x, y, w, h` drawn `lw` thick: inside the
rectangle, and less than `lw` from one of its edges. All values clamped as in the code. -/
def Outline (cw : Nat) (x y w h lw : Int) (k : Nat) : Prop :=
  let X := clampC x; let Y := clampC y; let W := clampS w; let H := clampS h
  let L := clampW lw; let col : Int := (k % cw : Nat); let row : Int := (k / cw : Nat)
  X ≤ col ∧ col < X + W ∧ Y ≤ row ∧ row < Y + H ∧
    (col < X + L ∨ X + W - L ≤ col ∨ row < Y + L ∨ Y + H - L ≤ row)

instance (cw : Nat) (x y w h lw : Int) (k : Nat) : Decidable (Outline cw x y w h lw k) := by
  unfold Outline; infer_instance

/-- The four bands of `rect_outline` cover exactly the `Outline` pixels. `tx` and `ty`
are the band sides: `lw`, or the side of the rectangle if that is smaller. -/
theorem outline_key {X Y W H L tx ty col row : Int} (hc : 0 ≤ col ∧ col < 4096)
    (hr : 0 ≤ row ∧ row < 4096) (hX : -65536 ≤ X ∧ X ≤ 65536) (hY : -65536 ≤ Y ∧ Y ≤ 65536)
    (hW : 0 < W ∧ W ≤ 131072) (hH : 0 < H ∧ H ≤ 131072) (hL : 1 ≤ L ∧ L ≤ 64)
    (htx : (tx = L ∧ L < W) ∨ (tx = W ∧ ¬ L < W)) (hty : (ty = L ∧ L < H) ∨ (ty = H ∧ ¬ L < H)) :
    ((clampC X ≤ col ∧ col < clampC X + clampS W ∧ clampC Y ≤ row ∧ row < clampC Y + clampS ty) ∨
     (clampC X ≤ col ∧ col < clampC X + clampS W ∧
       clampC (Y + H - ty) ≤ row ∧ row < clampC (Y + H - ty) + clampS ty) ∨
     (clampC X ≤ col ∧ col < clampC X + clampS tx ∧ clampC Y ≤ row ∧ row < clampC Y + clampS H) ∨
     (clampC (X + W - tx) ≤ col ∧ col < clampC (X + W - tx) + clampS tx ∧
       clampC Y ≤ row ∧ row < clampC Y + clampS H)) ↔
    (X ≤ col ∧ col < X + W ∧ Y ≤ row ∧ row < Y + H ∧
      (col < X + L ∨ X + W - L ≤ col ∨ row < Y + L ∨ Y + H - L ≤ row)) := by
  have htx' : 0 ≤ tx ∧ tx ≤ 64 := by omega
  have hty' : 0 ≤ ty ∧ ty ≤ 64 := by omega
  rw [clampC_of hX.1 hX.2, clampC_of hY.1 hY.2, clampS_of (by omega) (by omega),
    clampS_of (v := ty) (by omega) (by omega), clampS_of (v := tx) (by omega) (by omega),
    clampS_of (v := H) (by omega) (by omega)]
  have hb := band_clamp (a := Y + H - ty) (i := row.toNat) hty' (by omega)
  have hb' := band_clamp (a := X + W - tx) (i := col.toNat) htx' (by omega)
  rw [Int.toNat_of_nonneg hr.1] at hb
  rw [Int.toNat_of_nonneg hc.1] at hb'
  generalize clampC (Y + H - ty) = A at hb ⊢
  generalize clampC (X + W - tx) = B at hb' ⊢
  constructor
  · intro h
    rcases h with h | h | h | h
    · omega
    · have := hb.mp ⟨h.2.2.1, h.2.2.2⟩; omega
    · omega
    · have := hb'.mp ⟨h.1, h.2.1⟩; omega
  · intro h
    by_cases h1 : row < Y + ty
    · omega
    by_cases h2 : Y + H - ty ≤ row
    · have := hb.mpr ⟨h2, by omega⟩; omega
    by_cases h3 : col < X + tx
    · omega
    have := hb'.mpr ⟨by omega, by omega⟩; omega

/-- `if` chains with one value: four tests in a row are one test of the `∨`. -/
theorem ite4 {α} {p q r s : Prop} [Decidable p] [Decidable q] [Decidable r] [Decidable s]
    (a b : α) : (if s then a else if r then a else if q then a else if p then a else b) =
      if (p ∨ q ∨ r ∨ s) then a else b := by
  by_cases hp : p <;> by_cases hq : q <;> by_cases hr : r <;> by_cases hs : s <;> simp [*]

set_option maxHeartbeats 1000000 in
/-- P4: `rect_outline` sets exactly the border pixels (see `Outline`) to `color`. Other
pixels stay. A canvas that is not valid does not change (P10). -/
@[step]
theorem rect_outline_spec (c : Canvas) (x y w h lw : Std.I32) (color : Std.U32) :
    rect_outline c x y w h lw color ⦃ r => r.w = c.w ∧ r.h = c.h ∧
      r.px.val.length = c.px.val.length ∧
      (Valid c → ∀ k < c.px.val.length, r.px.val[k]! =
        if Outline c.w.val x.val y.val w.val h.val lw.val k then color else c.px.val[k]!) ∧
      (¬ Valid c → r = c) ⦄ := by
  unfold rect_outline
  have := clampC_bounds x.val
  have := clampC_bounds y.val
  have := clampS_bounds w.val
  have := clampS_bounds h.val
  have := clampW_bounds lw.val
  step*
  · refine ⟨fun _ k _ => ?_, fun _ => trivial⟩
    have : ¬ Outline c.w.val x.val y.val w.val h.val lw.val k := by
      unfold Outline; scalar_tac
    simp [this]
  · refine ⟨fun _ k _ => ?_, fun _ => trivial⟩
    have : ¬ Outline c.w.val x.val y.val w.val h.val lw.val k := by
      unfold Outline; scalar_tac
    simp [this]
  split <;> split <;> step*
  all_goals
    have v1 := valid_of_same c1_post c1_post1 c1_post2
    have v2 := valid_of_same c2_post c2_post1 c2_post2
    have v3 := valid_of_same c3_post c3_post1 c3_post2
    refine ⟨by rw [r_post, c3_post, c2_post, c1_post], by rw [r_post1, c3_post1, c2_post1, c1_post1],
      by rw [r_post2, c3_post2, c2_post2, c1_post2], fun hc k hk => ?_, fun hc => ?_⟩
    · have hv1 := v1.mpr hc
      have hv2 := v2.mpr hv1
      have hv3 := v3.mpr hv2
      rw [r_post3 hv3 k (by rw [c3_post2, c2_post2, c1_post2]; exact hk),
        c3_post3 hv2 k (by rw [c2_post2, c1_post2]; exact hk),
        c2_post3 hv1 k (by rw [c1_post2]; exact hk), c1_post3 hc k hk, ite4]
      refine if_congr ?_ rfl rfl
      simp only [c3_post, c2_post, c1_post]
      obtain ⟨hw1, -, hw4, hh4, hl⟩ := hc
      have hm := Nat.mod_lt k (by omega : 0 < c.w.val)
      have hd : k / c.w.val < c.h.val := div_lt_h (by rw [← hl]; exact hk)
      unfold InClip Outline
      rw [← x1_post, ← y1_post, ← w1_post, ← h1_post, ← lw1_post, i3_post, i2_post, i1_post, i_post]
      have hcol : (0 : Int) ≤ ((k % c.w.val : Nat) : Int) ∧ ((k % c.w.val : Nat) : Int) < 4096 :=
        ⟨Int.natCast_nonneg _, by exact_mod_cast (show k % c.w.val < 4096 by omega)⟩
      have hrow : (0 : Int) ≤ ((k / c.w.val : Nat) : Int) ∧ ((k / c.w.val : Nat) : Int) < 4096 :=
        ⟨Int.natCast_nonneg _, by exact_mod_cast (show k / c.w.val < 4096 by omega)⟩
      have hX : -65536 ≤ x1.val ∧ x1.val ≤ 65536 := by rw [x1_post]; exact clampC_bounds _
      have hY : -65536 ≤ y1.val ∧ y1.val ≤ 65536 := by rw [y1_post]; exact clampC_bounds _
      have hW : 0 < w1.val ∧ w1.val ≤ 131072 := by scalar_tac
      have hH : 0 < h1.val ∧ h1.val ≤ 131072 := by scalar_tac
      have hL : 1 ≤ lw1.val ∧ lw1.val ≤ 64 := by rw [lw1_post]; exact clampW_bounds _
      apply outline_key hcol hrow hX hY hW hH hL
      · first | exact Or.inl ⟨rfl, by scalar_tac⟩ | exact Or.inr ⟨rfl, by scalar_tac⟩
      · first | exact Or.inl ⟨rfl, by scalar_tac⟩ | exact Or.inr ⟨rfl, by scalar_tac⟩
    · have h1 := c1_post4 hc
      have h2 := c2_post4 (fun h => hc (v1.mp h))
      have h3 := c3_post4 (fun h => hc (v1.mp (v2.mp h)))
      have h4 := r_post4 (fun h => hc (v1.mp (v2.mp (v3.mp h))))
      rw [h4, h3, h2, h1]

/-- P4: `rect_outline` changes only pixels of the border (see `Outline`): inside the
rectangle and less than `lw` from one of its edges. -/
theorem rect_outline_only (c : Canvas) (x y w h lw : Std.I32) (color : Std.U32) :
    rect_outline c x y w h lw color ⦃ r => ∀ k < c.px.val.length, r.px.val[k]! ≠ c.px.val[k]! →
      Outline c.w.val x.val y.val w.val h.val lw.val k ⦄ := by
  apply WP.spec_mono (rect_outline_spec c x y w h lw color)
  rintro r ⟨-, -, -, hp, hi⟩ k hk hne
  by_cases hc : Valid c
  · rw [hp hc k hk] at hne
    by_contra ho
    rw [if_neg ho] at hne
    exact hne rfl
  · rw [hi hc] at hne; exact absurd rfl hne

end Logbook.Paint
