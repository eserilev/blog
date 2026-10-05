import LogbookCore.Funs
import Mathlib.Data.List.Chain
open Aeneas Aeneas.Std Result logbook_core

/-! Slugs (spec 7.6): T4 and T5, with T12 (no panic) built in. `SlugExact.lean` has T6–T9. -/

namespace Logbook

/-- `a-z` or `0-9`. -/
def slugChar (c : Std.U8) : Prop :=
  (97 ≤ c.val ∧ c.val ≤ 122) ∨ (48 ≤ c.val ∧ c.val ≤ 57)

/-- Two neighbors that are not both `-`. -/
def noDD (a b : Std.U8) : Prop := ¬ (a = 45#u8 ∧ b = 45#u8)

/-- The slug rules of spec 4.3. -/
structure ValidSlug (s : List Std.U8) : Prop where
  len_pos : 0 < s.length
  len_le : s.length ≤ 80
  chars : ∀ c ∈ s, slugChar c ∨ c = 45#u8
  head : ∀ c ∈ s.head?, slugChar c
  last : ∀ c ∈ s.getLast?, slugChar c
  no_double_dash : List.IsChain noDD s

theorem slugChar_ne_dash {c : Std.U8} (h : slugChar c) : c ≠ 45#u8 := by
  intro hc; subst hc; simp [slugChar] at h

theorem valid_singleton {c : Std.U8} (hc : slugChar c) : ValidSlug [c] where
  len_pos := by simp
  len_le := by simp
  chars := by simp [hc]
  head := by simp [hc]
  last := by simp [hc]
  no_double_dash := List.isChain_singleton c

theorem valid_push {s : List Std.U8} {c : Std.U8} (h : ValidSlug s) (hc : slugChar c)
    (hl : s.length + 1 ≤ 80) : ValidSlug (s ++ [c]) where
  len_pos := by simp
  len_le := by simp; omega
  chars := by
    intro x hx
    simp only [List.mem_append, List.mem_singleton] at hx
    rcases hx with hx | rfl
    · exact h.chars x hx
    · exact Or.inl hc
  head := by
    intro x hx
    have hne : s ≠ [] := List.ne_nil_of_length_pos h.len_pos
    rw [List.head?_append_of_ne_nil _ hne] at hx
    exact h.head x hx
  last := by simp [hc]
  no_double_dash := by
    apply List.IsChain.append h.no_double_dash (List.isChain_singleton c)
    intro x _ y hy
    simp at hy; subst hy
    exact fun ⟨_, h2⟩ => slugChar_ne_dash hc h2

theorem valid_push_dash {s : List Std.U8} {c : Std.U8} (h : ValidSlug s) (hc : slugChar c)
    (hl : s.length + 2 ≤ 80) : ValidSlug (s ++ [45#u8] ++ [c]) where
  len_pos := by simp
  len_le := by simp; omega
  chars := by
    intro x hx
    simp only [List.mem_append, List.mem_singleton] at hx
    rcases hx with (hx | rfl) | rfl
    · exact h.chars x hx
    · exact Or.inr rfl
    · exact Or.inl hc
  head := by
    intro x hx
    have hne : s ≠ [] := List.ne_nil_of_length_pos h.len_pos
    rw [List.append_assoc, List.head?_append_of_ne_nil _ hne] at hx
    exact h.head x hx
  last := by simp [hc]
  no_double_dash := by
    rw [List.append_assoc]
    apply List.IsChain.append h.no_double_dash
    · simp only [List.singleton_append, List.isChain_pair]
      exact fun ⟨_, h2⟩ => slugChar_ne_dash hc h2
    · intro x hx y hy
      simp at hy; subst hy
      exact fun ⟨h1, _⟩ => slugChar_ne_dash (h.last x hx) h1

theorem step_push {s : List Std.U8} {c : Std.U8} (h : s = [] ∨ ValidSlug s) (hc : slugChar c)
    (hl : s.length + 1 ≤ 80) : ValidSlug (s ++ [c]) := by
  rcases h with rfl | h
  · exact valid_singleton hc
  · exact valid_push h hc hl

@[step]
theorem is_slug_char_spec (c : Std.U8) : slug.is_slug_char c ⦃ b => (b ↔ slugChar c) ⦄ := by
  unfold slug.is_slug_char slugChar
  split <;> (try split) <;> (try split) <;> simp <;> scalar_tac

@[step]
theorem lower_spec (c : Std.U8) : slug.lower c ⦃ _ => True ⦄ := by
  unfold slug.lower
  split <;> (try split) <;> step* <;> scalar_tac

theorem make_slug_loop_spec (title : Slice Std.U8) (out : alloc.vec.Vec Std.U8) (gap : Bool)
    (i : Std.Usize) (hi : i.val ≤ title.length) (hout : out.val = [] ∨ ValidSlug out.val) :
    slug.make_slug_loop title out gap i ⦃ r => r.val = [] ∨ ValidSlug r.val ⦄ := by
  unfold slug.make_slug_loop
  apply loop.spec_decr_nat
    (measure := fun (x : alloc.vec.Vec Std.U8 × Bool × Std.Usize) => title.length - x.2.2.val)
    (inv := fun x => x.2.2.val ≤ title.length ∧ (x.1.val = [] ∨ ValidSlug x.1.val))
  · rintro ⟨out, gap, i⟩ ⟨hi, hout⟩
    simp only at hi hout
    have hlen : out.val.length ≤ 80 := by
      rcases hout with h | h
      · simp [h]
      · exact h.len_le
    unfold slug.make_slug_loop.body slug.SLUG_MAX
    step*
    all_goals first
      -- Overflow side conditions: the slug is at most 80 bytes.
      | scalar_tac
      -- Stop without a push: the output does not change.
      | exact hout
      -- The branch after a `-` push that stops: it cannot run.
      | (exfalso; simp_all; scalar_tac)
      -- Push `-` and a char onto a nonempty slug.
      | (refine ⟨by scalar_tac, Or.inr ?_, by scalar_tac⟩
         have hne : (out.val).length ≠ 0 := by scalar_tac
         have hv : ValidSlug out.val := hout.resolve_left (fun h => hne (by simp [h]))
         rw [out2_post, out1_post]
         exact valid_push_dash hv (b_post.mp (by assumption)) (by scalar_tac))
      -- Push a char.
      | (refine ⟨by scalar_tac, Or.inr ?_, by scalar_tac⟩
         rw [out1_post]
         exact step_push hout (b_post.mp (by assumption)) (by scalar_tac))
      -- Not a slug char: only the gap flag changes.
      | (split <;> step* <;> exact ⟨by scalar_tac, hout, by scalar_tac⟩)
  · exact ⟨hi, hout⟩

/-- An ASCII digit, `0-9`. -/
def digit (c : Std.U8) : Prop := 48 ≤ c.val ∧ c.val ≤ 57

theorem ten_pow_bound {k n : Nat} (h : 10 ^ k * n < 2 ^ 64) (hn : 0 < n) : k ≤ 19 := by
  by_contra hk
  have h1 : 10 ^ 20 ≤ 10 ^ k := Nat.pow_le_pow_right (by norm_num) (by omega)
  have h2 : 10 ^ k ≤ 10 ^ k * n := Nat.le_mul_of_pos_right _ hn
  have : (2 : Nat) ^ 64 < 10 ^ 20 := by norm_num
  omega

/-- The digit loop: at most 20 ASCII digits, at least one. -/
theorem fallback_loop0_spec (id : Std.U64) (digits : alloc.vec.Vec Std.U8) (n : Std.U64)
    (hd : ∀ c ∈ digits.val, digit c) (hb : 10 ^ digits.val.length * n.val ≤ id.val)
    (h0 : n.val = 0 → digits.val = []) :
    slug.fallback_loop0 digits n ⦃ r => 0 < r.val.length ∧ r.val.length ≤ 20 ∧ ∀ c ∈ r.val, digit c ⦄ := by
  unfold slug.fallback_loop0
  apply loop.spec_decr_nat
    (measure := fun (x : alloc.vec.Vec Std.U8 × Std.U64) => x.2.val)
    (inv := fun x => (∀ c ∈ x.1.val, digit c) ∧ 10 ^ x.1.val.length * x.2.val ≤ id.val ∧
      (x.2.val = 0 → x.1.val = []))
  · rintro ⟨digits, n⟩ ⟨hd, hb, h0⟩
    simp only at hd hb h0
    have hid : id.val < 2 ^ 64 := id.hBounds
    -- Before the push there are at most 19 digits.
    have hlen : digits.val.length ≤ 19 := by
      rcases Nat.eq_zero_or_pos n.val with hn | hn
      · simp [h0 hn]
      · exact ten_pow_bound (by omega) hn
    unfold slug.fallback_loop0.body
    simp only [lift]
    step*
    all_goals
      have hdig : digit i2 := by
        unfold digit
        simp only [UScalar.cast_val_eq] at i2_post
        simp only [UScalarTy.numBits] at i2_post
        omega
      have hall : ∀ c ∈ digits1.val, digit c := by
        rw [digits1_post]
        intro c hc
        simp only [List.mem_append, List.mem_singleton] at hc
        rcases hc with hc | rfl
        · exact hd c hc
        · exact hdig
    · rw [digits1_post] at hall ⊢
      refine ⟨by simp, by simp; omega, hall⟩
    · have hn1 : n1.val ≠ 0 := by intro h; apply ‹¬n1 = 0#u64›; scalar_tac
      refine ⟨hall, ?_, fun h => absurd h hn1, by scalar_tac⟩
      rw [digits1_post, n1_post]
      simp only [List.length_append, List.length_singleton, pow_succ]
      calc 10 ^ digits.val.length * 10 * (n.val / 10)
          = 10 ^ digits.val.length * (10 * (n.val / 10)) := by ring
        _ ≤ 10 ^ digits.val.length * n.val := Nat.mul_le_mul_left _ (Nat.mul_div_le n.val 10)
        _ ≤ id.val := hb
  · exact ⟨hd, hb, h0⟩

@[simp]
theorem make_slug_bytes (n : Std.Usize) (l : List Std.U8) (h : l.length = n.val) :
    (Array.make n l h).to_slice.val = l := by
  simp [Array.to_slice]

/-- `post-` as bytes. -/
def postDash : List Std.U8 := [112#u8, 111#u8, 115#u8, 116#u8, 45#u8]

/-- The copy loop: `post-` followed by the digits in reverse order. -/
@[step]
theorem fallback_loop1_spec (digits out : alloc.vec.Vec Std.U8) (i : Std.Usize)
    (hi : i.val ≤ digits.val.length) (hdl : digits.val.length ≤ 20)
    (hout : out.val = postDash ++ (digits.val.drop i.val).reverse) :
    slug.fallback_loop1 digits out i ⦃ r => r.val = postDash ++ digits.val.reverse ⦄ := by
  unfold slug.fallback_loop1
  apply loop.spec_decr_nat
    (measure := fun (x : alloc.vec.Vec Std.U8 × Std.Usize) => x.2.val)
    (inv := fun x => x.2.val ≤ digits.val.length ∧ x.1.val = postDash ++ (digits.val.drop x.2.val).reverse)
  · rintro ⟨out, i⟩ ⟨hi, hout⟩
    simp only at hi hout
    have hlen : out.val.length ≤ 25 := by
      rw [hout]; simp [postDash]; omega
    unfold slug.fallback_loop1.body
    step*
    · refine ⟨by scalar_tac, ?_, by scalar_tac⟩
      have hd : digits.val.drop i1.val = i2 :: digits.val.drop i.val := by
        rw [List.drop_eq_getElem_cons (by scalar_tac), i2_post]
        congr 2
        scalar_tac
      rw [out1_post, hout, hd]
      simp
    · have : i.val = 0 := by scalar_tac
      simp [hout, this]
  · exact ⟨hi, hout⟩

/-- `fallback id` is `post-` followed by 1 to 20 ASCII digits, and never fails. -/
@[step]
theorem fallback_spec (id : Std.U64) :
    slug.fallback id ⦃ r => ∃ ds, r.val = postDash ++ ds ∧ 0 < ds.length ∧ ds.length ≤ 20 ∧ ∀ c ∈ ds, digit c ⦄ := by
  unfold slug.fallback
  apply WP.spec_bind (fallback_loop0_spec id (alloc.vec.Vec.new Std.U8) id (by simp) (by simp) (by simp))
  rintro digits ⟨hpos, hle, hall⟩
  simp only [lift]
  step*
  case hout =>
    have h := congrArg Slice.val out_post
    simp only [make_slug_bytes] at h
    unfold postDash
    rw [h]
    simp
    rfl
  exact ⟨digits.val.reverse, r_post, by simpa using hpos, by simpa using hle, by simpa using hall⟩

theorem digit_slugChar {c : Std.U8} (h : digit c) : slugChar c := Or.inr h

theorem chain_no_dash : ∀ (l : List Std.U8), (∀ c ∈ l, c ≠ 45#u8) → List.IsChain noDD l
  | [], _ => List.isChain_nil
  | a :: l, h => by
    rw [List.isChain_cons]
    exact ⟨fun _ _ ⟨h1, _⟩ => h a (by simp) h1, chain_no_dash l (fun c hc => h c (by simp [hc]))⟩

theorem valid_fallback {ds : List Std.U8} (hpos : 0 < ds.length) (hle : ds.length ≤ 20)
    (hall : ∀ c ∈ ds, digit c) : ValidSlug (postDash ++ ds) where
  len_pos := by simp [postDash]
  len_le := by simp [postDash]; omega
  chars := by
    intro c hc
    simp only [postDash, List.mem_append] at hc
    rcases hc with hc | hc
    · simp at hc
      rcases hc with rfl | rfl | rfl | rfl | rfl <;> simp [slugChar]
    · exact Or.inl (digit_slugChar (hall c hc))
  head := by simp [postDash, slugChar]
  last := by
    intro c hc
    rw [List.getLast?_append_of_ne_nil _ (List.ne_nil_of_length_pos hpos)] at hc
    exact digit_slugChar (hall c (List.mem_of_getLast? hc))
  no_double_dash := by
    apply List.IsChain.append
    · simp [postDash, List.isChain_cons_cons, noDD]
    · exact chain_no_dash ds (fun c hc => slugChar_ne_dash (digit_slugChar (hall c hc)))
    · intro x _ y hy
      have := hall y (List.mem_of_head? hy)
      exact fun ⟨_, h2⟩ => slugChar_ne_dash (digit_slugChar this) h2

/-- T4 + T5 + T12: every slug follows the rules of spec 4.3, for every title and id,
and `make_slug` never fails. -/
theorem make_slug_spec (title : Slice Std.U8) (id : Std.U64) :
    slug.make_slug title id ⦃ s => ValidSlug s.val ⦄ := by
  unfold slug.make_slug
  apply WP.spec_bind (make_slug_loop_spec title (alloc.vec.Vec.new Std.U8) false 0#usize (by simp) (by simp))
  intro out hout
  dsimp only
  split
  · apply WP.spec_mono (fallback_spec id)
    rintro r ⟨ds, hr, hpos, hle, hall⟩
    rw [hr]; exact valid_fallback hpos hle hall
  · rename_i hne
    simp only [WP.spec_ok]
    exact hout.resolve_left (fun h => hne (by scalar_tac))

/-- T4 in the words of the spec: a slug contains only `a-z`, `0-9`, and `-`. -/
theorem slug_charset (title : Slice Std.U8) (id : Std.U64) :
    slug.make_slug title id ⦃ s => ∀ c ∈ s.val,
      (97 ≤ c.val ∧ c.val ≤ 122) ∨ (48 ≤ c.val ∧ c.val ≤ 57) ∨ c.val = 45 ⦄ := by
  apply WP.spec_mono (make_slug_spec title id)
  intro s hs c hc
  rcases hs.chars c hc with (h | h) | rfl
  · exact Or.inl h
  · exact Or.inr (Or.inl h)
  · exact Or.inr (Or.inr rfl)

end Logbook
