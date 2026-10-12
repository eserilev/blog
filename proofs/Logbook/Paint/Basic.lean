import LogbookPaint.Funs
open Aeneas Aeneas.Std Result logbook_paint

/-! The Paint core (spec 7.6): `valid`, the clamps, pixels, and `clip_rect` (P9). The other
files in `Logbook/Paint/` use these specs. -/

namespace Logbook.Paint

/-- The fields of a canvas agree: the Lean form of `valid`. -/
def Valid (c : Canvas) : Prop :=
  1 ≤ c.w.val ∧ 1 ≤ c.h.val ∧ c.w.val ≤ 4096 ∧ c.h.val ≤ 4096 ∧
    c.px.val.length = c.w.val * c.h.val

@[step]
theorem valid_spec (c : Canvas) : valid c ⦃ b => (b ↔ Valid c) ⦄ := by
  unfold valid Valid
  simp only [MAX_DIM]
  split
  · split
    · split
      · split
        · step*
          simp only [decide_eq_true_eq]
          constructor
          · intro h
            have := congrArg UScalar.val h
            simp at this
            refine ⟨by scalar_tac, by scalar_tac, by scalar_tac, by scalar_tac, by omega⟩
          · rintro ⟨-, -, -, -, h⟩
            apply UScalar.eq_of_val_eq
            simp [h, i1_post]
        · simp_all
      · simp_all
    · simp_all
  · simp_all

/-! Clamps. -/

/-- `clamp_coord`: `v` in `-65536..=65536`. -/
def clampC (v : Int) : Int := if v < -65536 then -65536 else if v > 65536 then 65536 else v

/-- `clamp_span`: `v` in `0..=131072`. -/
def clampS (v : Int) : Int := if v < 0 then 0 else if v > 131072 then 131072 else v

/-- `clamp_width`: `v` in `1..=64`. -/
def clampW (v : Int) : Int := if v < 1 then 1 else if v > 64 then 64 else v

/-- `clamp_dim`: `n` in `1..=4096`. -/
def clampD (n : Nat) : Nat := if n < 1 then 1 else if n > 4096 then 4096 else n

/-- `min`, hidden from `omega`: a chain of `min` and `max` makes `omega` split too
many cases. -/
def imin (a b : Int) : Int := min a b

/-- `max`, hidden from `omega`. See `imin`. -/
def imax (a b : Int) : Int := max a b

@[step]
theorem min_i64_spec (a b : Std.I64) : min_i64 a b ⦃ r => r.val = imin a.val b.val ⦄ := by
  unfold min_i64 imin; split <;> simp <;> scalar_tac

@[step]
theorem max_i64_spec (a b : Std.I64) : max_i64 a b ⦃ r => r.val = imax a.val b.val ⦄ := by
  unfold max_i64 imax; split <;> simp <;> scalar_tac

@[step]
theorem abs_i32_spec (v : Std.I32) (h : -2147483647 ≤ v.val) :
    abs_i32 v ⦃ r => r.val = |v.val| ⦄ := by
  unfold abs_i32; split <;> step*

@[step]
theorem clamp_coord_spec (v : Std.I32) : clamp_coord v ⦃ r => r.val = clampC v.val ⦄ := by
  unfold clamp_coord clampC COORD_LIMIT
  step* <;> split_ifs <;> scalar_tac

@[step]
theorem clamp_span_spec (v : Std.I32) : clamp_span v ⦃ r => r.val = clampS v.val ⦄ := by
  unfold clamp_span clampS COORD_LIMIT
  split_ifs <;> step*

@[step]
theorem clamp_width_spec (v : Std.I32) : clamp_width v ⦃ r => r.val = clampW v.val ⦄ := by
  unfold clamp_width clampW MAX_WIDTH
  split_ifs <;> simp only [WP.spec_ok] <;> scalar_tac

