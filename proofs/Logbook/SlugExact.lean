import Logbook.Slug
import Mathlib.Tactic.IntervalCases
open Aeneas Aeneas.Std Result logbook_core

/-! Exact slugs (spec 7.6): T6, T7, T8, T9, with T12 (no panic) built in.

The proofs first show that `make_slug` computes `slugOf`, a plain Lean function.
Then each theorem is a fact about `slugOf`. -/

namespace Logbook

/-! ## Bytes -/

/-- `A-Z`. -/
def upperChar (c : Std.U8) : Prop := 65 ≤ c.val ∧ c.val ≤ 90

/-- An ASCII letter or digit: `A-Z`, `a-z`, or `0-9`. -/
def asciiAlnum (c : Std.U8) : Prop := upperChar c ∨ slugChar c

/-- `lower` as a plain function: `A-Z` becomes `a-z`. Other bytes do not change. -/
def lowerU (c : Std.U8) : Std.U8 :=
  if 65 ≤ c.val ∧ c.val ≤ 90 then ⟨c.bv + 32#8⟩ else c

/-- `a-z` becomes `A-Z`. Other bytes do not change. -/
def upperU (c : Std.U8) : Std.U8 :=
  if 97 ≤ c.val ∧ c.val ≤ 122 then ⟨c.bv - 32#8⟩ else c

/-- The title in uppercase, byte by byte. -/
def upper (t : Slice Std.U8) : Slice Std.U8 :=
  Slice.from (t.val.map upperU) (by simp)

