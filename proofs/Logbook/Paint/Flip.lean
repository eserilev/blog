import Logbook.Paint.Basic
open Aeneas Aeneas.Std Result logbook_paint

/-! Flips and Invert Colors (spec 7.6): P5 and P7, with P10 (no panic) built in. -/

namespace Logbook.Paint

/-! Invert Colors (P7). -/

/-- The inverted pixel: red, green, and blue flip. -/
def inv (p : Std.U32) : Std.U32 := p ^^^ 16777215#u32

theorem inv_inv (p : Std.U32) : inv (inv p) = p := by
  unfold inv
  apply UScalar.eq_of_val_eq
  simp only [UScalar.val_xor, Nat.xor_assoc, Nat.xor_self, Nat.xor_zero]

/-- The alpha byte is the value divided by `2 ^ 24`. -/
theorem inv_alpha (p : Std.U32) : (inv p).val / 2 ^ 24 = p.val / 2 ^ 24 := by
  unfold inv
  simp only [UScalar.val_xor, ← Nat.shiftRight_eq_div_pow, Nat.shiftRight_xor_distrib]
  simp

theorem invert_loop_spec (c0 c : Canvas) (i : Std.Usize) (hl : c.px.val.length = c0.px.val.length)
    (hw : c.w = c0.w) (hh : c.h = c0.h) (hi : i.val ≤ c0.px.val.length)
    (hp : ∀ k < c0.px.val.length, c.px.val[k]! = if k < i.val then inv c0.px.val[k]! else c0.px.val[k]!) :
    invert_loop c i ⦃ r => r.1 = c0.w ∧ r.2.1 = c0.h ∧ r.2.2.val.length = c0.px.val.length ∧
      ∀ k < c0.px.val.length, r.2.2.val[k]! = inv c0.px.val[k]! ⦄ := by
  unfold invert_loop
  apply loop.spec_decr_nat
    (measure := fun (p : Canvas × Std.Usize) => c0.px.val.length - p.2.val)
    (inv := fun p => p.1.px.val.length = c0.px.val.length ∧ p.1.w = c0.w ∧ p.1.h = c0.h ∧
      p.2.val ≤ c0.px.val.length ∧ ∀ k < c0.px.val.length,
        p.1.px.val[k]! = if k < p.2.val then inv c0.px.val[k]! else c0.px.val[k]!)
  · rintro ⟨d, j⟩ ⟨hl, hw, hh, hj, hp⟩
    simp only at hl hw hh hj hp
    unfold invert_loop.body
    step*
    · have hi3 : i3 = inv c0.px.val[j.val]! := by
        have := hp j.val (by scalar_tac)
        rw [if_neg (by omega)] at this
        apply UScalar.eq_of_val_eq
        rw [i3_post, inv, ← this, getElem!_pos d.px.val j.val (by scalar_tac), ← i2_post, RGB_MASK]
      refine ⟨by simp [*], hw, hh, by scalar_tac, fun k hk => ?_, by scalar_tac⟩
      rw [index_mut_back_post, alloc.vec.Vec.set_val_eq]
      by_cases hkj : k = j.val
      · rw [hkj, set_eq _ _ (by omega), if_pos (by omega), hi3]
      · rw [set_ne _ _ (Ne.symm hkj), hp k hk]
        split_ifs <;> first | rfl | omega
  · exact ⟨hl, hw, hh, hi, hp⟩

/-- `invert` flips red, green, and blue of each pixel. A canvas that is not valid does
not change (P10). -/
@[step]
theorem invert_spec (c : Canvas) :
    invert c ⦃ r => r.w = c.w ∧ r.h = c.h ∧ r.px.val.length = c.px.val.length ∧
      (Valid c → ∀ k < c.px.val.length, r.px.val[k]! = inv c.px.val[k]!) ∧
      (¬ Valid c → r = c) ⦄ := by
  unfold invert
  step*
  · have := invert_loop_spec c c 0#usize rfl rfl rfl (by simp) (fun k _ => by simp)
    step*

