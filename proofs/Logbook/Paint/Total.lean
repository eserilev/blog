import Logbook.Paint.Basic
open Aeneas Aeneas.Std Result logbook_paint

/-! P10 (spec 7.6): the whole-picture functions return `ok` for every input. -/

namespace Logbook.Paint

@[scalar_tac clampD n]
theorem clampD_bounds (n : Nat) : 1 ≤ clampD n ∧ clampD n ≤ 4096 := clampD_le n

@[scalar_tac clampC v]
theorem clampC_bounds' (v : Int) : -65536 ≤ clampC v ∧ clampC v ≤ 65536 := clampC_bounds v

theorem mul_le_max {a b : Nat} (ha : a ≤ 4096) (hb : b ≤ 4096) : a * b ≤ Usize.max :=
  le_trans (Nat.mul_le_mul ha hb) (by scalar_tac)

theorem and255 (p : Std.U32) : (p &&& 255#u32).val ≤ 255 := by
  rw [UScalar.val_and]
  exact Nat.and_le_right

theorem tbw_loop_spec (c : Canvas) (i : Std.Usize) (hi : i.val ≤ c.px.val.length) :
    to_black_white_loop c i ⦃ r => r.1 = c.w ∧ r.2.1 = c.h ∧
      r.2.2.val.length = c.px.val.length ⦄ := by
  unfold to_black_white_loop
  apply loop.spec_decr_nat
    (measure := fun (p : Canvas × Std.Usize) => c.px.val.length - p.2.val)
    (inv := fun p => p.1.w = c.w ∧ p.1.h = c.h ∧ p.1.px.val.length = c.px.val.length ∧
      p.2.val ≤ c.px.val.length)
  · rintro ⟨c1, i⟩ ⟨hw, hh, hl, hi⟩
    simp only at hw hh hl hi
    unfold to_black_white_loop.body
    step* <;> first
      | (have := and255 p; have := and255 i2; have := and255 i3; scalar_tac)
      | (split <;> step* <;> simp [index_mut_back_post, hw, hh, hl] <;> scalar_tac)
  · exact ⟨rfl, rfl, rfl, hi⟩

/-- P10: `to_black_white` returns `ok`. The sides and the pixel count stay. A canvas
that is not valid does not change. -/
@[step]
theorem to_black_white_spec (c : Canvas) :
    to_black_white c ⦃ r => r.w = c.w ∧ r.h = c.h ∧ r.px.val.length = c.px.val.length ∧
      (¬ Valid c → r = c) ⦄ := by
  unfold to_black_white
  have := tbw_loop_spec c 0#usize (by simp)
  step*

/-! `copy_region`. -/

@[step]
theorem hcast_i32_spec (n : Std.Usize) (h : n.val ≤ 4096) :
    (lift (UScalar.hcast .I32 n) : Result Std.I32) ⦃ r => r.val = n.val ⦄ := by
  simp only [lift, WP.spec_ok, UScalar.hcast_val_eq]
  apply Int.bmod_eq_of_le
  all_goals simp
  omega

@[step]
theorem hcast_usize32_spec (x : Std.I32) (h : 0 ≤ x.val) :
    (lift (IScalar.hcast .Usize x) : Result Std.Usize) ⦃ r => r.val = x.val.toNat ⦄ := by
  simp [lift, hcast_usize_val x h]

theorem copy_inner_spec (c : Canvas) (w : Std.Usize) (x y : Std.I32) (yy : Std.Usize)
    (v : alloc.vec.Vec Std.U32) (xx : Std.Usize) (hw : w.val ≤ 4096) (hyy : yy.val < 4096)
    (hx : -65536 ≤ x.val ∧ x.val ≤ 65536) (hy : -65536 ≤ y.val ∧ y.val ≤ 65536)
    (hv : (yy.val + 1) * w.val ≤ v.val.length) (hxx : xx.val ≤ w.val) :
    copy_region_loop0_loop0 c w x y v yy xx ⦃ r => r.val.length = v.val.length ⦄ := by
  unfold copy_region_loop0_loop0
  apply loop.spec_decr_nat
    (measure := fun (p : alloc.vec.Vec Std.U32 × Std.Usize) => w.val - p.2.val)
    (inv := fun p => p.1.val.length = v.val.length ∧ p.2.val ≤ w.val)
  · rintro ⟨v1, xx⟩ ⟨hl, hxx⟩
    simp only at hl hxx
    unfold copy_region_loop0_loop0.body
    by_cases h : xx.val < w.val
    · have : yy.val * w.val + xx.val < v.val.length := by
        have := Nat.add_mul yy.val 1 w.val; omega
      step*
      simp [index_mut_back_post, hl]; scalar_tac
    · have : ¬ xx < w := by scalar_tac
      simp [this, hl]
  · exact ⟨rfl, hxx⟩

@[step]
theorem copy_outer_spec (c : Canvas) (w h : Std.Usize) (x y : Std.I32)
    (v : alloc.vec.Vec Std.U32) (yy : Std.Usize) (hw : w.val ≤ 4096) (hh : h.val ≤ 4096)
    (hx : -65536 ≤ x.val ∧ x.val ≤ 65536) (hy : -65536 ≤ y.val ∧ y.val ≤ 65536)
    (hv : v.val.length = w.val * h.val) (hyy : yy.val ≤ h.val) :
    copy_region_loop0 c w h x y v yy ⦃ r => r.val.length = w.val * h.val ⦄ := by
  unfold copy_region_loop0
  apply loop.spec_decr_nat
    (measure := fun (p : alloc.vec.Vec Std.U32 × Std.Usize) => h.val - p.2.val)
    (inv := fun p => p.1.val.length = w.val * h.val ∧ p.2.val ≤ h.val)
  · rintro ⟨v1, yy⟩ ⟨hl, hyy⟩
    simp only at hl hyy
    unfold copy_region_loop0.body
    by_cases hlt : yy.val < h.val
    · have hrow : (yy.val + 1) * w.val ≤ v1.val.length := by
        rw [hl, Nat.mul_comm w.val]; exact Nat.mul_le_mul_right _ hlt
      have := copy_inner_spec c w x y yy v1 0#usize hw (by omega) hx hy hrow (by simp)
      step*
    · have : ¬ yy < h := by scalar_tac
      simp [this, hl]
  · exact ⟨hv, hyy⟩

/-- P10: `copy_region` returns `ok` and a valid canvas, for every input. -/
@[step]
theorem copy_region_spec (c : Canvas) (x y w h : Std.I32) :
    copy_region c x y w h ⦃ r => Valid r ⦄ := by
  unfold copy_region
  have hx := clampC_bounds x.val
  have hy := clampC_bounds y.val
  split <;> step* <;> split <;> step*
  all_goals (try (unfold Valid; simp only))
  all_goals first
    | scalar_tac
    | exact mul_le_max (by scalar_tac) (by scalar_tac)

/-! `blit`. -/

/-- `set` keeps the sides and the pixel count. -/
@[step]
theorem set_len_spec (c : Canvas) (x y : Std.I32) (color : Std.U32) :
    logbook_paint.set c x y color ⦃ r => r.w = c.w ∧ r.h = c.h ∧
      r.px.val.length = c.px.val.length ⦄ := by
  apply WP.spec_mono (set_spec c x y color)
  rintro r ⟨h1, h2, h3, h4⟩
  refine ⟨h1, h2, ?_⟩
  by_cases h : Inside c x.val y.val
  · simp [h3 h]
  · simp [h4 h]

theorem blit_inner_spec (dst : Canvas) (sw : Std.Usize) (v : alloc.vec.Vec Std.U32)
    (use_key : Bool) (key : Std.U32) (x y : Std.I32) (sy sx : Std.Usize) (hsw : sw.val ≤ 4096)
    (hsy : sy.val < 4096) (hx : -65536 ≤ x.val ∧ x.val ≤ 65536)
    (hy : -65536 ≤ y.val ∧ y.val ≤ 65536) (hv : (sy.val + 1) * sw.val ≤ v.val.length)
    (hsx : sx.val ≤ sw.val) :
    blit_loop0_loop0 dst sw v use_key key x y sy sx ⦃ r => r.w = dst.w ∧ r.h = dst.h ∧
      r.px.val.length = dst.px.val.length ⦄ := by
  unfold blit_loop0_loop0
  apply loop.spec_decr_nat
    (measure := fun (p : Canvas × Std.Usize) => sw.val - p.2.val)
    (inv := fun p => p.1.w = dst.w ∧ p.1.h = dst.h ∧ p.1.px.val.length = dst.px.val.length ∧
      p.2.val ≤ sw.val)
  · rintro ⟨d, sx⟩ ⟨hw, hh, hl, hsx⟩
    simp only at hw hh hl hsx
    unfold blit_loop0_loop0.body
    by_cases h : sx.val < sw.val
    · have : sy.val * sw.val + sx.val < v.val.length := by
        have := Nat.add_mul sy.val 1 sw.val; omega
      step*
      all_goals (split <;> (try split) <;> (try split) <;> step*)
    · have : ¬ sx < sw := by scalar_tac
      simp [this, hw, hh, hl]
  · exact ⟨rfl, rfl, rfl, hsx⟩

theorem blit_outer_spec (dst src : Canvas) (use_key : Bool) (key : Std.U32) (x y : Std.I32)
    (sy : Std.Usize) (hs : Valid src) (hx : -65536 ≤ x.val ∧ x.val ≤ 65536)
    (hy : -65536 ≤ y.val ∧ y.val ≤ 65536) (hsy : sy.val ≤ src.h.val) :
    blit_loop0 dst src use_key key x y sy ⦃ r => r.w = dst.w ∧ r.h = dst.h ∧
      r.px.val.length = dst.px.val.length ⦄ := by
  obtain ⟨h1, h2, h3, h4, h5⟩ := hs
  unfold blit_loop0
  apply loop.spec_decr_nat
    (measure := fun (p : Canvas × Std.Usize) => src.h.val - p.2.val)
    (inv := fun p => p.1.w = dst.w ∧ p.1.h = dst.h ∧ p.1.px.val.length = dst.px.val.length ∧
      p.2.val ≤ src.h.val)
  · rintro ⟨d, sy⟩ ⟨hw, hh, hl, hsy⟩
    simp only at hw hh hl hsy
    unfold blit_loop0.body
    by_cases hlt : sy.val < src.h.val
    · have hrow : (sy.val + 1) * src.w.val ≤ src.px.val.length := by
        rw [h5, Nat.mul_comm src.w.val]; exact Nat.mul_le_mul_right _ hlt
      have := blit_inner_spec d src.w src.px use_key key x y sy 0#usize h3 (by omega) hx hy hrow
        (by simp)
      step*
    · have : ¬ sy < src.h := by scalar_tac
      simp [this, hw, hh, hl]
  · exact ⟨rfl, rfl, rfl, hsy⟩

/-- P10: `blit` returns `ok`. The sides and the pixel count of `dst` stay, so a valid
`dst` stays valid. If `dst` is not valid, it does not change. -/
@[step]
theorem blit_spec (dst src : Canvas) (x y : Std.I32) (use_key : Bool) (key : Std.U32) :
    blit dst src x y use_key key ⦃ r => r.w = dst.w ∧ r.h = dst.h ∧
      r.px.val.length = dst.px.val.length ∧ (¬ Valid dst → r = dst) ⦄ := by
  unfold blit
  step*
  have hd : Valid dst := b_post.mp (by assumption)
  apply WP.spec_mono (blit_outer_spec dst src use_key key x1 y1 0#usize (b1_post.mp (by assumption))
    (by scalar_tac) (by scalar_tac) (by simp))
  rintro r ⟨h1, h2, h3⟩
  exact ⟨h1, h2, h3, fun h => absurd hd h⟩

/-! `resize`. -/

/-- Row `y` is inside a `w × h` grid, so it fits: `(y + 1) * w ≤ w * h`. -/
theorem row_le {y w h : Nat} (hy : y < h) : (y + 1) * w ≤ w * h := by
  rw [Nat.mul_comm w h]; exact Nat.mul_le_mul_right _ hy

theorem resize_inner_spec (cw : Std.Usize) (v : alloc.vec.Vec Std.U32) (ow : Std.Usize)
    (v1 : alloc.vec.Vec Std.U32) (mw y x : Std.Usize) (hmw : mw.val ≤ cw.val)
    (hmw1 : mw.val ≤ ow.val) (hv : (y.val + 1) * cw.val ≤ v.val.length)
    (hv1 : (y.val + 1) * ow.val ≤ v1.val.length) (hx : x.val ≤ mw.val) :
    resize_loop0_loop0 cw v ow v1 mw y x ⦃ r => r.val.length = v1.val.length ⦄ := by
  unfold resize_loop0_loop0
  apply loop.spec_decr_nat
    (measure := fun (p : alloc.vec.Vec Std.U32 × Std.Usize) => mw.val - p.2.val)
    (inv := fun p => p.1.val.length = v1.val.length ∧ p.2.val ≤ mw.val)
  · rintro ⟨v2, x⟩ ⟨hl, hx⟩
    simp only at hl hx
    unfold resize_loop0_loop0.body
    by_cases h : x.val < mw.val
    · have : y.val * cw.val + x.val < v.val.length := by
        have := Nat.add_mul y.val 1 cw.val; omega
      have : y.val * ow.val + x.val < v1.val.length := by
        have := Nat.add_mul y.val 1 ow.val; omega
      step*
      simp [index_mut_back_post, hl]; scalar_tac
    · have : ¬ x < mw := by scalar_tac
      simp [this, hl]
  · exact ⟨rfl, hx⟩

theorem resize_outer_spec (cw : Std.Usize) (v : alloc.vec.Vec Std.U32) (ow : Std.Usize)
    (v1 : alloc.vec.Vec Std.U32) (mw mh y : Std.Usize) (ch oh : Nat) (hmw : mw.val ≤ cw.val)
    (hmw1 : mw.val ≤ ow.val) (hmh : mh.val ≤ ch) (hmh1 : mh.val ≤ oh)
    (hv : v.val.length = cw.val * ch) (hv1 : v1.val.length = ow.val * oh) (hy : y.val ≤ mh.val) :
    resize_loop0 cw v ow v1 mw mh y ⦃ r => r.val.length = v1.val.length ⦄ := by
  unfold resize_loop0
  apply loop.spec_decr_nat
    (measure := fun (p : alloc.vec.Vec Std.U32 × Std.Usize) => mh.val - p.2.val)
    (inv := fun p => p.1.val.length = v1.val.length ∧ p.2.val ≤ mh.val)
  · rintro ⟨v2, y⟩ ⟨hl, hy⟩
    simp only at hl hy
    unfold resize_loop0.body
    by_cases h : y.val < mh.val
    · have := resize_inner_spec cw v ow v2 mw y 0#usize hmw hmw1
        (by rw [hv]; exact row_le (by omega)) (by rw [hl, hv1]; exact row_le (by omega)) (by simp)
      step*
    · have : ¬ y < mh := by scalar_tac
      simp [this, hl]
  · exact ⟨rfl, hy⟩

/-- P10: `resize` returns `ok` and a valid canvas, for every input. -/
@[step]
theorem resize_spec (c : Canvas) (w h : Std.Usize) (bg : Std.U32) :
    resize c w h bg ⦃ r => Valid r ⦄ := by
  unfold resize
  step*
  all_goals have hout : Valid out :=
    ⟨by scalar_tac, by scalar_tac, by scalar_tac, by scalar_tac,
      by rw [out_post2, List.length_replicate, out_post, out_post1]⟩
  · have hc : Valid c := b_post.mp (by assumption)
    obtain ⟨_, _, _, _, hl⟩ := hc
    obtain ⟨_, _, _, _, hlo⟩ := hout
    obtain ⟨mw, e1, hw1, hw2⟩ : ∃ mw : Std.Usize,
        (if c.w < out.w then ok c.w else ok out.w : Result Std.Usize) = ok mw ∧
        mw.val ≤ c.w.val ∧ mw.val ≤ out.w.val := by
      split
      · exact ⟨c.w, rfl, le_refl _, by scalar_tac⟩
      · exact ⟨out.w, rfl, by scalar_tac, le_refl _⟩
    obtain ⟨mh, e2, hh1, hh2⟩ : ∃ mh : Std.Usize,
        (if c.h < out.h then ok c.h else ok out.h : Result Std.Usize) = ok mh ∧
        mh.val ≤ c.h.val ∧ mh.val ≤ out.h.val := by
      split
      · exact ⟨c.h, rfl, le_refl _, by scalar_tac⟩
      · exact ⟨out.h, rfl, by scalar_tac, le_refl _⟩
    rw [e1]; simp only [bind_ok]; rw [e2]; simp only [bind_ok]
    have := resize_outer_spec c.w c.px out.w out.px mw mh 0#usize c.h.val out.h.val hw1 hw2 hh1 hh2
      hl hlo (by simp)
    step*
    refine ⟨by scalar_tac, by scalar_tac, by scalar_tac, by scalar_tac, by simp only; omega⟩
  · exact hout

/-! `stretch`. -/

/-- The nearest source pixel is inside: `a < n → a * b / n < b`. -/
theorem scale_lt {a b n : Nat} (ha : a < n) (hb : 0 < b) : a * b / n < b := by
  rw [Nat.div_lt_iff_lt_mul (by omega)]
  rw [Nat.mul_comm b n]
  exact Nat.mul_lt_mul_of_pos_right ha hb

theorem stretch_inner_spec (cw : Std.Usize) (v : alloc.vec.Vec Std.U32) (nw : Std.Usize)
    (v1 : alloc.vec.Vec Std.U32) (y sy x : Std.Usize) (hcw : 1 ≤ cw.val ∧ cw.val ≤ 4096)
    (hnw : 1 ≤ nw.val ∧ nw.val ≤ 4096) (hv : (sy.val + 1) * cw.val ≤ v.val.length)
    (hv1 : (y.val + 1) * nw.val ≤ v1.val.length) (hy : y.val < 4096) (hx : x.val ≤ nw.val) :
    stretch_loop0_loop0 cw v nw v1 y sy x ⦃ r => r.val.length = v1.val.length ⦄ := by
  unfold stretch_loop0_loop0
  apply loop.spec_decr_nat
    (measure := fun (p : alloc.vec.Vec Std.U32 × Std.Usize) => nw.val - p.2.val)
    (inv := fun p => p.1.val.length = v1.val.length ∧ p.2.val ≤ nw.val)
  · rintro ⟨v2, x⟩ ⟨hl, hx⟩
    simp only at hl hx
    unfold stretch_loop0_loop0.body
    by_cases h : x.val < nw.val
    · have hsx : x.val * cw.val / nw.val < cw.val := scale_lt h (by omega)
      have : x.val * cw.val ≤ 4096 * 4096 := Nat.mul_le_mul (by omega) hcw.2
      have : sy.val * cw.val + x.val * cw.val / nw.val < v.val.length := by
        have := Nat.add_mul sy.val 1 cw.val; omega
      have : y.val * nw.val + x.val < v1.val.length := by
        have := Nat.add_mul y.val 1 nw.val; omega
      step*
      simp [index_mut_back_post, hl]; scalar_tac
    · have : ¬ x < nw := by scalar_tac
      simp [this, hl]
  · exact ⟨rfl, hx⟩

theorem stretch_outer_spec (c : Canvas) (nw nh : Std.Usize) (v : alloc.vec.Vec Std.U32)
    (y : Std.Usize) (hc : Valid c) (hnw : 1 ≤ nw.val ∧ nw.val ≤ 4096)
    (hnh : 1 ≤ nh.val ∧ nh.val ≤ 4096) (hv : v.val.length = nw.val * nh.val)
    (hy : y.val ≤ nh.val) :
    stretch_loop0 c nw nh v y ⦃ r => r.val.length = v.val.length ⦄ := by
  obtain ⟨h1, h2, h3, h4, h5⟩ := hc
  unfold stretch_loop0
  apply loop.spec_decr_nat
    (measure := fun (p : alloc.vec.Vec Std.U32 × Std.Usize) => nh.val - p.2.val)
    (inv := fun p => p.1.val.length = v.val.length ∧ p.2.val ≤ nh.val)
  · rintro ⟨v2, y⟩ ⟨hl, hy⟩
    simp only at hl hy
    unfold stretch_loop0.body
    by_cases h : y.val < nh.val
    · have hsy : y.val * c.h.val / nh.val < c.h.val := scale_lt h (by omega)
      have : y.val * c.h.val ≤ 4096 * 4096 := Nat.mul_le_mul (by omega) h4
      step*
      have hsy' : sy.val < c.h.val := by rw [sy_post, i_post]; exact hsy
      have := stretch_inner_spec c.w c.px nw v2 y sy 0#usize ⟨h1, h3⟩ hnw
        (by rw [h5]; exact row_le hsy') (by rw [hl, hv]; exact row_le h) (by omega) (by simp)
      step*
    · have : ¬ y < nh := by scalar_tac
      simp [this, hl]
  · exact ⟨rfl, hy⟩

/-- P10: `stretch` returns `ok` for every input. A valid canvas gives a valid canvas.
A canvas that is not valid comes back the same. -/
@[step]
theorem stretch_spec (c : Canvas) (nw nh : Std.Usize) :
    stretch c nw nh ⦃ r => (Valid c → Valid r) ∧ (¬ Valid c → r = c) ⦄ := by
  unfold stretch
  step*
  · exact mul_le_max (by scalar_tac) (by scalar_tac)
  · have hc : Valid c := b_post.mp (by assumption)
    have := stretch_outer_spec c nw1 nh1 v 0#usize hc (by scalar_tac) (by scalar_tac)
      (by rw [v_post, List.length_replicate, i_post]) (by simp)
    step*
    refine ⟨fun _ => ⟨by scalar_tac, by scalar_tac, by scalar_tac, by scalar_tac, ?_⟩,
      fun h => absurd hc h⟩
    simp only
    rw [v1_post, v_post, List.length_replicate, i_post]

/-! `skew_x` and `skew_y`. -/

theorem skew_x_inner_spec (cw : Std.Usize) (v : alloc.vec.Vec Std.U32) (out : Canvas)
    (y : Std.Usize) (shift : Std.I64) (x : Std.Usize) (hcw : cw.val ≤ 4096)
    (hout : Valid out) (hy : y.val < out.h.val) (hv : (y.val + 1) * cw.val ≤ v.val.length)
    (hsh : -2 ^ 40 ≤ shift.val ∧ shift.val ≤ 2 ^ 40) (hx : x.val ≤ cw.val) :
    skew_x_loop0_loop0 cw v out y shift x ⦃ r => r.w = out.w ∧ r.h = out.h ∧
      r.px.val.length = out.px.val.length ⦄ := by
  obtain ⟨h1, h2, h3, h4, h5⟩ := hout
  unfold skew_x_loop0_loop0
  apply loop.spec_decr_nat
    (measure := fun (p : Canvas × Std.Usize) => cw.val - p.2.val)
    (inv := fun p => p.1.w = out.w ∧ p.1.h = out.h ∧ p.1.px.val.length = out.px.val.length ∧
      p.2.val ≤ cw.val)
  · rintro ⟨o, x⟩ ⟨hw, hh, hl, hx⟩
    simp only at hw hh hl hx
    unfold skew_x_loop0_loop0.body
    by_cases h : x.val < cw.val
    · have : y.val * cw.val + x.val < v.val.length := by
        have := Nat.add_mul y.val 1 cw.val; omega
      have hrow : (y.val + 1) * out.w.val ≤ o.px.val.length := by
        rw [hl, h5]; exact row_le hy
      have hrow2 : y.val * o.w.val + o.w.val ≤ o.px.val.length := by
        rw [hw]; have := Nat.add_mul y.val 1 out.w.val; omega
      step*
      split <;> step*
      split <;> step*
      all_goals first
        | scalar_tac
        | (obtain ⟨_, _⟩ := x; simp only at x_post1; step*; simp [*]; omega)
    · have : ¬ x < cw := by scalar_tac
      simp [this, hw, hh, hl]
  · exact ⟨rfl, rfl, rfl, hx⟩

theorem skew_mul_bound {a b : Int} (ha : 0 ≤ a ∧ a ≤ 4096) (hb : -57289 ≤ b ∧ b ≤ 57289) :
    -(4096 * 57289) ≤ a * b ∧ a * b ≤ 4096 * 57289 := by
  constructor <;> nlinarith

theorem skew_div_bound {t d : Int} (h : -(4096 * 57289) ≤ t ∧ t ≤ 4096 * 57289) :
    -(4096 * 57289) ≤ t.tdiv d ∧ t.tdiv d ≤ 4096 * 57289 := by
  have := Int.natAbs_tdiv_le_natAbs t d
  omega

theorem skew_x_outer_spec (c : Canvas) (num den base : Std.I64) (out : Canvas) (y : Std.Usize)
    (hc : Valid c) (hout : Valid out) (hh : out.h = c.h)
    (hnum : -57289 ≤ num.val ∧ num.val ≤ 57289) (hden : den.val = 1000)
    (hbase : 0 ≤ base.val ∧ base.val ≤ 2 ^ 30) (hy : y.val ≤ c.h.val) :
    skew_x_loop0 c.w c.h c.px num den base out y ⦃ r => Valid r ⦄ := by
  obtain ⟨h1, h2, h3, h4, h5⟩ := hc
  unfold skew_x_loop0
  apply loop.spec_decr_nat
    (measure := fun (p : Canvas × Std.Usize) => c.h.val - p.2.val)
    (inv := fun p => p.1.w = out.w ∧ p.1.h = out.h ∧ p.1.px.val.length = out.px.val.length ∧
      p.2.val ≤ c.h.val)
  · rintro ⟨o, y⟩ ⟨hw, hh', hl, hy⟩
    simp only at hw hh' hl hy
    unfold skew_x_loop0.body
    by_cases hlt : y.val < c.h.val
    · have : y < c.h := by scalar_tac
      simp only [this, if_true]
      step as ⟨i2, hi2⟩
      have hb2 := skew_mul_bound (a := i2.val) (b := num.val) (by scalar_tac) hnum
      step as ⟨i3, hi3⟩
      have hb3 := skew_div_bound (t := i3.val) (d := den.val) (by rw [hi3]; exact hb2)
      step as ⟨i4, hi4⟩
      step as ⟨shift, hshift⟩
      obtain ⟨o1, o2, o3, o4, o5⟩ := hout
      have hov : Valid o := ⟨by rw [hw]; omega, by rw [hh']; omega, by rw [hw]; omega,
        by rw [hh']; omega, by rw [hl, o5, hw, hh']⟩
      have := skew_x_inner_spec c.w c.px o y shift 0#usize h3 hov (by rw [hh', hh]; exact hlt)
        (by rw [h5]; exact row_le hlt) (by scalar_tac) (by simp)
      step*
    · have : ¬ y < c.h := by scalar_tac
      simp only [this, if_false, WP.spec_ok]
      obtain ⟨_, _, _, _, ho⟩ := hout
      exact ⟨by rw [hw]; omega, by rw [hh']; omega, by rw [hw]; omega, by rw [hh']; omega,
        by rw [hl, ho, hw, hh']⟩
  · exact ⟨rfl, rfl, rfl, hy⟩

theorem spec_ok_exists' {α} {m : Result α} {P : α → Prop} (h : m ⦃ P ⦄) : ∃ x, m = ok x ∧ P x := by
  cases m <;> simp_all [WP.spec_ok, WP.spec_div]

theorem hcast_usize_big (x : Std.I64) (h : 0 ≤ x.val) (h' : x.val ≤ 2 ^ 31) :
    (lift (IScalar.hcast .Usize x) : Result Std.Usize) ⦃ r => r.val = x.val.toNat ⦄ := by
  simp only [lift, WP.spec_ok, IScalar.hcast_val_eq]
  have : (2 : Int) ^ 32 ≤ 2 ^ UScalarTy.Usize.numBits := by
    rcases System.Platform.numBits_eq with h | h <;> simp [h]
  rw [Int.emod_eq_of_lt h (by omega)]

/-- P10: `skew_x` returns `ok` for every input. A valid canvas gives a valid canvas.
A canvas that is not valid comes back the same. -/
@[step]
theorem skew_x_spec (c : Canvas) (num : Std.I32) (bg : Std.U32) :
    skew_x c num bg ⦃ r => (Valid c → Valid r) ∧ (¬ Valid c → r = c) ⦄ := by
  unfold skew_x SKEW_MAX SKEW_DEN
  step*
  have hc : Valid c := b_post.mp (by assumption)
  have hc' := hc
  obtain ⟨h1, h2, h3, h4, h5⟩ := hc'
  obtain ⟨num1, e1, hn1⟩ : ∃ n : Std.I32, (if num < i then ok i else if num > 57289#i32 then
      ok 57289#i32 else ok num : Result Std.I32) = ok n ∧ -57289 ≤ n.val ∧ n.val ≤ 57289 := by
    split
    · exact ⟨i, rfl, by scalar_tac, by scalar_tac⟩
    · split
      · exact ⟨57289#i32, rfl, by simp, by simp⟩
      · exact ⟨num, rfl, by scalar_tac, by scalar_tac⟩
  rw [e1]; simp only [bind_ok]
  step as ⟨num2, hnum2⟩
  step as ⟨den, hden⟩
  step as ⟨i1, hi1⟩
  step as ⟨i2, hi2⟩
  have hb2 := skew_mul_bound (a := i2.val) (b := num2.val) (by scalar_tac) (by scalar_tac)
  step as ⟨i3, hi3⟩
  have hb3 := skew_div_bound (t := i3.val) (d := den.val) (by rw [hi3]; exact hb2)
  step as ⟨last, hlast⟩
  obtain ⟨extra, e2, hex⟩ : ∃ e : Std.I64, (if last < 0#i64 then -.last else ok last) = ok e ∧
      0 ≤ e.val ∧ e.val ≤ 4096 * 57289 := by
    split
    · have := HNeg.hNeg.step last (by intro h; scalar_tac)
      obtain ⟨e, he, he2⟩ := spec_ok_exists' this
      exact ⟨e, he, by scalar_tac, by scalar_tac⟩
    · exact ⟨last, rfl, by scalar_tac, by scalar_tac⟩
  rw [e2]; simp only [bind_ok]
  obtain ⟨base, e3, hbase⟩ : ∃ b : Std.I64, (if num2 < 0#i64 then ok extra else ok 0#i64 :
      Result Std.I64) = ok b ∧ 0 ≤ b.val ∧ b.val ≤ 4096 * 57289 := by
    split
    · exact ⟨extra, rfl, hex⟩
    · exact ⟨0#i64, rfl, by simp, by simp⟩
  rw [e3]; simp only [bind_ok]
  obtain ⟨i4, e4, hi4⟩ := spec_ok_exists' (hcast_usize_big extra hex.1 (by omega))
  rw [e4]; simp only [bind_ok]
  have hi4' : i4.val ≤ 4096 * 57289 := by omega
  step as ⟨i5, hi5⟩
  obtain ⟨out, eo, hout1, hout2, hout3⟩ := spec_ok_exists' (new_canvas_spec i5 c.h bg)
  rw [eo]; simp only [bind_ok]
  have hout : Valid out := ⟨by scalar_tac, by scalar_tac, by scalar_tac, by scalar_tac,
    by rw [hout3, List.length_replicate, hout1, hout2]⟩
  have hoh : out.h = c.h := by
    apply UScalar.eq_of_val_eq; rw [hout2]; unfold clampD; split_ifs <;> omega
  have := skew_x_outer_spec c num2 den base out 0#usize hc hout hoh (by rw [hnum2]; exact hn1)
    hden (by omega) (by simp)
  apply WP.spec_mono this
  intro r hr
  exact ⟨fun _ => hr, fun h => absurd hc h⟩

theorem skew_y_inner_spec (cw ch : Std.Usize) (v : alloc.vec.Vec Std.U32) (out : Canvas)
    (col : Std.Usize) (shift : Std.I64) (y : Std.Usize) (hcw : cw.val ≤ 4096) (hch : ch.val ≤ 4096)
    (hout : Valid out) (hx : col.val < out.w.val) (hxc : col.val < cw.val)
    (hv : v.val.length = cw.val * ch.val)
    (hsh : -2 ^ 40 ≤ shift.val ∧ shift.val ≤ 2 ^ 40) (hy : y.val ≤ ch.val) :
    skew_y_loop0_loop0 cw ch v out col shift y ⦃ r => r.w = out.w ∧ r.h = out.h ∧
      r.px.val.length = out.px.val.length ⦄ := by
  obtain ⟨h1, h2, h3, h4, h5⟩ := hout
  unfold skew_y_loop0_loop0
  apply loop.spec_decr_nat
    (measure := fun (p : Canvas × Std.Usize) => ch.val - p.2.val)
    (inv := fun p => p.1.w = out.w ∧ p.1.h = out.h ∧ p.1.px.val.length = out.px.val.length ∧
      p.2.val ≤ ch.val)
  · rintro ⟨o, y⟩ ⟨hw, hh, hl, hy⟩
    simp only at hw hh hl hy
    unfold skew_y_loop0_loop0.body
    by_cases h : y.val < ch.val
    · have : y.val * cw.val + col.val < v.val.length := by rw [hv]; exact idx_lt hxc h
      step*
      split
      · step*
        split
        · have hny : ny.val.toNat < o.h.val := by scalar_tac
          have hk := idx_lt (a := col.val) (b := ny.val.toNat) (w := o.w.val) (h := o.h.val)
            (by rw [hw]; exact hx) hny
          have hlo : o.px.val.length = o.w.val * o.h.val := by rw [hl, h5, hw, hh]
          step*
          obtain ⟨_, _⟩ := x
          simp only at x_post1
          step*
          simp [*]; omega
        · step*
      · step*
    · have : ¬ y < ch := by scalar_tac
      simp [this, hw, hh, hl]
  · exact ⟨rfl, rfl, rfl, hy⟩

theorem skew_y_outer_spec (c : Canvas) (num den base : Std.I64) (out : Canvas) (x : Std.Usize)
    (hc : Valid c) (hout : Valid out) (hw : out.w = c.w)
    (hnum : -57289 ≤ num.val ∧ num.val ≤ 57289) (hden : den.val = 1000)
    (hbase : 0 ≤ base.val ∧ base.val ≤ 2 ^ 30) (hx : x.val ≤ c.w.val) :
    skew_y_loop0 c.w c.h c.px num den base out x ⦃ r => Valid r ⦄ := by
  obtain ⟨h1, h2, h3, h4, h5⟩ := hc
  unfold skew_y_loop0
  apply loop.spec_decr_nat
    (measure := fun (p : Canvas × Std.Usize) => c.w.val - p.2.val)
    (inv := fun p => p.1.w = out.w ∧ p.1.h = out.h ∧ p.1.px.val.length = out.px.val.length ∧
      p.2.val ≤ c.w.val)
  · rintro ⟨o, x⟩ ⟨hw', hh, hl, hx⟩
    simp only at hw' hh hl hx
    unfold skew_y_loop0.body
    obtain ⟨o1, o2, o3, o4, o5⟩ := hout
    by_cases hlt : x.val < c.w.val
    · have : x < c.w := by scalar_tac
      simp only [this, if_true]
      step as ⟨i2, hi2⟩
      have hb2 := skew_mul_bound (a := i2.val) (b := num.val) (by scalar_tac) hnum
      step as ⟨i3, hi3⟩
      have hb3 := skew_div_bound (t := i3.val) (d := den.val) (by rw [hi3]; exact hb2)
      step as ⟨i4, hi4⟩
      step as ⟨shift, hshift⟩
      have hov : Valid o := ⟨by rw [hw']; omega, by rw [hh]; omega, by rw [hw']; omega,
        by rw [hh]; omega, by rw [hl, o5, hw', hh]⟩
      have := skew_y_inner_spec c.w c.h c.px o x shift 0#usize h3 h4 hov
        (by rw [hw', hw]; exact hlt) hlt h5 (by scalar_tac) (by simp)
      step*
    · have : ¬ x < c.w := by scalar_tac
      simp only [this, if_false, WP.spec_ok]
      exact ⟨by rw [hw']; omega, by rw [hh]; omega, by rw [hw']; omega, by rw [hh]; omega,
        by rw [hl, o5, hw', hh]⟩
  · exact ⟨rfl, rfl, rfl, hx⟩

/-- P10: `skew_y` returns `ok` for every input. A valid canvas gives a valid canvas.
A canvas that is not valid comes back the same. -/
@[step]
theorem skew_y_spec (c : Canvas) (num : Std.I32) (bg : Std.U32) :
    skew_y c num bg ⦃ r => (Valid c → Valid r) ∧ (¬ Valid c → r = c) ⦄ := by
  unfold skew_y SKEW_MAX SKEW_DEN
  step*
  have hc : Valid c := b_post.mp (by assumption)
  have hc' := hc
  obtain ⟨h1, h2, h3, h4, h5⟩ := hc'
  obtain ⟨num1, e1, hn1⟩ : ∃ n : Std.I32, (if num < i then ok i else if num > 57289#i32 then
      ok 57289#i32 else ok num : Result Std.I32) = ok n ∧ -57289 ≤ n.val ∧ n.val ≤ 57289 := by
    split
    · exact ⟨i, rfl, by scalar_tac, by scalar_tac⟩
    · split
      · exact ⟨57289#i32, rfl, by simp, by simp⟩
      · exact ⟨num, rfl, by scalar_tac, by scalar_tac⟩
  rw [e1]; simp only [bind_ok]
  step as ⟨num2, hnum2⟩
  step as ⟨den, hden⟩
  step as ⟨i1, hi1⟩
  step as ⟨i2, hi2⟩
  have hb2 := skew_mul_bound (a := i2.val) (b := num2.val) (by scalar_tac) (by scalar_tac)
  step as ⟨i3, hi3⟩
  have hb3 := skew_div_bound (t := i3.val) (d := den.val) (by rw [hi3]; exact hb2)
  step as ⟨last, hlast⟩
  obtain ⟨extra, e2, hex⟩ : ∃ e : Std.I64, (if last < 0#i64 then -.last else ok last) = ok e ∧
      0 ≤ e.val ∧ e.val ≤ 4096 * 57289 := by
    split
    · have := HNeg.hNeg.step last (by intro h; scalar_tac)
      obtain ⟨e, he, he2⟩ := spec_ok_exists' this
      exact ⟨e, he, by scalar_tac, by scalar_tac⟩
    · exact ⟨last, rfl, by scalar_tac, by scalar_tac⟩
  rw [e2]; simp only [bind_ok]
  obtain ⟨base, e3, hbase⟩ : ∃ b : Std.I64, (if num2 < 0#i64 then ok extra else ok 0#i64 :
      Result Std.I64) = ok b ∧ 0 ≤ b.val ∧ b.val ≤ 4096 * 57289 := by
    split
    · exact ⟨extra, rfl, hex⟩
    · exact ⟨0#i64, rfl, by simp, by simp⟩
  rw [e3]; simp only [bind_ok]
  obtain ⟨i4, e4, hi4⟩ := spec_ok_exists' (hcast_usize_big extra hex.1 (by omega))
  rw [e4]; simp only [bind_ok]
  have hi4' : i4.val ≤ 4096 * 57289 := by omega
  step as ⟨i5, hi5⟩
  obtain ⟨out, eo, hout1, hout2, hout3⟩ := spec_ok_exists' (new_canvas_spec c.w i5 bg)
  rw [eo]; simp only [bind_ok]
  have hout : Valid out := ⟨by scalar_tac, by scalar_tac, by scalar_tac, by scalar_tac,
    by rw [hout3, List.length_replicate, hout1, hout2]⟩
  have hoh : out.w = c.w := by
    apply UScalar.eq_of_val_eq; rw [hout1]; unfold clampD; split_ifs <;> omega
  have := skew_y_outer_spec c num2 den base out 0#usize hc hout hoh (by rw [hnum2]; exact hn1)
    hden (by omega) (by simp)
  apply WP.spec_mono this
  intro r hr
  exact ⟨fun _ => hr, fun h => absurd hc h⟩

end Logbook.Paint