theorem lowerU_val (c : Std.U8) :
    (lowerU c).val = if 65 ≤ c.val ∧ c.val ≤ 90 then c.val + 32 else c.val := by
  unfold lowerU
  have h256 := U8.lt_succ_max c
  split
  · show (c.bv + 32#8).toNat = c.val + 32
    rw [BitVec.toNat_add]; simp; omega
  · rfl

theorem upperU_val (c : Std.U8) :
    (upperU c).val = if 97 ≤ c.val ∧ c.val ≤ 122 then c.val - 32 else c.val := by
  unfold upperU
  have h256 := U8.lt_succ_max c
  split
  · show (c.bv - 32#8).toNat = c.val - 32
    rw [BitVec.toNat_sub]; simp; omega
  · rfl

theorem lowerU_upperU (c : Std.U8) : lowerU (upperU c) = lowerU c := by
  rw [UScalar.eq_equiv, lowerU_val, lowerU_val, upperU_val]
  split <;> split <;> (try split) <;> omega

theorem lowerU_of_not_upper {c : Std.U8} (h : ¬ upperChar c) : lowerU c = c := by
  unfold upperChar at h
  simp [lowerU, h]

theorem slugChar_lowerU_iff (c : Std.U8) : slugChar (lowerU c) ↔ asciiAlnum c := by
  unfold asciiAlnum slugChar
  rw [lowerU_val]
  unfold upperChar
  split <;> omega

instance (c : Std.U8) : Decidable (slugChar c) :=
  inferInstanceAs (Decidable ((97 ≤ c.val ∧ c.val ≤ 122) ∨ (48 ≤ c.val ∧ c.val ≤ 57)))

@[step]
theorem lower_exact (c : Std.U8) : slug.lower c ⦃ r => r = lowerU c ⦄ := by
  unfold slug.lower
  split <;> (try split) <;> step* <;> (rw [UScalar.eq_equiv, lowerU_val]; split <;> scalar_tac)

/-! ## The model -/

/-- The loop of `make_slug` as a plain function. The arguments are the rest of the
title, the output so far, and the gap flag. -/
def slugLoop : List Std.U8 → List Std.U8 → Bool → List Std.U8
  | [], out, _ => out
  | c :: cs, out, gap =>
    if slugChar (lowerU c) then
      if gap ∧ out ≠ [] then
        if out.length + 2 ≤ 80 then slugLoop cs (out ++ [45#u8, lowerU c]) false else out
      else if out.length + 1 ≤ 80 then slugLoop cs (out ++ [lowerU c]) false else out
    else slugLoop cs out (if lowerU c = 39#u8 then gap else true)

theorem make_slug_loop_exact (title : Slice Std.U8) (out : alloc.vec.Vec Std.U8) (gap : Bool)
    (i : Std.Usize) (hi : i.val ≤ title.length) (hout : out.val.length ≤ 80) :
    slug.make_slug_loop title out gap i ⦃ r => r.val = slugLoop (title.val.drop i.val) out.val gap ⦄ := by
  unfold slug.make_slug_loop
  apply loop.spec_decr_nat
    (measure := fun (x : alloc.vec.Vec Std.U8 × Bool × Std.Usize) => title.length - x.2.2.val)
    (inv := fun x => x.2.2.val ≤ title.length ∧ x.1.val.length ≤ 80 ∧
      slugLoop (title.val.drop x.2.2.val) x.1.val x.2.1 = slugLoop (title.val.drop i.val) out.val gap)
  · rintro ⟨out', gap', i'⟩ ⟨hi', hout', heq⟩
    simp only at hi' hout' heq
    rw [← heq]
    clear heq hi hout
    unfold slug.make_slug_loop.body slug.SLUG_MAX
    step*
    all_goals try (refine ⟨by scalar_tac, ?_, ?_, by scalar_tac⟩)
    all_goals first
      | scalar_tac
      -- The end of the title.
      | (rw [List.drop_eq_nil_of_le (by scalar_tac)]; rfl)
      -- A slug char: unfold one step of the model.
      | (have hlt : i'.val < title.val.length := by scalar_tac
         rw [List.drop_eq_getElem_cons hlt, ← i2_post]
         simp only [slugLoop, ← c_post]
         have hc : slugChar c := b_post.mp (by assumption)
         have hl : (↑out'.len : Nat) = (↑out' : List Std.U8).length := by simp
         split_ifs <;> simp_all [List.ne_nil_iff_length_pos] <;> omega)
      -- Not a slug char: only the gap flag changes.
      | (have hlt : i'.val < title.val.length := by scalar_tac
         have hc : ¬ slugChar c := fun h => by simp_all
         split <;> step* <;> refine ⟨by scalar_tac, by scalar_tac, ?_, by scalar_tac⟩ <;>
           rw [List.drop_eq_getElem_cons hlt, ← i2_post] <;>
           simp only [slugLoop, ← c_post, hc, if_false, i3_post] <;>
           simp_all [UScalar.eq_equiv])
  · exact ⟨hi, hout, rfl⟩

/-! ## The fallback -/

/-- A character as one byte. -/
def asciiByte (ch : Char) : Std.U8 := ⟨BitVec.ofNat 8 ch.toNat⟩

/-- The decimal digits of `n` as ASCII bytes, as `format!("{n}")` writes them. -/
def decimal (n : Nat) : List Std.U8 := (Nat.toDigits 10 n).map asciiByte

theorem digit_byte {d : Nat} (h : d < 10) : (asciiByte (Nat.digitChar d)).val = 48 + d := by
  interval_cases d <;> rfl

theorem decimal_of_lt {n : Nat} (h : n < 10) : decimal n = [asciiByte (Nat.digitChar n)] := by
  simp [decimal, Nat.toDigits_of_lt_base h]

theorem decimal_of_ge {n : Nat} (h : 10 ≤ n) :
    decimal n = decimal (n / 10) ++ [asciiByte (Nat.digitChar (n % 10))] := by
  simp [decimal, Nat.toDigits_of_base_le (by omega) h]

theorem decimal_length (id : Std.U64) : (decimal id.val).length ≤ 20 := by
  have h := U64.lt_succ_max id
  simp only [decimal, List.length_map]
  rw [Nat.length_toDigits_le_iff (by omega) (by omega)]
  omega

/-- The digit loop writes the decimal digits of `id`, last digit first. -/
theorem fallback_loop0_exact (id : Std.U64) (digits : alloc.vec.Vec Std.U8) (n : Std.U64)
    (h : decimal id.val = decimal n.val ++ digits.val.reverse) :
    slug.fallback_loop0 digits n ⦃ r => r.val.reverse = decimal id.val ⦄ := by
  unfold slug.fallback_loop0
  apply loop.spec_decr_nat
    (measure := fun (x : alloc.vec.Vec Std.U8 × Std.U64) => x.2.val)
    (inv := fun x => decimal id.val = decimal x.2.val ++ x.1.val.reverse)
  · rintro ⟨digits, n⟩ h
    simp only at h
    unfold slug.fallback_loop0.body
    have hlen : digits.val.length ≤ 20 := by
      have := decimal_length id
      rw [h] at this
      simp at this
      omega
    simp only [lift]
    step*
    all_goals
      have hb : i2 = asciiByte (Nat.digitChar (n.val % 10)) := by
        rw [UScalar.eq_equiv, digit_byte (by omega)]
        simp only [UScalar.cast_val_eq, UScalarTy.numBits] at i2_post
        scalar_tac
    · have hn : n.val < 10 := by scalar_tac
      rw [h, decimal_of_lt hn, digits1_post, hb, Nat.mod_eq_of_lt hn]
      simp
    · have hn : 10 ≤ n.val := by scalar_tac
      refine ⟨?_, by scalar_tac⟩
      rw [h, decimal_of_ge hn, digits1_post, hb, n1_post]
      simp
  · exact h

/-- `fallback id` is exactly `post-` followed by the decimal digits of `id`. -/
theorem fallback_exact (id : Std.U64) :
    slug.fallback id ⦃ r => r.val = postDash ++ decimal id.val ⦄ := by
  unfold slug.fallback
  apply WP.spec_bind (fallback_loop0_exact id (alloc.vec.Vec.new Std.U8) id (by simp))
  intro digits hdig
  have hlen : digits.val.length ≤ 20 := by
    have := decimal_length id
    rw [← hdig] at this
    simpa using this
  simp only [lift]
  step*
  case hout =>
    have h := congrArg Slice.val out_post
    simp only [make_slug_bytes] at h
    unfold postDash
    rw [h]
    simp
    rfl

/-! ## `make_slug` computes `slugOf` -/

/-- The slug of a title, as a plain function. -/
def slugOf (t : List Std.U8) (id : Nat) : List Std.U8 :=
  if slugLoop t [] false = [] then postDash ++ decimal id else slugLoop t [] false

theorem make_slug_exact (title : Slice Std.U8) (id : Std.U64) :
    slug.make_slug title id ⦃ r => r.val = slugOf title.val id.val ⦄ := by
  unfold slug.make_slug
  apply WP.spec_bind (make_slug_loop_exact title (alloc.vec.Vec.new Std.U8) false 0#usize
    (by simp) (by simp))
  intro out hout
  simp at hout
  dsimp only
  unfold slugOf
  simp only [← hout]
  split
  · have : out.val = [] := by
      have : out.val.length = 0 := by scalar_tac
      simpa using this
    simp only [this, if_true]
    exact fallback_exact id
  · rename_i hne
    have : out.val ≠ [] := by
      intro h; apply hne
      have : out.val.length = 0 := by rw [h]; rfl
      clear hout; scalar_tac
    simp only [WP.spec_ok, this, if_false]

/-! ## A valid slug maps to itself -/

theorem dash_not_slugChar : ¬ slugChar 45#u8 := by simp [slugChar]

theorem no_double_dash_mid {l1 l2 : List Std.U8}
    (h : List.IsChain noDD (l1 ++ [45#u8] ++ 45#u8 :: l2)) : False := by
  rw [List.append_assoc, List.singleton_append] at h
  have h2 := (List.isChain_append.mp h).2.1
  rw [List.isChain_cons_cons] at h2
  exact h2.1 ⟨rfl, rfl⟩

/-- The loop invariant on a valid slug `s = p ++ r`: the output so far is `p`, where a
`-` at the end of `p` is held back in the gap flag. -/
theorem slugLoop_valid (s : List Std.U8) (hs : ValidSlug s) :
    ∀ (r p out : List Std.U8) (gap : Bool), p ++ r = s →
      out ++ (if gap then [45#u8] else []) = p → slugLoop r out gap = s
  | [], p, out, gap, hpr, hout => by
    simp only [List.append_nil] at hpr
    cases gap
    · simp only [Bool.false_eq_true, if_false, List.append_nil] at hout
      simp only [slugLoop]; rw [hout, hpr]
    · exfalso
      simp only [if_true] at hout
      have := hs.last 45#u8 (by rw [← hpr, ← hout]; simp)
      exact dash_not_slugChar this
  | c :: r, p, out, gap, hpr, hout => by
    have hcs : c ∈ s := by rw [← hpr]; simp
    have hlen : p.length + 1 ≤ 80 := by
      have := hs.len_le; rw [← hpr] at this; simp at this; omega
    rcases hs.chars c hcs with hc | hc
    · -- A slug char: the loop writes the held-back `-` (if any) and then `c`.
      have hl : lowerU c = c := lowerU_of_not_upper (by unfold upperChar; unfold slugChar at hc; omega)
      cases gap
      · simp only [Bool.false_eq_true, if_false, List.append_nil] at hout
        subst hout
        have h1 : out.length + 1 ≤ 80 := hlen
        simp only [slugLoop, hl, hc, if_true, Bool.false_eq_true, false_and, if_false, h1]
        exact slugLoop_valid s hs r (out ++ [c]) (out ++ [c]) false (by simp [← hpr]) (by simp)
      · simp only [if_true] at hout
        have hne : out ≠ [] := by
          intro h; subst h
          simp only [List.nil_append] at hout
          have := hs.head 45#u8 (by rw [← hpr, ← hout]; simp)
          exact dash_not_slugChar this
        have h2 : out.length + 2 ≤ 80 := by rw [← hout] at hlen; simp at hlen; omega
        simp only [slugLoop, hl, hc, if_true, true_and, hne, ne_eq, not_false_eq_true, h2]
        exact slugLoop_valid s hs r (p ++ [c]) (out ++ [45#u8, c]) false
          (by simp [← hpr]) (by simp [← hout])
    · -- A `-`: the loop holds it back in the gap flag.
      subst hc
      have hl : lowerU 45#u8 = 45#u8 := lowerU_of_not_upper (by simp [upperChar])
      have h39 : (45#u8 : Std.U8) ≠ 39#u8 := by decide
      cases gap
      · simp only [Bool.false_eq_true, if_false, List.append_nil] at hout
        simp only [slugLoop, hl, dash_not_slugChar, if_false, h39]
        exact slugLoop_valid s hs r (p ++ [45#u8]) out true (by simp [← hpr]) (by simp [hout])
      · exfalso
        simp only [if_true] at hout
        apply no_double_dash_mid (l1 := out) (l2 := r)
        rw [hout, hpr]
        exact hs.no_double_dash

theorem slugOf_valid {s : List Std.U8} (hs : ValidSlug s) (id : Nat) : slugOf s id = s := by
  have h := slugLoop_valid s hs s [] [] false (by simp) (by simp)
  unfold slugOf
  rw [h]
  simp [List.ne_nil_of_length_pos hs.len_pos]

/-- `make_slug` maps a valid slug to itself. -/
theorem make_slug_of_valid (title : Slice Std.U8) (id : Std.U64) (h : ValidSlug title.val) :
    slug.make_slug title id ⦃ s => s.val = title.val ⦄ := by
  apply WP.spec_mono (make_slug_exact title id)
  intro s hs
  rw [hs, slugOf_valid h]

/-! ## The theorems -/

/-- T6 + T12: a plain title is its own slug. -/
theorem slug_plain (title : Slice Std.U8) (id : Std.U64) (hc : ∀ c ∈ title.val, slugChar c)
    (hpos : 0 < title.length) (hle : title.length ≤ 80) :
    slug.make_slug title id ⦃ s => s.val = title.val ⦄ :=
  make_slug_of_valid title id
    { len_pos := hpos
      len_le := hle
      chars := fun c h => Or.inl (hc c h)
      head := fun c h => hc c (List.mem_of_head? h)
      last := fun c h => hc c (List.mem_of_getLast? h)
      no_double_dash := chain_no_dash _ (fun c h => slugChar_ne_dash (hc c h)) }

theorem slugLoop_no_alnum : ∀ (t out : List Std.U8) (gap : Bool),
    (∀ c ∈ t, ¬ asciiAlnum c) → slugLoop t out gap = out
  | [], _, _, _ => rfl
  | c :: cs, out, gap, h => by
    have hc : ¬ slugChar (lowerU c) := by rw [slugChar_lowerU_iff]; exact h c (by simp)
    simp only [slugLoop, hc, if_false]
    exact slugLoop_no_alnum cs out _ (fun d hd => h d (by simp [hd]))

/-- T7 + T12: a title with no ASCII letter or digit gets `post-<id>`. -/
theorem slug_fallback (title : Slice Std.U8) (id : Std.U64) (h : ∀ c ∈ title.val, ¬ asciiAlnum c) :
    slug.make_slug title id ⦃ s => s.val = postDash ++ decimal id.val ⦄ := by
  apply WP.spec_mono (make_slug_exact title id)
  intro s hs
  rw [hs, slugOf, slugLoop_no_alnum _ _ _ h]
  simp

theorem slugLoop_upper : ∀ (t out : List Std.U8) (gap : Bool),
    slugLoop (t.map upperU) out gap = slugLoop t out gap
  | [], _, _ => rfl
  | c :: cs, out, gap => by
    simp only [List.map_cons, slugLoop, lowerU_upperU, slugLoop_upper cs]

/-- T8 + T12: case does not matter. -/
theorem slug_lower (title : Slice Std.U8) (id : Std.U64) :
    slug.make_slug (upper title) id ⦃ s => slug.make_slug title id = ok s ⦄ := by
  obtain ⟨s0, hs0, hv0⟩ := (WP.spec_equiv_exists _ _).mp (make_slug_exact title id)
  apply WP.spec_mono (make_slug_exact (upper title) id)
  intro s hs
  rw [hs0]
  congr 1
  apply alloc.vec.Vec.ext
  rw [hs, hv0]
  simp [upper, slugOf, Slice.from_val, slugLoop_upper]

/-- T9 + T12: the slug of a slug is the same slug. -/
theorem slug_idempotent (title : Slice Std.U8) (id : Std.U64) :
    slug.make_slug title id ⦃ s => slug.make_slug s.slice id = ok s ⦄ := by
  apply WP.spec_mono (make_slug_spec title id)
  intro s hs
  obtain ⟨s', hs', hv⟩ := (WP.spec_equiv_exists _ _).mp (make_slug_of_valid s.slice id hs)
  rw [hs']
  congr 1
  exact alloc.vec.Vec.ext _ _ hv

end Logbook