/-- P7: Invert Colors twice gives the same canvas, for any canvas. -/
theorem invert_twice (c : Canvas) : (do let c1 ← invert c; invert c1) = ok c := by
  obtain ⟨c1, e1, hw1, hh1, hl1, hp1, hi1⟩ := spec_ok_exists (invert_spec c)
  obtain ⟨c2, e2, hw2, hh2, hl2, hp2, hi2⟩ := spec_ok_exists (invert_spec c1)
  simp only [e1, e2, bind_ok, ok.injEq]
  by_cases hc : Valid c
  · have hv1 : Valid c1 := (valid_of_same hw1 hh1 hl1).mpr hc
    apply canvas_ext (by rw [hw2, hw1]) (by rw [hh2, hh1])
    apply list_ext! _ _ (by rw [hl2, hl1])
    intro k hk
    rw [hl2, hl1] at hk
    rw [hp2 hv1 k (by rw [hl1]; exact hk), hp1 hc k hk, inv_inv]
  · have := hi1 hc
    subst this
    exact hi2 hc

/-- P7: Invert Colors keeps the alpha byte (the value divided by `2 ^ 24`). -/
theorem invert_alpha (c : Canvas) (hc : Valid c) :
    invert c ⦃ r => ∀ k < c.px.val.length, (r.px.val[k]!).val / 2 ^ 24 = (c.px.val[k]!).val / 2 ^ 24 ⦄ := by
  apply WP.spec_mono (invert_spec c)
  rintro r ⟨-, -, -, hp, -⟩ k hk
  rw [hp hc k hk, inv_alpha]

/-! Flip horizontal (P5). -/

/-- The index of the pixel that flip horizontal moves to index `k`, in a `w`-wide picture. -/
def mirH (w k : Nat) : Nat := k / w * w + (w - 1 - k % w)

theorem mirH_div_mod {w k : Nat} (hw : 0 < w) : mirH w k / w = k / w ∧ mirH w k % w = w - 1 - k % w := by
  unfold mirH
  have := Nat.mod_lt k hw
  exact div_mod_of_lt (by omega)

theorem mirH_mirH {w k : Nat} (hw : 0 < w) : mirH w (mirH w k) = k := by
  obtain ⟨h1, h2⟩ := mirH_div_mod (k := k) hw
  have := Nat.mod_lt k hw
  rw [show mirH w (mirH w k) = mirH w k / w * w + (w - 1 - mirH w k % w) from rfl, h1, h2,
    show w - 1 - (w - 1 - k % w) = k % w by omega, Nat.mul_comm, Nat.div_add_mod]

theorem mirH_lt {w k n : Nat} (hw : 0 < w) (hk : k < n * w) : mirH w k < n * w := by
  obtain ⟨h1, h2⟩ := mirH_div_mod (k := k) hw
  have hdm := Nat.div_add_mod (mirH w k) w
  have h3 : k / w < n := Nat.div_lt_of_lt_mul (by rw [Nat.mul_comm]; exact hk)
  have h4 : (k / w + 1) * w ≤ n * w := Nat.mul_le_mul_right _ h3
  rw [Nat.add_mul, Nat.one_mul] at h4
  have := Nat.mod_lt k hw
  rw [h1, h2] at hdm
  have : w * (k / w) = k / w * w := Nat.mul_comm _ _
  omega

