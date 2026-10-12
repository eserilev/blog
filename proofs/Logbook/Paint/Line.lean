import Logbook.Paint.Basic
import Mathlib.Tactic.LinearCombination
import Mathlib.Tactic.Linarith
open Aeneas Aeneas.Std Result logbook_paint

/-! The points of a line (spec 7.6): P3 for `line_points`, with P10 (no panic) built in. -/

namespace Logbook.Paint

/-- The loop invariant of `line_points` on plain integers. `a` and `b` count the
steps in x and y after `k` turns. `DX` and `DY` are the distances, both at least 0. -/
def LInv (DX DY k a b err : Int) : Prop :=
  0 ≤ a ∧ a ≤ k ∧ 0 ≤ b ∧ b ≤ k ∧ err = DX * (1 + b) - DY * (1 + a) ∧
  (DY ≤ DX → a = k ∧ -DY ≤ 2 * err ∧ 2 * err ≤ 3 * DX - 2 * DY) ∧
  (DX ≤ DY → b = k ∧ 2 * DX - 3 * DY ≤ 2 * err ∧ 2 * err ≤ DX)

theorem linv_init {DX DY : Int} (h1 : 0 ≤ DX) (h2 : 0 ≤ DY) : LInv DX DY 0 0 0 (DX - DY) := by
  refine ⟨le_refl _, le_refl _, le_refl _, le_refl _, by ring, fun h => ⟨rfl, by omega, by omega⟩,
    fun h => ⟨rfl, by omega, by omega⟩⟩

/-- One turn of the loop: `ex` and `ey` are the steps in x and y. -/
theorem linv_step {DX DY k a b err ex ey : Int} (h1 : 0 ≤ DX) (h2 : 0 ≤ DY)
    (h : LInv DX DY k a b err)
    (hx : (ex = 1 ∧ -DY ≤ 2 * err) ∨ (ex = 0 ∧ 2 * err < -DY))
    (hy : (ey = 1 ∧ 2 * err ≤ DX) ∨ (ey = 0 ∧ DX < 2 * err)) :
    LInv DX DY (k + 1) (a + ex) (b + ey) (err - ex * DY + ey * DX) := by
  obtain ⟨ha0, hak, hb0, hbk, he, hX, hY⟩ := h
  rcases hx with ⟨rfl, hx⟩ | ⟨rfl, hx⟩ <;> rcases hy with ⟨rfl, hy⟩ | ⟨rfl, hy⟩
  all_goals first
    | omega
    | (refine ⟨by omega, by omega, by omega, by omega, by rw [he]; ring, fun hc => ?_, fun hc => ?_⟩
       · have := hX hc; omega
       · have := hY hc; omega)

/-- With `DY ≤ DX`, the y steps never pass `DY`. -/
theorem steps_le {DX DY k b err : Int} (hk : k ≤ DX) (hDY : 0 ≤ DY) (hb0 : 0 ≤ b)
    (hbk : b ≤ k) (he : err = DX * (1 + b) - DY * (1 + k)) (hu : 2 * err ≤ 3 * DX - 2 * DY) :
    b ≤ DY := by
  rcases lt_or_eq_of_le (show 0 ≤ DX by omega) with hp | hz
  · by_contra hc
    have : DX * (2 * b - 2 * DY - 1) ≤ 0 := by nlinarith
    have : 0 < DX * (2 * b - 2 * DY - 1) := by
      apply mul_pos hp; omega
    omega
  · omega

/-- With `DY ≤ DX`, after `DX` turns the y steps reach `DY`. -/
theorem steps_ge {DX DY b err : Int} (hp : 0 < DX) (hDY : 0 ≤ DY) (hb0 : 0 ≤ b)
    (he : err = DX * (1 + b) - DY * (1 + DX)) (hl : -DY ≤ 2 * err) : DY ≤ b := by
  by_contra hc
  have : 2 * DX * (1 + b) ≤ 2 * DX * DY := by
    have : 1 + b ≤ DY := by omega
    nlinarith
  rcases lt_or_eq_of_le hDY with hq | hq
  · nlinarith
  · omega