@[step]
theorem clamp_dim_spec (n : Std.Usize) : clamp_dim n ⦃ r => r.val = clampD n.val ⦄ := by
  unfold clamp_dim clampD MAX_DIM
  split_ifs <;> simp only [WP.spec_ok] <;> scalar_tac

theorem clampD_le (n : Nat) : 1 ≤ clampD n ∧ clampD n ≤ 4096 := by
  unfold clampD; split_ifs <;> omega

theorem clampC_bounds (v : Int) : -65536 ≤ clampC v ∧ clampC v ≤ 65536 := by
  unfold clampC; split_ifs <;> omega

theorem clampS_bounds (v : Int) : 0 ≤ clampS v ∧ clampS v ≤ 131072 := by
  unfold clampS; split_ifs <;> omega

theorem clampW_bounds (v : Int) : 1 ≤ clampW v ∧ clampW v ≤ 64 := by
  unfold clampW; split_ifs <;> omega

/-! Building vectors. -/

@[step]
theorem filled_spec (n : Std.Usize) (v : Std.U32) :
    filled n v ⦃ r => r.val = List.replicate n.val v ⦄ := by
  unfold filled filled_loop
  apply loop.spec_decr_nat
    (measure := fun (x : alloc.vec.Vec Std.U32 × Std.Usize) => n.val - x.2.val)
    (inv := fun x => x.2.val ≤ n.val ∧ x.1.val = List.replicate x.2.val v)
  · rintro ⟨out, i⟩ ⟨hi, hout⟩
    unfold filled_loop.body
    simp only at hi hout
    step*
    · refine ⟨by scalar_tac, ?_, by scalar_tac⟩
      simp [*, List.replicate_succ']
  · simp

@[step]
theorem zeros_spec (n : Std.Usize) :
    zeros n ⦃ r => r.val = List.replicate n.val 0#usize ⦄ := by
  unfold zeros zeros_loop
  apply loop.spec_decr_nat
    (measure := fun (x : alloc.vec.Vec Std.Usize × Std.Usize) => n.val - x.2.val)
    (inv := fun x => x.2.val ≤ n.val ∧ x.1.val = List.replicate x.2.val 0#usize)
  · rintro ⟨out, i⟩ ⟨hi, hout⟩
    unfold zeros_loop.body
    simp only at hi hout
    step*
    · refine ⟨by scalar_tac, ?_, by scalar_tac⟩
      simp [*, List.replicate_succ']
  · simp

@[step]
theorem clone_spec (c : Canvas) : Canvas.Insts.CoreCloneClone.clone c ⦃ r => r = c ⦄ := by
  unfold Canvas.Insts.CoreCloneClone.clone alloc.vec.CloneVec.clone
  have := Slice.clone_spec (clone := liftFun1 core.clone.impls.CloneU32.clone) (s := c.px.slice)
    (by intro x _; rfl)
  have h : Slice.clone (liftFun1 core.clone.impls.CloneU32.clone) c.px.slice = ok c.px.slice := by
    revert this
    cases Slice.clone (liftFun1 core.clone.impls.CloneU32.clone) c.px.slice <;>
      simp [WP.spec_ok, WP.spec_div]
    intro h; simp [h]
  simp [h]

/-! Pixels. -/

/-- Row `b`, column `a` of a `w × h` grid is in `0..w * h`. -/
theorem idx_lt {a b w h : Nat} (ha : a < w) (hb : b < h) : b * w + a < w * h := by
  have : (b + 1) * w ≤ h * w := Nat.mul_le_mul_right _ hb
  rw [Nat.mul_comm w h]; rw [Nat.add_mul] at this; omega

theorem hcast_usize_val (x : Std.I32) (h : 0 ≤ x.val) :
    (IScalar.hcast .Usize x).val = x.val.toNat := by
  rw [IScalar.hcast_val_eq]
  have : x.val < 2 ^ UScalarTy.Usize.numBits := by
    have := x.hBounds
    have : (2 : Int) ^ 32 ≤ 2 ^ UScalarTy.Usize.numBits := by
      rcases System.Platform.numBits_eq with h | h <;> simp [h]
    scalar_tac
  rw [Int.emod_eq_of_lt h this]

/-- `(x, y)` is a pixel of a valid canvas. -/
def Inside (c : Canvas) (x y : Int) : Prop :=
  Valid c ∧ 0 ≤ x ∧ 0 ≤ y ∧ x < c.w.val ∧ y < c.h.val

theorem Inside.lt {c : Canvas} {x y : Int} (h : Inside c x y) :
    y.toNat * c.w.val + x.toNat < c.px.val.length := by
  obtain ⟨⟨-, -, -, -, hl⟩, hx, hy, hxw, hyh⟩ := h
  rw [hl]; exact idx_lt (by omega) (by omega)

@[step]
theorem index_spec (c : Canvas) (x y : Std.I32) :
    index c x y ⦃ o => match o with
      | none => ¬ Inside c x.val y.val
      | some i => Inside c x.val y.val ∧ i.val = y.val.toNat * c.w.val + x.val.toNat ⦄ := by
  unfold index Inside
  step*
  all_goals
    have hv : Valid c := b_post.mp (by assumption)
    have h1 := hcast_usize_val x (by scalar_tac)
    have h2 := hcast_usize_val y (by scalar_tac)
    unfold Valid at hv
    subst_vars
    scalar_tac

@[step]
theorem new_canvas_spec (w h : Std.Usize) (color : Std.U32) :
    new_canvas w h color ⦃ r => r.w.val = clampD w.val ∧ r.h.val = clampD h.val ∧
      r.px.val = List.replicate (clampD w.val * clampD h.val) color ⦄ := by
  unfold new_canvas
  have := clampD_le w.val
  have := clampD_le h.val
  step*

@[step]
theorem get_spec (c : Canvas) (x y : Std.I32) :
    logbook_paint.get c x y ⦃ p => (Inside c x.val y.val →
        p = c.px.val[y.val.toNat * c.w.val + x.val.toNat]!) ∧
      (¬ Inside c x.val y.val → p = 0#u32) ⦄ := by
  unfold logbook_paint.get
  step as ⟨o, ho⟩
  cases o with
  | none => simp only at ho; simp [ho, TRANSPARENT]
  | some i =>
    simp only at ho
    have := ho.1.lt
    step*
    refine ⟨fun _ => ?_, fun h => absurd ho.1 h⟩
    rw [p_post, ← ho.2, getElem!_pos]

@[step]
theorem set_spec (c : Canvas) (x y : Std.I32) (color : Std.U32) :
    logbook_paint.set c x y color ⦃ r => r.w = c.w ∧ r.h = c.h ∧
      (Inside c x.val y.val →
        r.px.val = c.px.val.set (y.val.toNat * c.w.val + x.val.toNat) color) ∧
      (¬ Inside c x.val y.val → r = c) ⦄ := by
  unfold logbook_paint.set
  step as ⟨o, ho⟩
  cases o with
  | none => simp only at ho; simp [ho]
  | some i =>
    simp only at ho
    have := ho.1.lt
    step*
    refine ⟨fun _ => ?_, fun h => absurd ho.1 h⟩
    simp [index_mut_back_post, ho.2]

/-! The clipped rectangle (P9). -/


@[step]
theorem from_i32_spec (x : Std.I32) :
    (lift (core.convert.num.FromI64I32.from x) : Result Std.I64) ⦃ r => r.val = x.val ⦄ := by
  simp [lift]

theorem hcast_i64_val (n : Std.Usize) (h : n.val ≤ 4096) : (UScalar.hcast .I64 n).val = n.val := by
  rw [UScalar.hcast_val_eq]
  apply Int.bmod_eq_of_le <;> simp; omega

theorem hcast_usize_val64 (x : Std.I64) (h : 0 ≤ x.val) (h' : x.val ≤ 4096) :
    (IScalar.hcast .Usize x).val = x.val.toNat := by
  rw [IScalar.hcast_val_eq]
  have : (2 : Int) ^ 32 ≤ 2 ^ UScalarTy.Usize.numBits := by
    rcases System.Platform.numBits_eq with h | h <;> simp [h]
  rw [Int.emod_eq_of_lt h (by omega)]

@[step]
theorem hcast_i64_spec (n : Std.Usize) (h : n.val ≤ 4096) :
    (lift (UScalar.hcast .I64 n) : Result Std.I64) ⦃ r => r.val = n.val ⦄ := by
  simp [lift, hcast_i64_val n h]

@[step]
theorem hcast_usize64_spec (x : Std.I64) (h : 0 ≤ x.val) (h' : x.val ≤ 4096) :
    (lift (IScalar.hcast .Usize x) : Result Std.Usize) ⦃ r => r.val = x.val.toNat ⦄ := by
  simp [lift, hcast_usize_val64 x h h']

/-- One axis of `clip_rect`: `X` and `W` are the clamped start and side, `C` the
clamped canvas side. -/
theorem clip_math {X W C a x0 b x1 : Int} (hX : -65536 ≤ X ∧ X ≤ 65536)
    (hW : 0 ≤ W ∧ W ≤ 131072) (hC : 0 ≤ C ∧ C ≤ 4096) (ha : a = imax X 0)
    (h0 : x0 = imin a C) (hb : b = imax (X + W) x0) (h1 : x1 = imin b C) :
    0 ≤ x0 ∧ x0 ≤ x1 ∧ x1 ≤ C ∧
      ∀ i : Int, (x0 ≤ i ∧ i < x1 ↔ 0 ≤ i ∧ X ≤ i ∧ i < X + W ∧ i < C) := by
  unfold imax imin at *
  subst_vars
  refine ⟨by omega, by omega, by omega, fun i => by omega⟩

/-- P9: the rectangle from `clip_rect` is inside the `cw × ch` canvas, and inside a
`MAX_DIM × MAX_DIM` one. Its columns are those of the clamped rectangle that are in
the canvas, and the same holds for its rows. -/
@[step]
theorem clip_rect_spec (x y w h : Std.I32) (cw ch : Std.Usize) :
    clip_rect x y w h cw ch ⦃ r =>
      r.x.val + r.w.val ≤ cw.val ∧ r.x.val + r.w.val ≤ 4096 ∧
      r.y.val + r.h.val ≤ ch.val ∧ r.y.val + r.h.val ≤ 4096 ∧
      (∀ i : Nat, (r.x.val ≤ i ∧ i < r.x.val + r.w.val) ↔
        (clampC x.val ≤ i ∧ i < clampC x.val + clampS w.val ∧ i < cw.val ∧ i < 4096)) ∧
      (∀ i : Nat, (r.y.val ≤ i ∧ i < r.y.val + r.h.val) ↔
        (clampC y.val ≤ i ∧ i < clampC y.val + clampS h.val ∧ i < ch.val ∧ i < 4096)) ⦄ := by
  unfold clip_rect MAX_DIM
  have := clampC_bounds x.val
  have := clampC_bounds y.val
  have := clampS_bounds w.val
  have := clampS_bounds h.val
  step*
  have hcw : ∃ c : Std.Usize, (if cw < 4096#usize then ok cw else ok 4096#usize : Result _) = ok c ∧
      c.val = imin cw.val 4096 := by
    unfold imin; split <;> simp <;> scalar_tac
  have hch : ∃ c : Std.Usize, (if ch < 4096#usize then ok ch else ok 4096#usize : Result _) = ok c ∧
      c.val = imin ch.val 4096 := by
    unfold imin; split <;> simp <;> scalar_tac
  obtain ⟨cw1, hcw1, hcw2⟩ := hcw
  obtain ⟨ch1, hch1, hch2⟩ := hch
  have hcwb : cw1.val ≤ 4096 := by unfold imin at hcw2; omega
  have hchb : ch1.val ≤ 4096 := by unfold imin at hch2; omega
  simp only [hcw1, hch1]
  step*
  all_goals
    have hx := clip_math (X := x1.val) (W := w1.val) (C := cw2.val)
      (by rw [x1_post, i_post]; exact clampC_bounds _) (by rw [w1_post, i2_post]; exact clampS_bounds _)
      (by omega) i4_post x0_post (by rw [i7_post, i6_post]) x11_post
    have hy := clip_math (X := y1.val) (W := h1.val) (C := ch2.val)
      (by rw [y1_post, i1_post]; exact clampC_bounds _) (by rw [h1_post, i3_post]; exact clampS_bounds _)
      (by omega) i5_post y0_post (by rw [i9_post, i8_post]) y11_post
    first
    | omega
    | (have : I64.min ≤ -4096 := by scalar_tac
       omega)
    | (have : 4096 ≤ I64.max := by scalar_tac
       omega)
    | (unfold imin at hcw2 hch2
       refine ⟨by omega, by omega, by omega, by omega, fun i => ?_, fun i => ?_⟩
       · have := hx.2.2.2 i; rw [x1_post, i_post, w1_post, i2_post] at this; omega
       · have := hy.2.2.2 i; rw [y1_post, i1_post, h1_post, i3_post] at this; omega)

/-! Small lemmas on lists and results. -/

theorem div_mod_of_lt {a b w : Nat} (ha : a < w) : (b * w + a) / w = b ∧ (b * w + a) % w = a := by
  have hw : 0 < w := by omega
  constructor
  · rw [Nat.add_comm, Nat.add_mul_div_right _ _ hw, Nat.div_eq_of_lt ha]; simp
  · rw [Nat.add_comm, Nat.add_mul_mod_self_right, Nat.mod_eq_of_lt ha]

theorem set_ne {α} [Inhabited α] (l : List α) {i j : Nat} (x : α) (h : i ≠ j) :
    (l.set i x)[j]! = l[j]! := by
  simp [List.getElem!_eq_getElem?_getD, List.getElem?_set_ne h]

theorem set_eq {α} [Inhabited α] (l : List α) {i : Nat} (x : α) (h : i < l.length) :
    (l.set i x)[i]! = x := by
  simp [h]

/-- `j = (j / h) * h + j % h`, in the form `x * h + r`. -/
theorem eq_of_div_mod {j h x r : Nat} (h1 : j / h = x) (h2 : j % h = r) : j = x * h + r := by
  rw [← h1, ← h2, Nat.mul_comm, Nat.div_add_mod]

theorem spec_ok_exists {α} {m : Result α} {P : α → Prop} (h : m ⦃ P ⦄) : ∃ x, m = ok x ∧ P x := by
  cases m <;> simp_all [WP.spec_ok, WP.spec_div]

theorem list_ext! (l1 l2 : List Std.U32) (hl : l1.length = l2.length)
    (h : ∀ j < l1.length, l1[j]! = l2[j]!) : l1 = l2 := by
  apply List.ext_getElem hl
  intro j h1 h2
  have := h j h1
  rwa [getElem!_pos l1 j h1, getElem!_pos l2 j h2] at this

theorem canvas_ext {a b : Canvas} (hw : a.w = b.w) (hh : a.h = b.h) (hp : a.px.val = b.px.val) :
    a = b := by
  cases a; cases b; simp_all; exact alloc.vec.Vec.ext _ _ hp

/-- Two canvases with the same sides and pixel count are both valid or both not valid. -/
theorem valid_of_same {c r : Canvas} (hw : r.w = c.w) (hh : r.h = c.h)
    (hl : r.px.val.length = c.px.val.length) : Valid r ↔ Valid c := by
  unfold Valid; rw [hw, hh, hl]

end Logbook.Paint