theorem flip_h_inner_spec (w y : Nat) (v : alloc.vec.Vec Std.U32) (row a b : Std.Usize)
    (hw : 0 < w) (hrow : row.val = y * w) (hy : y * w + w ≤ v.val.length)
    (hlen : v.val.length ≤ 4096 * 4096) (hab : a.val + b.val = w - 1) (ha : a.val ≤ b.val + 1)
    (u : alloc.vec.Vec Std.U32) (hul : u.val.length = v.val.length)
    (hu : ∀ k < v.val.length, u.val[k]! =
      if k / w = y ∧ (k % w < a.val ∨ b.val < k % w) then v.val[mirH w k]! else v.val[k]!) :
    flip_h_loop0_loop0 u row a b ⦃ r => r.val.length = v.val.length ∧
      ∀ k < v.val.length, r.val[k]! = if k / w = y then v.val[mirH w k]! else v.val[k]! ⦄ := by
  obtain ⟨P, hP⟩ : ∃ P, y * w = P := ⟨_, rfl⟩
  rw [hP] at hy hrow
  unfold flip_h_loop0_loop0
  apply loop.spec_decr_nat
    (measure := fun (p : alloc.vec.Vec Std.U32 × Std.Usize × Std.Usize) => w - p.2.1.val)
    (inv := fun p => p.2.1.val + p.2.2.val = w - 1 ∧ p.2.1.val ≤ p.2.2.val + 1 ∧
      p.1.val.length = v.val.length ∧ ∀ k < v.val.length, p.1.val[k]! =
        if k / w = y ∧ (k % w < p.2.1.val ∨ p.2.2.val < k % w) then v.val[mirH w k]! else v.val[k]!)
  · rintro ⟨u, a, b⟩ ⟨hab, ha, hul, hu⟩
    simp only at hab ha hul hu
    unfold flip_h_loop0_loop0.body
    step*
    · simp only [index_mut_back_post]; scalar_tac
    · have hab' : a.val < b.val := by scalar_tac
      have hia : i.val = P + a.val := by rw [i_post, hrow]
      have hib : i1.val = P + b.val := by rw [i1_post, hrow]
      obtain ⟨hda, hma⟩ := div_mod_of_lt (w := w) (b := y) (a := a.val) (by scalar_tac)
      obtain ⟨hdb, hmb⟩ := div_mod_of_lt (w := w) (b := y) (a := b.val) (by scalar_tac)
      rw [hP] at hda hma hdb hmb
      have hla : P + a.val < v.val.length := by scalar_tac
      have hlb : P + b.val < v.val.length := by scalar_tac
      have mia : mirH w (P + a.val) = P + b.val := by
        show (P + a.val) / w * w + (w - 1 - (P + a.val) % w) = P + b.val
        rw [hda, hma, hP]; omega
      have mib : mirH w (P + b.val) = P + a.val := by
        show (P + b.val) / w * w + (w - 1 - (P + b.val) % w) = P + a.val
        rw [hdb, hmb, hP]; omega
      have ht : t = v.val[P + a.val]! := by
        have h1 : u.val[i.val]! = t := by
          rw [t_post]; exact getElem!_pos u.val i.val (by rw [hia, hul]; exact hla)
        rw [← h1, hia, hu _ hla, hda, hma, if_neg (by omega)]
      have hi2 : i2 = v.val[P + b.val]! := by
        have h1 : u.val[i1.val]! = i2 := by
          rw [i2_post]; exact getElem!_pos u.val i1.val (by rw [hib, hul]; exact hlb)
        rw [← h1, hib, hu _ hlb, hdb, hmb, if_neg (by omega)]
      refine ⟨by scalar_tac, by scalar_tac, by simp [*], fun k hk => ?_, by scalar_tac⟩
      rw [index_mut_back1_post, index_mut_back_post]
      simp only [alloc.vec.Vec.set_val_eq, hia, hib]
      by_cases hkb : k = P + b.val
      · rw [hkb, set_eq _ _ (by simp; omega), if_pos (by scalar_tac), mib, ht]
      by_cases hka : k = P + a.val
      · rw [hka, set_ne _ _ (by omega), set_eq _ _ (by omega), if_pos (by scalar_tac), mia, hi2]
      rw [set_ne _ _ (Ne.symm hkb), set_ne _ _ (Ne.symm hka), hu k hk]
      have : ¬ (k / w = y ∧ k % w = a.val) := by
        rintro ⟨h1, h2⟩; exact hka (by rw [← hP, ← h1, ← h2, Nat.mul_comm, Nat.div_add_mod])
      have : ¬ (k / w = y ∧ k % w = b.val) := by
        rintro ⟨h1, h2⟩; exact hkb (by rw [← hP, ← h1, ← h2, Nat.mul_comm, Nat.div_add_mod])
      have e : (k / w = y ∧ (k % w < a.val ∨ b.val < k % w)) ↔
          (k / w = y ∧ (k % w < a1.val ∨ b1.val < k % w)) := by
        constructor <;> intro h <;> scalar_tac
      simp only [e]
    · refine ⟨hul, fun k hk => ?_⟩
      rw [hu k hk]
      by_cases hky : k / w = y
      · by_cases hc : k % w < a.val ∨ b.val < k % w
        · rw [if_pos ⟨hky, hc⟩, if_pos hky]
        · rw [if_neg (fun h => hc h.2), if_pos hky]
          have hm : mirH w k = k := by
            have := Nat.div_add_mod k w
            show k / w * w + (w - 1 - k % w) = k
            have : w * (k / w) = k / w * w := Nat.mul_comm _ _
            scalar_tac
          rw [hm]
      · rw [if_neg (fun h => hky h.1), if_neg hky]
  · exact ⟨hab, ha, hul, hu⟩