/-- The steps stay in range, and the error term is small. -/
theorem linv_bounds {DX DY n k a b err : Int} (h1 : 0 ≤ DX) (h2 : 0 ≤ DY)
    (hn1 : DY ≤ DX → n = DX) (hn2 : DX ≤ DY → n = DY) (hk : k ≤ n)
    (h : LInv DX DY k a b err) :
    a ≤ DX ∧ b ≤ DY ∧ -3 * n ≤ 2 * err ∧ 2 * err ≤ 3 * n := by
  obtain ⟨ha0, hak, hb0, hbk, he, hX, hY⟩ := h
  rcases le_total DY DX with hc | hc
  · obtain ⟨hka, hl, hu⟩ := hX hc
    have := hn1 hc
    rw [hka] at he
    refine ⟨by omega, steps_le (by omega) h2 hb0 hbk he hu, by omega, by omega⟩
  · obtain ⟨hkb, hl, hu⟩ := hY hc
    have := hn2 hc
    have he' : -err = DY * (1 + a) - DX * (1 + k) := by rw [he, hkb]; ring
    refine ⟨steps_le (by omega) h1 ha0 hak he' (by omega), by omega, by omega, by omega⟩

/-- After `n` turns the steps are exactly `DX` and `DY`. -/
theorem linv_end {DX DY n a b err : Int} (h1 : 0 ≤ DX) (h2 : 0 ≤ DY)
    (hn1 : DY ≤ DX → n = DX) (hn2 : DX ≤ DY → n = DY)
    (h : LInv DX DY n a b err) : a = DX ∧ b = DY := by
  have hb := linv_bounds h1 h2 hn1 hn2 (le_refl _) h
  obtain ⟨ha0, hak, hb0, hbk, he, hX, hY⟩ := h
  rcases le_total DY DX with hc | hc
  · obtain ⟨ha, hl, hu⟩ := hX hc
    have hnd := hn1 hc
    refine ⟨by omega, ?_⟩
    rcases lt_or_eq_of_le h1 with hp | hz
    · rw [ha, hnd] at he; have := steps_ge hp h2 hb0 he hl; omega
    · omega
  · obtain ⟨hb', hl, hu⟩ := hY hc
    have hnd := hn2 hc
    refine ⟨?_, by omega⟩
    rcases lt_or_eq_of_le h2 with hp | hz
    · have he' : -err = DY * (1 + a) - DX * (1 + DY) := by rw [he, hb', hnd]; ring
      have := steps_ge hp h1 ha0 he' (by omega); omega
    · omega

/-- `v` is on the segment from `X0`, `D` steps of size `s`. -/
def Seg (X0 s D v : Int) : Prop := ∃ t, 0 ≤ t ∧ t ≤ D ∧ v = X0 + s * t

/-- The points so far: they start at `(X0, Y0)`, end at `(x, y)`, are neighbors, and
are on the two segments. -/
def LList (X0 Y0 DX DY sx sy x y : Int) (l : List (Std.I32 × Std.I32)) : Prop :=
  (l[0]!).1.val = X0 ∧ (l[0]!).2.val = Y0 ∧
    (l[l.length - 1]!).1.val = x ∧ (l[l.length - 1]!).2.val = y ∧
    (∀ j, j + 1 < l.length → |(l[j + 1]!).1.val - (l[j]!).1.val| ≤ 1 ∧
      |(l[j + 1]!).2.val - (l[j]!).2.val| ≤ 1) ∧
    (∀ j < l.length, Seg X0 sx DX (l[j]!).1.val ∧ Seg Y0 sy DY (l[j]!).2.val)

/-- The state of the `line_points` loop after `k` turns. -/
def LState (X0 Y0 DX DY sx sy k err x y : Int) (l : List (Std.I32 × Std.I32)) : Prop :=
  ∃ a b, LInv DX DY k a b err ∧ x = X0 + sx * a ∧ y = Y0 + sy * b ∧
    (l.length : Int) = k + 1 ∧ LList X0 Y0 DX DY sx sy x y l