theorem flip_h_outer_spec (c0 : Canvas) (hv : Valid c0) (c : Canvas) (y : Std.Usize)
    (hw : c.w = c0.w) (hh : c.h = c0.h) (hl : c.px.val.length = c0.px.val.length)
    (hy : y.val ≤ c0.h.val) (hp : ∀ k < c0.px.val.length, c.px.val[k]! =
      if k / c0.w.val < y.val then c0.px.val[mirH c0.w.val k]! else c0.px.val[k]!) :
    flip_h_loop0 c y ⦃ r => r.1 = c0.w ∧ r.2.1 = c0.h ∧ r.2.2.val.length = c0.px.val.length ∧
      ∀ k < c0.px.val.length, r.2.2.val[k]! = c0.px.val[mirH c0.w.val k]! ⦄ := by
  obtain ⟨hw1, hh1, hw4, hh4, hl0⟩ := hv
  unfold flip_h_loop0
  apply loop.spec_decr_nat
    (measure := fun (p : Canvas × Std.Usize) => c0.h.val - p.2.val)
    (inv := fun p => p.1.w = c0.w ∧ p.1.h = c0.h ∧ p.1.px.val.length = c0.px.val.length ∧
      p.2.val ≤ c0.h.val ∧ ∀ k < c0.px.val.length, p.1.px.val[k]! =
        if k / c0.w.val < p.2.val then c0.px.val[mirH c0.w.val k]! else c0.px.val[k]!)
  · rintro ⟨d, j⟩ ⟨hw, hh, hl, hj, hp⟩
    simp only at hw hh hl hj hp
    unfold flip_h_loop0.body
    step*
    · have hrow : (j.val * c0.w.val + c0.w.val ≤ d.px.val.length) := by
        have : (j.val + 1) * c0.w.val ≤ c0.h.val * c0.w.val :=
          Nat.mul_le_mul_right _ (by scalar_tac)
        rw [Nat.add_mul, Nat.one_mul] at this
        rw [hl, hl0, Nat.mul_comm c0.w.val]; exact this
      have := flip_h_inner_spec c0.w.val j.val d.px row 0#usize b (by omega)
        (by rw [row_post, hw]) hrow (by rw [hl, hl0]; exact Nat.mul_le_mul hw4 hh4)
        (by simp; scalar_tac) (by simp) d.px rfl (fun k _ => by
          have := Nat.mod_lt k (show 0 < c0.w.val by omega)
          rw [if_neg]; intro h; scalar_tac)
      step*
      refine ⟨hw, hh, by rw [v_post, hl], by scalar_tac, fun k hk => ?_, by scalar_tac⟩
      rw [v_post1 k (by omega)]
      have hmk := mirH_div_mod (k := k) (w := c0.w.val) (by omega)
      have hml : mirH c0.w.val k < c0.px.val.length := by
        rw [hl0, Nat.mul_comm]; exact mirH_lt (by omega) (by rw [Nat.mul_comm, ← hl0]; exact hk)
      by_cases hky : k / c0.w.val = j.val
      · rw [if_pos hky, hp _ hml, if_neg (by omega), if_pos (by scalar_tac)]
      · rw [if_neg hky, hp k hk]
        by_cases hlt : k / c0.w.val < j.val
        · rw [if_pos hlt, if_pos (by scalar_tac)]
        · rw [if_neg hlt, if_neg (by scalar_tac)]
    · refine ⟨hw, hh, hl, fun k hk => ?_⟩
      rw [hp k hk, if_pos]
      have : k / c0.w.val < c0.h.val := Nat.div_lt_of_lt_mul (by rw [← hl0]; exact hk)
      scalar_tac
  · exact ⟨hw, hh, hl, hy, hp⟩

/-- `flip_h` mirrors each row: pixel `k` of the result is pixel `mirH w k`. A canvas that
is not valid does not change (P10). -/
@[step]
theorem flip_h_spec (c : Canvas) :
    flip_h c ⦃ r => r.w = c.w ∧ r.h = c.h ∧ r.px.val.length = c.px.val.length ∧
      (Valid c → ∀ k < c.px.val.length, r.px.val[k]! = c.px.val[mirH c.w.val k]!) ∧
      (¬ Valid c → r = c) ⦄ := by
  unfold flip_h
  step*
  have hv : Valid c := b_post.mp (by assumption)
  have := flip_h_outer_spec c hv c 0#usize rfl rfl rfl (by simp) (fun k _ => by simp)
  step*

/-- P5: flip horizontal twice gives the same canvas, for any canvas. -/
theorem flip_h_twice (c : Canvas) : (do let c1 ← flip_h c; flip_h c1) = ok c := by
  obtain ⟨c1, e1, hw1, hh1, hl1, hp1, hi1⟩ := spec_ok_exists (flip_h_spec c)
  obtain ⟨c2, e2, hw2, hh2, hl2, hp2, hi2⟩ := spec_ok_exists (flip_h_spec c1)
  simp only [e1, e2, bind_ok, ok.injEq]
  by_cases hc : Valid c
  · have hv1 : Valid c1 := (valid_of_same hw1 hh1 hl1).mpr hc
    have hw : 0 < c.w.val := hc.1
    have hl := hc.2.2.2.2
    apply canvas_ext (by rw [hw2, hw1]) (by rw [hh2, hh1])
    apply list_ext! _ _ (by rw [hl2, hl1])
    intro k hk
    rw [hl2, hl1] at hk
    have hm : mirH c.w.val k < c.px.val.length := by
      rw [hl, Nat.mul_comm]; exact mirH_lt hw (by rw [Nat.mul_comm, ← hl]; exact hk)
    rw [hp2 hv1 k (by rw [hl1]; exact hk), hw1, hp1 hc _ hm, mirH_mirH hw]
  · have := hi1 hc
    subst this
    exact hi2 hc

/-! Flip vertical (P5). -/

/-- The index of the pixel that flip vertical moves to index `k`, in a `w × h` picture. -/
def mirV (w h k : Nat) : Nat := (h - 1 - k / w) * w + k % w

theorem mirV_div_mod {w h k : Nat} (hw : 0 < w) :
    mirV w h k / w = h - 1 - k / w ∧ mirV w h k % w = k % w := by
  unfold mirV
  exact div_mod_of_lt (Nat.mod_lt k hw)

theorem mirV_mirV {w h k : Nat} (hw : 0 < w) (hk : k < w * h) : mirV w h (mirV w h k) = k := by
  obtain ⟨h1, h2⟩ := mirV_div_mod (h := h) (k := k) hw
  have hq : k / w < h := Nat.div_lt_of_lt_mul hk
  have e : h - 1 - (h - 1 - k / w) = k / w := by
    generalize k / w = q at hq ⊢; omega
  rw [show mirV w h (mirV w h k) = (h - 1 - mirV w h k / w) * w + mirV w h k % w from rfl, h1, h2,
    e, Nat.mul_comm, Nat.div_add_mod]

theorem mirV_lt {w h k : Nat} (hw : 0 < w) (hk : k < w * h) : mirV w h k < w * h := by
  unfold mirV
  rw [Nat.mul_comm w h]
  have hq : k / w < h := Nat.div_lt_of_lt_mul hk
  have := idx_lt (a := k % w) (b := h - 1 - k / w) (h := h) (Nat.mod_lt k hw)
    (by generalize k / w = q at hq ⊢; omega)
  rwa [Nat.mul_comm w h] at this