theorem get_append_lt {α : Type} [Inhabited α] {l t : List α} {j : Nat} (h : j < l.length) :
    (l ++ t)[j]! = l[j]! := by
  simp [List.getElem!_eq_getElem?_getD, List.getElem?_append_left h]

theorem get_append_end {α : Type} [Inhabited α] (l : List α) (p : α) :
    (l ++ [p])[l.length]! = p := by
  simp

/-- One turn of the loop adds one point and keeps the state. -/
theorem lstate_push {X0 Y0 DX DY sx sy n k a b err x y ex ey k' e' x' y' : Int}
    {l l' : List (Std.I32 × Std.I32)} {px py : Std.I32}
    (hDX : 0 ≤ DX) (hDY : 0 ≤ DY) (hn1 : DY ≤ DX → n = DX) (hn2 : DX ≤ DY → n = DY)
    (hkn : k + 1 ≤ n) (hsx : sx = 1 ∨ sx = -1) (hsy : sy = 1 ∨ sy = -1)
    (hinv : LInv DX DY k a b err) (hxa : x = X0 + sx * a) (hyb : y = Y0 + sy * b)
    (hlen : (l.length : Int) = k + 1) (hl : LList X0 Y0 DX DY sx sy x y l)
    (hx : (ex = 1 ∧ -DY ≤ 2 * err) ∨ (ex = 0 ∧ 2 * err < -DY))
    (hy : (ey = 1 ∧ 2 * err ≤ DX) ∨ (ey = 0 ∧ DX < 2 * err))
    (hk' : k' = k + 1) (he' : e' = err - ex * DY + ey * DX) (hx' : x' = x + sx * ex)
    (hy' : y' = y + sy * ey) (hl' : l' = l ++ [(px, py)]) (hp1 : px.val = x')
    (hp2 : py.val = y') :
    LState X0 Y0 DX DY sx sy k' e' x' y' l' := by
  have hinv' := linv_step hDX hDY hinv hx hy
  have hb := linv_bounds hDX hDY hn1 hn2 hkn hinv'
  have hk0 : 0 ≤ k := by obtain ⟨h1, h2, -⟩ := hinv; omega
  have hex : 0 ≤ ex ∧ ex ≤ 1 := by omega
  have hey : 0 ≤ ey ∧ ey ≤ 1 := by omega
  have ha : 0 ≤ a + ex := by obtain ⟨h1, -⟩ := hinv'; exact h1
  have hb0 : 0 ≤ b + ey := by obtain ⟨-, -, h1, -⟩ := hinv'; exact h1
  obtain ⟨hf1, hf2, hl1, hl2, hnear, hseg⟩ := hl
  have hL : 0 < l.length := by omega
  subst hl' hk' he'
  refine ⟨a + ex, b + ey, hinv', by rw [hx', hxa]; ring, by rw [hy', hyb]; ring,
    by simp; omega, ?_⟩
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩
  · rw [get_append_lt hL]; exact hf1
  · rw [get_append_lt hL]; exact hf2
  · simp only [List.length_append, List.length_singleton, Nat.add_sub_cancel, get_append_end]
    exact hp1
  · simp only [List.length_append, List.length_singleton, Nat.add_sub_cancel, get_append_end]
    exact hp2
  · intro j hj
    simp only [List.length_append, List.length_singleton] at hj
    by_cases hjl : j + 1 < l.length
    · rw [get_append_lt hjl, get_append_lt (by omega)]; exact hnear j hjl
    · have hj' : j = l.length - 1 := by omega
      have hj1 : j + 1 = l.length := by omega
      rw [hj1, get_append_end, get_append_lt (by omega), hj', hp1, hp2, hl1, hl2, hx', hy']
      rcases hsx with h | h <;> rcases hsy with h' | h' <;> rw [h, h'] <;>
        simp only [one_mul, neg_one_mul, add_sub_cancel_left, abs_neg] <;>
        rw [abs_le, abs_le] <;> omega
  · intro j hj
    simp only [List.length_append, List.length_singleton] at hj
    by_cases hjl : j < l.length
    · rw [get_append_lt hjl]; exact hseg j hjl
    · have hj' : j = l.length := by omega
      rw [hj', get_append_end, hp1, hp2, hx', hy', hxa, hyb]
      exact ⟨⟨a + ex, ha, hb.1, by ring⟩, ⟨b + ey, hb0, hb.2.1, by ring⟩⟩

theorem line_points_loop_spec (dx dy sx sy n err x y : Std.I32)
    (out : alloc.vec.Vec (Std.I32 × Std.I32)) (k : Std.I32) (X0 Y0 : Int)
    (hdx : 0 ≤ dx.val ∧ dx.val ≤ 131072) (hdy : -131072 ≤ dy.val ∧ dy.val ≤ 0)
    (hsx : sx.val = 1 ∨ sx.val = -1) (hsy : sy.val = 1 ∨ sy.val = -1)
    (hn1 : -dy.val ≤ dx.val → n.val = dx.val) (hn2 : dx.val ≤ -dy.val → n.val = -dy.val)
    (hX : -65536 ≤ X0 ∧ X0 ≤ 65536 ∧ -65536 ≤ X0 + sx.val * dx.val ∧ X0 + sx.val * dx.val ≤ 65536)
    (hY : -65536 ≤ Y0 ∧ Y0 ≤ 65536 ∧ -65536 ≤ Y0 + sy.val * -dy.val ∧
      Y0 + sy.val * -dy.val ≤ 65536)
    (hk : 0 ≤ k.val ∧ k.val ≤ n.val)
    (hs : LState X0 Y0 dx.val (-dy.val) sx.val sy.val k.val err.val x.val y.val out.val) :
    line_points_loop dx dy sx sy n err x y out k ⦃ r =>
      ∃ e xx yy, LState X0 Y0 dx.val (-dy.val) sx.val sy.val n.val e xx yy r.val ⦄ := by
  unfold line_points_loop
  apply loop.spec_decr_nat
    (measure := fun (s : Std.I32 × Std.I32 × Std.I32 × alloc.vec.Vec (Std.I32 × Std.I32) × Std.I32) =>
      (n.val - s.2.2.2.2.val).toNat)
    (inv := fun s => 0 ≤ s.2.2.2.2.val ∧ s.2.2.2.2.val ≤ n.val ∧
      LState X0 Y0 dx.val (-dy.val) sx.val sy.val s.2.2.2.2.val s.1.val s.2.1.val s.2.2.1.val
        s.2.2.2.1.val)
  · rintro ⟨err, x, y, out, k⟩ ⟨hk0, hkn, hs⟩
    simp only at hk0 hkn hs
    unfold line_points_loop.body
    obtain ⟨a, b, hinv, hxa, hyb, hlen, hlist⟩ := hs
    have hbd := linv_bounds hdx.1 (by omega) hn1 hn2 hkn hinv
    have hab := hinv
    obtain ⟨ha0, hak, hb0, hbk, -, -, -⟩ := hab
    have hxr : -65536 ≤ x.val ∧ x.val ≤ 65536 := by
      rcases hsx with h | h <;> rw [h] at hxa hX <;> omega
    have hyr : -65536 ≤ y.val ∧ y.val ≤ 65536 := by
      rcases hsy with h | h <;> rw [h] at hyb hY <;> omega
    have hn : n.val ≤ 131072 := by
      rcases le_total (-dy.val) dx.val with h | h
      · rw [hn1 h]; omega
      · rw [hn2 h]; omega
    have hsxb : -1 ≤ sx.val ∧ sx.val ≤ 1 := by omega
    have hsyb : -1 ≤ sy.val ∧ sy.val ≤ 1 := by omega
    step*
    · split <;> step* <;> split <;> step*
      all_goals
        simp only [IScalar.lt_equiv, IScalar.le_equiv, ge_iff_le, not_le] at *
        refine ⟨by omega, by omega, ?_, by omega⟩
      · exact lstate_push (ex := 1) (ey := 1) hdx.1 (by omega) hn1 hn2 (by omega) hsx hsy
          hinv hxa hyb hlen hlist (by omega) (by omega) (by omega) (by omega)
          (by omega) (by omega) out1_post (by omega) (by omega)
      · exact lstate_push (ex := 1) (ey := 0) hdx.1 (by omega) hn1 hn2 (by omega) hsx hsy
          hinv hxa hyb hlen hlist (by omega) (by omega) (by omega) (by omega)
          (by omega) (by omega) out1_post (by omega) (by omega)
      · exact lstate_push (ex := 0) (ey := 1) hdx.1 (by omega) hn1 hn2 (by omega) hsx hsy
          hinv hxa hyb hlen hlist (by omega) (by omega) (by omega) (by omega)
          (by omega) (by omega) out1_post (by omega) (by omega)
    · have hkn' : k.val = n.val := by scalar_tac
      rw [← hkn']
      exact ⟨_, _, _, a, b, hinv, hxa, hyb, hlen, hlist⟩
  · exact ⟨hk.1, hk.2, hs⟩

/-- P3: the points of a line from `(X0, Y0)` to `(X1, Y1)`. There are
`max |dx| |dy| + 1` of them. The first is `(X0, Y0)` and the last is `(X1, Y1)`.
Each point is a neighbor (8-way) of the one before. All of them are in the bounding
box of the two ends. -/
structure LineOk (X0 Y0 X1 Y1 : Int) (l : List (Std.I32 × Std.I32)) : Prop where
  len : (l.length : Int) = max |X1 - X0| |Y1 - Y0| + 1
  first : (l[0]!).1.val = X0 ∧ (l[0]!).2.val = Y0
  last : (l[l.length - 1]!).1.val = X1 ∧ (l[l.length - 1]!).2.val = Y1
  near : ∀ j, j + 1 < l.length → |(l[j + 1]!).1.val - (l[j]!).1.val| ≤ 1 ∧
    |(l[j + 1]!).2.val - (l[j]!).2.val| ≤ 1
  box : ∀ j < l.length, min X0 X1 ≤ (l[j]!).1.val ∧ (l[j]!).1.val ≤ max X0 X1 ∧
    min Y0 Y1 ≤ (l[j]!).2.val ∧ (l[j]!).2.val ≤ max Y0 Y1

/-- A step `s` of `±1` toward `X1`, `|X1 - X0|` times, ends at `X1`. -/
theorem walk_end {X0 X1 s : Int} (h1 : X0 < X1 → s = 1) (h2 : ¬ X0 < X1 → s = -1) :
    X0 + s * |X1 - X0| = X1 := by
  by_cases h : X0 < X1
  · rw [h1 h, abs_of_pos (by omega)]; ring
  · rw [h2 h, abs_of_nonpos (by omega)]; ring

theorem seg_box {X0 X1 s D v : Int} (h1 : X0 < X1 → s = 1) (h2 : ¬ X0 < X1 → s = -1)
    (hD : D = |X1 - X0|) (hv : Seg X0 s D v) : min X0 X1 ≤ v ∧ v ≤ max X0 X1 := by
  obtain ⟨t, ht0, htD, rfl⟩ := hv
  by_cases h : X0 < X1
  · rw [h1 h]; rw [hD, abs_of_pos (by omega)] at htD; omega
  · rw [h2 h]; rw [hD, abs_of_nonpos (by omega)] at htD; omega

/-- The end state of the loop gives P3. -/
theorem lineOk_of_state {X0 Y0 X1 Y1 sx sy n e x y : Int} {l : List (Std.I32 × Std.I32)}
    (hsx1 : X0 < X1 → sx = 1) (hsx2 : ¬ X0 < X1 → sx = -1)
    (hsy1 : Y0 < Y1 → sy = 1) (hsy2 : ¬ Y0 < Y1 → sy = -1)
    (hn1 : |Y1 - Y0| ≤ |X1 - X0| → n = |X1 - X0|) (hn2 : |X1 - X0| ≤ |Y1 - Y0| → n = |Y1 - Y0|)
    (h : LState X0 Y0 |X1 - X0| |Y1 - Y0| sx sy n e x y l) : LineOk X0 Y0 X1 Y1 l := by
  obtain ⟨a, b, hinv, hxa, hyb, hlen, hf1, hf2, hl1, hl2, hnear, hseg⟩ := h
  obtain ⟨ha, hb⟩ := linv_end (abs_nonneg _) (abs_nonneg _) hn1 hn2 hinv
  have hn : n = max |X1 - X0| |Y1 - Y0| := by
    rcases le_total |Y1 - Y0| |X1 - X0| with h | h
    · rw [hn1 h, max_eq_left h]
    · rw [hn2 h, max_eq_right h]
  refine ⟨by rw [hlen, hn], ⟨hf1, hf2⟩, ⟨?_, ?_⟩, hnear, fun j hj => ?_⟩
  · rw [hl1, hxa, ha]; exact walk_end hsx1 hsx2
  · rw [hl2, hyb, hb]; exact walk_end hsy1 hsy2
  · obtain ⟨h1, h2⟩ := hseg j hj
    exact ⟨(seg_box hsx1 hsx2 rfl h1).1, (seg_box hsx1 hsx2 rfl h1).2,
      (seg_box hsy1 hsy2 rfl h2).1, (seg_box hsy1 hsy2 rfl h2).2⟩

/-- With clamped ends, there are at most `131073` points, and each one is clamped. -/
theorem LineOk.small {X0 Y0 X1 Y1 : Int} {l : List (Std.I32 × Std.I32)}
    (hX0 : -65536 ≤ X0 ∧ X0 ≤ 65536) (hY0 : -65536 ≤ Y0 ∧ Y0 ≤ 65536)
    (hX1 : -65536 ≤ X1 ∧ X1 ≤ 65536) (hY1 : -65536 ≤ Y1 ∧ Y1 ≤ 65536)
    (h : LineOk X0 Y0 X1 Y1 l) :
    l.length ≤ 131073 ∧ ∀ j < l.length,
      -65536 ≤ (l[j]!).1.val ∧ (l[j]!).1.val ≤ 65536 ∧
      -65536 ≤ (l[j]!).2.val ∧ (l[j]!).2.val ≤ 65536 := by
  have hx : |X1 - X0| ≤ 131072 := abs_le.mpr ⟨by omega, by omega⟩
  have hy : |Y1 - Y0| ≤ 131072 := abs_le.mpr ⟨by omega, by omega⟩
  have hl := h.len
  have : max |X1 - X0| |Y1 - Y0| ≤ 131072 := max_le hx hy
  refine ⟨by omega, fun j hj => ?_⟩
  have := h.box j hj
  have := min_le_left X0 X1; have := le_max_left X0 X1
  have := min_le_right X0 X1; have := le_max_right X0 X1
  have := min_le_left Y0 Y1; have := le_max_left Y0 Y1
  have := min_le_right Y0 Y1; have := le_max_right Y0 Y1
  have := max_le_iff.mpr ⟨hX0.2, hX1.2⟩
  have := le_min_iff.mpr ⟨hX0.1, hX1.1⟩
  have := max_le_iff.mpr ⟨hY0.2, hY1.2⟩
  have := le_min_iff.mpr ⟨hY0.1, hY1.1⟩
  omega

/-- P3 and P10 for `line_points`: it returns `ok` for all inputs, and the points
are the line between the clamped ends (see `LineOk`). There are at most `131073`
points, and each one is in `-65536..=65536`. -/
@[step]
theorem line_points_spec (x0 y0 x1 y1 : Std.I32) :
    line_points x0 y0 x1 y1 ⦃ pts =>
      LineOk (clampC x0.val) (clampC y0.val) (clampC x1.val) (clampC y1.val) pts.val ∧
      pts.val.length ≤ 131073 ∧ ∀ j < pts.val.length,
        -65536 ≤ (pts.val[j]!).1.val ∧ (pts.val[j]!).1.val ≤ 65536 ∧
        -65536 ≤ (pts.val[j]!).2.val ∧ (pts.val[j]!).2.val ≤ 65536 ⦄ := by
  unfold line_points
  have b0 := clampC_bounds x0.val
  have b1 := clampC_bounds y0.val
  have b2 := clampC_bounds x1.val
  have b3 := clampC_bounds y1.val
  step*
  obtain ⟨sx, hsx, hsx1, hsx2⟩ : ∃ s : Std.I32,
      (if x01 < x11 then ok 1#i32 else ok (-1)#i32 : Result Std.I32) = ok s ∧
      (x01.val < x11.val → s.val = 1) ∧ (¬ x01.val < x11.val → s.val = -1) := by
    split <;> simp_all
  obtain ⟨sy, hsy, hsy1, hsy2⟩ : ∃ s : Std.I32,
      (if y01 < y11 then ok 1#i32 else ok (-1)#i32 : Result Std.I32) = ok s ∧
      (y01.val < y11.val → s.val = 1) ∧ (¬ y01.val < y11.val → s.val = -1) := by
    split <;> simp_all
  simp only [hsx, hsy]
  step*
  obtain ⟨n, hn, hn1, hn2⟩ : ∃ m : Std.I32,
      (if dx > i3 then ok dx else ok i3 : Result Std.I32) = ok m ∧
      (i3.val ≤ dx.val → m.val = dx.val) ∧ (dx.val ≤ i3.val → m.val = i3.val) := by
    split <;> simp_all
    scalar_tac
  simp only [hn]
  step*
  have hDX : dx.val = |clampC x1.val - clampC x0.val| := by rw [dx_post, i_post, x11_post, x01_post]
  have hDY : -dy.val = |clampC y1.val - clampC y0.val| := by
    rw [dy_post, i2_post, i1_post, y11_post, y01_post]; ring
  have hdx0 := abs_nonneg (clampC x1.val - clampC x0.val)
  have hdy0 := abs_nonneg (clampC y1.val - clampC y0.val)
  have hdx1 : |clampC x1.val - clampC x0.val| ≤ 131072 := abs_le.mpr ⟨by omega, by omega⟩
  have hdy1 : |clampC y1.val - clampC y0.val| ≤ 131072 := abs_le.mpr ⟨by omega, by omega⟩
  rw [x01_post, x11_post] at hsx1 hsx2
  rw [y01_post, y11_post] at hsy1 hsy2
  have hwx := walk_end hsx1 hsx2
  have hwy := walk_end hsy1 hsy2
  have hsx' : sx.val = 1 ∨ sx.val = -1 := by by_cases h : clampC x0.val < clampC x1.val <;> simp_all
  have hsy' : sy.val = 1 ∨ sy.val = -1 := by by_cases h : clampC y0.val < clampC y1.val <;> simp_all
  have hinit : LState (clampC x0.val) (clampC y0.val) dx.val (-dy.val) sx.val sy.val
      (0#i32).val err.val x01.val y01.val out.val := by
    have he : err.val = dx.val - -dy.val := by omega
    have hl : out.val = [(x01, y01)] := by simp [out_post]
    refine ⟨0, 0, by rw [he]; exact linv_init (by omega) (by omega), by simp [x01_post],
      by simp [y01_post], by simp [hl], ?_⟩
    refine ⟨by simp [hl, x01_post], by simp [hl, y01_post], by simp [hl], by simp [hl],
      fun j hj => by simp [hl] at hj, fun j hj => ?_⟩
    simp only [hl, List.length_singleton, Nat.lt_one_iff] at hj
    subst hj
    simp only [hl]
    exact ⟨⟨0, le_refl _, by omega, by simp [x01_post]⟩, ⟨0, le_refl _, by omega, by simp [y01_post]⟩⟩
  apply WP.spec_mono (line_points_loop_spec dx dy sx sy n err x01 y01 out 0#i32
    (clampC x0.val) (clampC y0.val) (by omega) (by omega) hsx' hsy' (by omega) (by omega)
    ⟨b0.1, b0.2, by rw [hDX, hwx]; exact b2.1, by rw [hDX, hwx]; exact b2.2⟩
    ⟨b1.1, b1.2, by rw [hDY, hwy]; exact b3.1, by rw [hDY, hwy]; exact b3.2⟩
    (by simp; omega) hinit)
  rintro r ⟨e, xx, yy, hst⟩
  rw [hDX, hDY] at hst
  have hok := lineOk_of_state hsx1 hsx2 hsy1 hsy2 (by omega) (by omega) hst
  exact ⟨hok, hok.small b0 b1 b2 b3⟩

end Logbook.Paint