theorem flip_v_inner_spec (w h : Nat) (v : alloc.vec.Vec Std.U32) (W a b x : Std.Usize)
    (hW : W.val = w) (hw : 0 < w) (hl : v.val.length = w * h) (hlen : v.val.length ≤ 4096 * 4096)
    (hab : a.val < b.val) (hbh : b.val < h) (hsum : a.val + b.val = h - 1) (hx : x.val ≤ w)
    (u : alloc.vec.Vec Std.U32) (hul : u.val.length = v.val.length)
    (hu : ∀ k < v.val.length, u.val[k]! =
      if (k / w = a.val ∨ k / w = b.val) ∧ k % w < x.val then v.val[mirV w h k]! else v.val[k]!) :
    flip_v_loop0_loop0 W u a b x ⦃ r => r.val.length = v.val.length ∧
      ∀ k < v.val.length, r.val[k]! =
        if k / w = a.val ∨ k / w = b.val then v.val[mirV w h k]! else v.val[k]! ⦄ := by
  obtain ⟨Pa, hPa⟩ : ∃ P, a.val * w = P := ⟨_, rfl⟩
  obtain ⟨Pb, hPb⟩ : ∃ P, b.val * w = P := ⟨_, rfl⟩
  have hb1 : (b.val + 1) * w ≤ h * w := Nat.mul_le_mul_right _ (by omega)
  have ha1 : (a.val + 1) * w ≤ b.val * w := Nat.mul_le_mul_right _ (by omega)
  rw [Nat.add_mul, Nat.one_mul, hPb] at hb1
  rw [Nat.add_mul, Nat.one_mul, hPa, hPb] at ha1
  have hhw : h * w = v.val.length := by rw [hl, Nat.mul_comm]
  unfold flip_v_loop0_loop0
  apply loop.spec_decr_nat
    (measure := fun (p : alloc.vec.Vec Std.U32 × Std.Usize) => w - p.2.val)
    (inv := fun p => p.2.val ≤ w ∧ p.1.val.length = v.val.length ∧ ∀ k < v.val.length,
      p.1.val[k]! = if (k / w = a.val ∨ k / w = b.val) ∧ k % w < p.2.val then
        v.val[mirV w h k]! else v.val[k]!)
  · rintro ⟨u, x⟩ ⟨hx, hul, hu⟩
    simp only at hx hul hu
    unfold flip_v_loop0_loop0.body
    step*
    · have : j.val = Pb + x.val := by rw [j_post, i3_post, hW, hPb]
      simp only [index_mut_back_post]; scalar_tac
    · have hxw : x.val < w := by scalar_tac
      have hA : i2.val = Pa + x.val := by rw [i2_post, i1_post, hW, hPa]
      have hB : j.val = Pb + x.val := by rw [j_post, i3_post, hW, hPb]
      obtain ⟨hda, hma⟩ := div_mod_of_lt (w := w) (b := a.val) (a := x.val) hxw
      obtain ⟨hdb, hmb⟩ := div_mod_of_lt (w := w) (b := b.val) (a := x.val) hxw
      rw [hPa] at hda hma
      rw [hPb] at hdb hmb
      have hla : Pa + x.val < v.val.length := by omega
      have hlb : Pb + x.val < v.val.length := by omega
      have mia : mirV w h (Pa + x.val) = Pb + x.val := by
        show (h - 1 - (Pa + x.val) / w) * w + (Pa + x.val) % w = Pb + x.val
        rw [hda, hma, ← hPb, show h - 1 - a.val = b.val by omega]
      have mib : mirV w h (Pb + x.val) = Pa + x.val := by
        show (h - 1 - (Pb + x.val) / w) * w + (Pb + x.val) % w = Pa + x.val
        rw [hdb, hmb, ← hPa, show h - 1 - b.val = a.val by omega]
      have ht : t = v.val[Pa + x.val]! := by
        have h1 : u.val[i2.val]! = t := by
          rw [t_post]; exact getElem!_pos u.val i2.val (by rw [hA, hul]; exact hla)
        rw [← h1, hA, hu _ hla, hma, if_neg (by omega)]
      have hi4 : i4 = v.val[Pb + x.val]! := by
        have h1 : u.val[j.val]! = i4 := by
          rw [i4_post]; exact getElem!_pos u.val j.val (by rw [hB, hul]; exact hlb)
        rw [← h1, hB, hu _ hlb, hmb, if_neg (by omega)]
      refine ⟨by scalar_tac, by simp [*], fun k hk => ?_, by scalar_tac⟩
      rw [index_mut_back1_post, index_mut_back_post]
      simp only [alloc.vec.Vec.set_val_eq, hA, hB]
      by_cases hkb : k = Pb + x.val
      · rw [hkb, set_eq _ _ (by simp; omega), if_pos (by rw [hdb, hmb]; scalar_tac), mib, ht]
      by_cases hka : k = Pa + x.val
      · rw [hka, set_ne _ _ (by omega), set_eq _ _ (by omega),
          if_pos (by rw [hda, hma]; scalar_tac), mia, hi4]
      rw [set_ne _ _ (Ne.symm hkb), set_ne _ _ (Ne.symm hka), hu k hk]
      have : ¬ (k / w = a.val ∧ k % w = x.val) := by
        rintro ⟨h1, h2⟩; exact hka (by rw [← hPa, ← h1, ← h2, Nat.mul_comm, Nat.div_add_mod])
      have : ¬ (k / w = b.val ∧ k % w = x.val) := by
        rintro ⟨h1, h2⟩; exact hkb (by rw [← hPb, ← h1, ← h2, Nat.mul_comm, Nat.div_add_mod])
      have e : ((k / w = a.val ∨ k / w = b.val) ∧ k % w < x.val) ↔
          ((k / w = a.val ∨ k / w = b.val) ∧ k % w < x1.val) := by
        constructor <;> intro h <;> scalar_tac
      simp only [e]
    · refine ⟨hul, fun k hk => ?_⟩
      rw [hu k hk]
      have := Nat.mod_lt k hw
      have e : ((k / w = a.val ∨ k / w = b.val) ∧ k % w < x.val) ↔ (k / w = a.val ∨ k / w = b.val) := by
        constructor
        · exact fun h => h.1
        · intro h; exact ⟨h, by scalar_tac⟩
      simp only [e]
  · exact ⟨hx, hul, hu⟩

theorem flip_v_outer_spec (c0 : Canvas) (hv : Valid c0) (u : alloc.vec.Vec Std.U32)
    (a b : Std.Usize) (hsum : a.val + b.val = c0.h.val - 1) (hab : a.val ≤ b.val + 1)
    (hul : u.val.length = c0.px.val.length)
    (hu : ∀ k < c0.px.val.length, u.val[k]! =
      if k / c0.w.val < a.val ∨ b.val < k / c0.w.val then c0.px.val[mirV c0.w.val c0.h.val k]!
      else c0.px.val[k]!) :
    flip_v_loop0 c0.w u a b ⦃ r => r.val.length = c0.px.val.length ∧
      ∀ k < c0.px.val.length, r.val[k]! = c0.px.val[mirV c0.w.val c0.h.val k]! ⦄ := by
  obtain ⟨hw1, hh1, hw4, hh4, hl0⟩ := hv
  unfold flip_v_loop0
  apply loop.spec_decr_nat
    (measure := fun (p : alloc.vec.Vec Std.U32 × Std.Usize × Std.Usize) => c0.h.val - p.2.1.val)
    (inv := fun p => p.2.1.val + p.2.2.val = c0.h.val - 1 ∧ p.2.1.val ≤ p.2.2.val + 1 ∧
      p.1.val.length = c0.px.val.length ∧ ∀ k < c0.px.val.length, p.1.val[k]! =
        if k / c0.w.val < p.2.1.val ∨ p.2.2.val < k / c0.w.val then
          c0.px.val[mirV c0.w.val c0.h.val k]! else c0.px.val[k]!)
  · rintro ⟨u, a, b⟩ ⟨hsum, hab, hul, hu⟩
    simp only at hsum hab hul hu
    unfold flip_v_loop0.body
    step*
    · have hab' : a.val < b.val := by scalar_tac
      have := flip_v_inner_spec c0.w.val c0.h.val u c0.w a b 0#usize rfl (by omega)
        (by rw [hul, hl0]) (by rw [hul, hl0]; exact Nat.mul_le_mul hw4 hh4) hab' (by omega) hsum
        (by simp) u rfl (fun k _ => by simp)
      step*
      refine ⟨by scalar_tac, by scalar_tac, by rw [v1_post, hul], fun k hk => ?_, by scalar_tac⟩
      have hk' : k < c0.w.val * c0.h.val := by rw [← hl0]; exact hk
      have hq : k / c0.w.val < c0.h.val := Nat.div_lt_of_lt_mul hk'
      obtain ⟨hmd, -⟩ := mirV_div_mod (h := c0.h.val) (k := k) (show 0 < c0.w.val by omega)
      have hml : mirV c0.w.val c0.h.val k < c0.px.val.length := by rw [hl0]; exact mirV_lt (by omega) hk'
      rw [v1_post1 k (by omega)]
      generalize hr : k / c0.w.val = r at hq hmd ⊢
      by_cases hrow : r = a.val ∨ r = b.val
      · rw [if_pos hrow, hu _ hml, hmd, if_neg (by omega), if_pos (by scalar_tac)]
      · rw [if_neg hrow, hu k hk, hr]
        by_cases hlt : r < a.val ∨ b.val < r
        · rw [if_pos hlt, if_pos (by scalar_tac)]
        · rw [if_neg hlt, if_neg (by scalar_tac)]
    · refine ⟨hul, fun k hk => ?_⟩
      rw [hu k hk]
      have hk' : k < c0.w.val * c0.h.val := by rw [← hl0]; exact hk
      have hq : k / c0.w.val < c0.h.val := Nat.div_lt_of_lt_mul hk'
      by_cases hlt : k / c0.w.val < a.val ∨ b.val < k / c0.w.val
      · rw [if_pos hlt]
      · rw [if_neg hlt]
        have hm : mirV c0.w.val c0.h.val k = k := by
          have := Nat.div_add_mod k c0.w.val
          show (c0.h.val - 1 - k / c0.w.val) * c0.w.val + k % c0.w.val = k
          have : c0.h.val - 1 - k / c0.w.val = k / c0.w.val := by
            generalize k / c0.w.val = q at hq hlt ⊢; scalar_tac
          rw [this, Nat.mul_comm, Nat.div_add_mod]
        rw [hm]
  · exact ⟨hsum, hab, hul, hu⟩

/-- `flip_v` mirrors the rows: pixel `k` of the result is pixel `mirV w h k`. A canvas
that is not valid does not change (P10). -/
@[step]
theorem flip_v_spec (c : Canvas) :
    flip_v c ⦃ r => r.w = c.w ∧ r.h = c.h ∧ r.px.val.length = c.px.val.length ∧
      (Valid c → ∀ k < c.px.val.length, r.px.val[k]! = c.px.val[mirV c.w.val c.h.val k]!) ∧
      (¬ Valid c → r = c) ⦄ := by
  unfold flip_v
  step as ⟨b, hb⟩
  split
  · have hv : Valid c := hb.mp (by assumption)
    obtain ⟨hw1, hh1, hw4, hh4, hl0⟩ := id hv
    step as ⟨b1, hb1, hb2⟩
    have := flip_v_outer_spec c hv c.px 0#usize b1 (by simp; scalar_tac) (by simp) rfl
      (fun k hk => by
        have : k / c.w.val < c.h.val := Nat.div_lt_of_lt_mul (by rw [← hl0]; exact hk)
        have hb3 : b1.val = c.h.val - 1 := hb1
        rw [if_neg (by rw [hb3]; simp; omega)])
    step*
  · have hv : ¬ Valid c := fun h => by simp_all
    simp [hv]

/-- P5: flip vertical twice gives the same canvas, for any canvas. -/
theorem flip_v_twice (c : Canvas) : (do let c1 ← flip_v c; flip_v c1) = ok c := by
  obtain ⟨c1, e1, hw1, hh1, hl1, hp1, hi1⟩ := spec_ok_exists (flip_v_spec c)
  obtain ⟨c2, e2, hw2, hh2, hl2, hp2, hi2⟩ := spec_ok_exists (flip_v_spec c1)
  simp only [e1, e2, bind_ok, ok.injEq]
  by_cases hc : Valid c
  · have hv1 : Valid c1 := (valid_of_same hw1 hh1 hl1).mpr hc
    have hw : 0 < c.w.val := hc.1
    have hl := hc.2.2.2.2
    apply canvas_ext (by rw [hw2, hw1]) (by rw [hh2, hh1])
    apply list_ext! _ _ (by rw [hl2, hl1])
    intro k hk
    rw [hl2, hl1] at hk
    have hm : mirV c.w.val c.h.val k < c.px.val.length := by
      rw [hl]; exact mirV_lt hw (by rw [← hl]; exact hk)
    rw [hp2 hv1 k (by rw [hl1]; exact hk), hw1, hh1, hp1 hc _ hm, mirV_mirV hw (by rw [← hl]; exact hk)]
  · have := hi1 hc
    subst this
    exact hi2 hc

end Logbook.Paint
