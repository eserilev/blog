import LogbookCore.Funs
import Logbook.Media
open Aeneas Aeneas.Std Result logbook_core

/-! The CSRF decision (spec 7.6): T13, with T12 (no panic) built in. -/

namespace Logbook.Csrf

/-- ASCII `A-Z` to `a-z`, on byte values. -/
def lowerN (n : Nat) : Nat := if 65 ≤ n ∧ n ≤ 90 then n + 32 else n

/-- A space (32) or a tab (9). -/
def isSp (c : Std.U8) : Prop := c.val = 32 ∨ c.val = 9

/-- `application/json`, as bytes. -/
def jsonBytes : List Std.U8 :=
  [97#u8, 112#u8, 112#u8, 108#u8, 105#u8, 99#u8, 97#u8, 116#u8, 105#u8, 111#u8, 110#u8,
   47#u8, 106#u8, 115#u8, 111#u8, 110#u8]

/-- `multipart/form-data`, as bytes. -/
def multipartBytes : List Std.U8 :=
  [109#u8, 117#u8, 108#u8, 116#u8, 105#u8, 112#u8, 97#u8, 114#u8, 116#u8, 47#u8, 102#u8,
   111#u8, 114#u8, 109#u8, 45#u8, 100#u8, 97#u8, 116#u8, 97#u8]

example : jsonBytes.map (·.val) = "application/json".toList.map Char.toNat := by decide
example : multipartBytes.map (·.val) = "multipart/form-data".toList.map Char.toNat := by decide

/-- `m` is `w` (lowercase) in any ASCII case. -/
def EqLower (m w : List Std.U8) : Prop := m.map (fun c => lowerN c.val) = w.map (·.val)

/-- The media type of a `Content-Type` value is `application/json`: the value is
spaces and tabs, `application/json` in any case, spaces and tabs, then the end or
a `;`. This is the part before the first `;`, trimmed, compared without case. -/
def JsonType (ct : List Std.U8) : Prop :=
  ∃ a m b r, ct = a ++ m ++ b ++ r ∧ (∀ c ∈ a, isSp c) ∧ EqLower m jsonBytes ∧
    (∀ c ∈ b, isSp c) ∧ (∀ c ∈ r.head?, c.val = 59)

/-- The value starts with `multipart/form-data`, in any case. -/
def MultipartForm (ct : List Std.U8) : Prop := EqLower (ct.take 19) multipartBytes

/-! Lemmas about lists. -/

theorem gb_lt {a t : List Std.U8} {k : Nat} (h : k < a.length) : (a ++ t)[k]! = a[k]! := by
  simp [List.getElem!_eq_getElem?_getD, List.getElem?_append_left h]

theorem gb_ge {a t : List Std.U8} {k : Nat} (h : a.length ≤ k) :
    (a ++ t)[k]! = t[k - a.length]! := by
  simp [List.getElem!_eq_getElem?_getD, List.getElem?_append_right h]

theorem gb_mem {a : List Std.U8} {k : Nat} (h : k < a.length) : a[k]! ∈ a := by
  rw [getElem!_pos a k h]; exact List.getElem_mem h

theorem lowerN_sp {c : Std.U8} (h : isSp c) : lowerN c.val < 33 := by
  unfold isSp at h; unfold lowerN; split_ifs <;> omega

theorem eqLower_iff (m w : List Std.U8) :
    EqLower m w ↔ m.length = w.length ∧ ∀ k < w.length, lowerN m[k]!.val = w[k]!.val := by
  unfold EqLower
  constructor
  · intro h
    have hl : m.length = w.length := by simpa using congrArg List.length h
    refine ⟨hl, fun k hk => ?_⟩
    have := congrArg (fun l => l[k]?) h
    simp [List.getElem?_eq_getElem hk, List.getElem?_eq_getElem (hl ▸ hk)] at this
    rw [getElem!_pos m k (by omega), getElem!_pos w k hk]
    exact this
  · rintro ⟨hl, h⟩
    apply List.ext_getElem (by simp [hl])
    intro k h1 h2
    simp only [List.getElem_map]
    have := h k (by simpa using h2)
    rwa [getElem!_pos _ _ (by simp at h1; omega), getElem!_pos _ _ (by simpa using h2)] at this

/-! The Rust functions. -/

@[step]
theorem lower_spec (c : Std.U8) : csrf.lower c ⦃ d => d.val = lowerN c.val ⦄ := by
  unfold csrf.lower lowerN
  split
  · split
    · step*
    · simp only [WP.spec_ok]; split_ifs <;> scalar_tac
  · simp only [WP.spec_ok]; split_ifs <;> scalar_tac

@[step]
theorem is_space_spec (c : Std.U8) : csrf.is_space c ⦃ b => (b ↔ isSp c) ⦄ := by
  unfold csrf.is_space isSp
  split
  · simp_all
  · simp_all; scalar_tac

/-- `j` is the first index at or after `i` that is not a space or a tab, or the length. -/
def SkipOk (l : List Std.U8) (i j : Nat) : Prop :=
  i ≤ j ∧ j ≤ l.length ∧ (∀ k, i ≤ k → k < j → isSp l[k]!) ∧ (j < l.length → ¬ isSp l[j]!)

@[step]
theorem skip_spaces_spec (s : Slice Std.U8) (i : Std.Usize) (hi : i.val ≤ s.length) :
    csrf.skip_spaces s i ⦃ j => SkipOk s.val i.val j.val ⦄ := by
  unfold csrf.skip_spaces csrf.skip_spaces_loop
  apply loop.spec_decr_nat
    (measure := fun (j : Std.Usize) => s.length - j.val)
    (inv := fun j => i.val ≤ j.val ∧ j.val ≤ s.length ∧ ∀ k, i.val ≤ k → k < j.val → isSp s.val[k]!)
  · rintro j ⟨hij, hj, hsp⟩
    unfold csrf.skip_spaces_loop.body
    step*
    · refine ⟨by scalar_tac, by scalar_tac, ?_, by scalar_tac⟩
      intro k hk1 hk2
      by_cases hkj : k < j.val
      · exact hsp k hk1 hkj
      · have : k = j.val := by scalar_tac
        subst this
        rw [getElem!_pos _ _ (by scalar_tac)]
        simp_all
    · refine ⟨hij, hj, hsp, fun _ => ?_⟩
      rw [getElem!_pos _ _ (by scalar_tac)]
      simp_all
    · refine ⟨hij, hj, hsp, fun h => ?_⟩
      scalar_tac
  · exact ⟨le_refl _, hi, fun k h1 h2 => by omega⟩

theorem starts_with_lower_loop_spec (s want : Slice Std.U8) (at' j : Std.Usize)
    (hlen : at'.val + want.length ≤ s.length) (hj : j.val ≤ want.length)
    (hpre : ∀ k < j.val, lowerN s.val[at'.val + k]!.val = want.val[k]!.val) :
    csrf.starts_with_lower_loop s at' want j ⦃ b =>
      (b ↔ ∀ k < want.length, lowerN s.val[at'.val + k]!.val = want.val[k]!.val) ⦄ := by
  unfold csrf.starts_with_lower_loop
  apply loop.spec_decr_nat
    (measure := fun (j : Std.Usize) => want.length - j.val)
    (inv := fun j => j.val ≤ want.length ∧
      ∀ k < j.val, lowerN s.val[at'.val + k]!.val = want.val[k]!.val)
  · rintro j ⟨hj, hpre⟩
    unfold csrf.starts_with_lower_loop.body
    step*
    · -- A byte differs.
      rename_i hne
      simp only [Bool.false_eq_true, false_iff, not_forall]
      refine ⟨j.val, by scalar_tac, ?_⟩
      rw [getElem!_pos _ _ (by scalar_tac), getElem!_pos _ _ (by scalar_tac)]
      simp only [bne_iff_ne, ne_eq] at hne
      intro h
      apply hne
      apply UScalar.eq_of_val_eq
      simp_all
    · rename_i heq
      refine ⟨by scalar_tac, ?_, by scalar_tac⟩
      intro k hk
      by_cases hkj : k < j.val
      · exact hpre k hkj
      · have : k = j.val := by scalar_tac
        subst this
        rw [getElem!_pos _ _ (by scalar_tac), getElem!_pos _ _ (by scalar_tac)]
        simp only [bne_iff_ne, ne_eq, Decidable.not_not] at heq
        simp_all
  · exact ⟨hj, hpre⟩

@[step]
theorem starts_with_lower_spec (s want : Slice Std.U8) (at' : Std.Usize) :
    csrf.starts_with_lower s at' want ⦃ b =>
      (b ↔ at'.val + want.length ≤ s.length ∧
        ∀ k < want.length, lowerN s.val[at'.val + k]!.val = want.val[k]!.val) ⦄ := by
  unfold csrf.starts_with_lower
  step*
  apply WP.spec_mono (starts_with_lower_loop_spec s want at' 0#usize (by scalar_tac)
    (by scalar_tac) (by intro k hk; simp at hk))
  intro b hb
  rw [hb]
  constructor
  · intro h; exact ⟨by scalar_tac, h⟩
  · intro h; exact h.2

/-! From the scan to `JsonType`, and back. -/

/-- `l = take p ++ (take (q - p) (drop p) ++ drop q)`. -/
theorem split3 (l : List Std.U8) {p q : Nat} (h : p ≤ q) :
    l.take p ++ ((l.drop p).take (q - p) ++ l.drop q) = l := by
  have : l.drop q = (l.drop p).drop (q - p) := by
    rw [List.drop_drop]; congr 1; omega
  rw [this, List.take_append_drop, List.take_append_drop]

theorem json_of_scan (l : List Std.U8) (st e : Nat) (h1 : SkipOk l 0 st)
    (hm : st + 16 ≤ l.length)
    (hmatch : ∀ k < 16, lowerN l[st + k]!.val = jsonBytes[k]!.val)
    (h2 : SkipOk l (st + 16) e) (hend : e = l.length ∨ l[e]!.val = 59) : JsonType l := by
  obtain ⟨-, -, hsp1, -⟩ := h1
  obtain ⟨he1, he2, hsp2, -⟩ := h2
  refine ⟨l.take st, (l.drop st).take 16, (l.drop (st + 16)).take (e - (st + 16)), l.drop e,
    ?_, ?_, ?_, ?_, ?_⟩
  · have d1 : (l.drop st).drop 16 = l.drop (st + 16) := by rw [List.drop_drop]
    have d2 : (l.drop (st + 16)).drop (e - (st + 16)) = l.drop e := by
      rw [List.drop_drop]; congr 1; omega
    conv_lhs => rw [← List.take_append_drop st l, ← List.take_append_drop 16 (l.drop st), d1,
      ← List.take_append_drop (e - (st + 16)) (l.drop (st + 16)), d2]
    simp only [List.append_assoc]
  · intro c hc
    obtain ⟨k, hk, rfl⟩ := List.mem_iff_getElem.mp hc
    have := hsp1 k (by omega) (by simp at hk; omega)
    rw [getElem!_pos l k (by simp at hk; omega)] at this
    simpa using this
  · rw [eqLower_iff]
    refine ⟨by simp [jsonBytes]; omega, fun k hk => ?_⟩
    simp only [jsonBytes, List.length_cons, List.length_nil] at hk
    rw [← hmatch k hk]
    congr 2
    rw [getElem!_pos _ _ (by simp; omega), getElem!_pos _ _ (by omega)]
    simp [List.getElem_take, List.getElem_drop]
  · intro c hc
    obtain ⟨k, hk, rfl⟩ := List.mem_iff_getElem.mp hc
    simp only [List.length_take, List.length_drop] at hk
    have := hsp2 (st + 16 + k) (by omega) (by omega)
    rw [getElem!_pos l _ (by omega)] at this
    simpa using this
  · intro c hc
    rcases hend with rfl | hend
    · simp at hc
    · have hlt : e < l.length := by
        rcases Nat.lt_or_ge e l.length with h | h
        · exact h
        · rw [getElem!_neg l e (by omega)] at hend; simp at hend
      rw [List.head?_drop, List.getElem?_eq_getElem hlt] at hc
      simp only [Option.mem_def, Option.some.injEq] at hc
      subst hc
      rwa [getElem!_pos l e hlt] at hend

theorem scan_of_json (l : List Std.U8) (st : Nat) (h1 : SkipOk l 0 st) (hj : JsonType l) :
    st + 16 ≤ l.length ∧ (∀ k < 16, lowerN l[st + k]!.val = jsonBytes[k]!.val) ∧
      ∀ e, SkipOk l (st + 16) e → (e = l.length ∨ l[e]!.val = 59) := by
  obtain ⟨a, m, b, r, rfl, ha, hm, hb, hr⟩ := hj
  obtain ⟨-, hst, hsp1, hns1⟩ := h1
  rw [eqLower_iff] at hm
  obtain ⟨hml, hmk⟩ := hm
  simp only [jsonBytes, List.length_cons, List.length_nil] at hml hmk
  simp only [List.append_assoc] at *
  simp only [List.length_append] at *
  -- The scan starts at the end of `a`.
  have hsta : st = a.length := by
    rcases Nat.lt_trichotomy st a.length with h | h | h
    · exfalso
      apply hns1 (by omega)
      rw [gb_lt h]
      exact ha _ (gb_mem h)
    · exact h
    · exfalso
      have hsp := hsp1 a.length (by omega) h
      rw [gb_ge (le_refl _), Nat.sub_self, gb_lt (by omega)] at hsp
      have := hmk 0 (by omega)
      have := lowerN_sp hsp
      simp_all
  subst hsta
  have hL : (a ++ (m ++ (b ++ r))).length = a.length + 16 + b.length + r.length := by
    simp only [List.length_append]; omega
  have hidx : ∀ k < 16, (a ++ (m ++ (b ++ r)))[a.length + k]! = m[k]! := by
    intro k hk
    rw [gb_ge (by omega), gb_lt (by omega)]
    congr 1; omega
  refine ⟨by omega, fun k hk => by rw [hidx k hk]; exact hmk k hk, ?_⟩
  -- The second scan stops at the end of `b`.
  intro e ⟨he1, he2, hsp2, hns2⟩
  have hp : (a ++ (m ++ (b ++ r)))[a.length + 16 + b.length]! = r[0]! := by
    rw [gb_ge (by omega), gb_ge (by omega), gb_ge (by omega)]
    congr 1; omega
  have heq : e = a.length + 16 + b.length := by
    rcases Nat.lt_trichotomy e (a.length + 16 + b.length) with h | h | h
    · exfalso
      apply hns2 (by omega)
      rw [gb_ge (by omega), gb_ge (by omega), gb_lt (by omega)]
      exact hb _ (gb_mem (by omega))
    · exact h
    · exfalso
      have hsp := hsp2 (a.length + 16 + b.length) (by omega) h
      rw [hp] at hsp
      have hrl : 0 < r.length := by omega
      have := hr r[0]! (by rw [getElem!_pos r 0 hrl]; simp [List.head?_eq_getElem?, hrl])
      unfold isSp at hsp
      omega
  subst heq
  rcases Nat.lt_or_ge (a.length + 16 + b.length) (a.length + (m.length + (b.length + r.length)))
    with h | h
  · right
    rw [hp]
    have hrl : 0 < r.length := by omega
    exact hr r[0]! (by rw [getElem!_pos r 0 hrl]; simp [List.head?_eq_getElem?, hrl])
  · left; omega

@[step]
theorem is_json_spec (ct : Slice Std.U8) :
    csrf.is_json ct ⦃ b => (b ↔ JsonType ct.val) ⦄ := by
  unfold csrf.is_json
  step*
  all_goals
    have hs : s.val = jsonBytes := by rw [s_post]; simp [jsonBytes]
    have hsl : s.length = 16 := by simp [Slice.length, hs, jsonBytes]
    rw [hs, hsl] at b_post
  · -- The scan reached the end.
    simp only [true_iff]
    obtain ⟨hm, hmatch⟩ := b_post.mp (by assumption)
    exact json_of_scan ct.val start.val «end».val start_post hm hmatch
      (by rw [← i_post]; exact end_post) (Or.inl (by scalar_tac))
  · -- The scan stopped at a byte.
    have hlt : «end».val < ct.val.length := by
      have := end_post.2.1; scalar_tac
    constructor
    · intro h
      obtain ⟨hm, hmatch⟩ := b_post.mp (by assumption)
      refine json_of_scan ct.val start.val «end».val start_post hm hmatch
        (by rw [← i_post]; exact end_post) (Or.inr ?_)
      rw [getElem!_pos ct.val «end».val hlt, ← i2_post]
      simp at h; simp [h]
    · intro hj
      obtain ⟨-, -, hend⟩ := scan_of_json ct.val start.val start_post hj
      rcases hend «end».val (by rw [← i_post]; exact end_post) with h | h
      · exfalso; scalar_tac
      · rw [getElem!_pos ct.val «end».val hlt, ← i2_post] at h
        simp only [decide_eq_true_eq]
        exact UScalar.eq_of_val_eq (by simpa using h)
  · -- `application/json` is not there.
    simp only [Bool.false_eq_true, false_iff]
    intro hj
    obtain ⟨hm, hmatch, -⟩ := scan_of_json ct.val start.val start_post hj
    exact (by assumption : ¬b = true) (b_post.mpr ⟨hm, hmatch⟩)

@[step]
theorem is_multipart_form_spec (ct : Slice Std.U8) :
    csrf.is_multipart_form ct ⦃ b => (b ↔ MultipartForm ct.val) ⦄ := by
  unfold csrf.is_multipart_form
  step*
  have hs : s.val = multipartBytes := by rw [s_post]; simp [multipartBytes]
  have hsl : s.length = 19 := by simp [Slice.length, hs, multipartBytes]
  rw [hs, hsl] at b_post
  rw [b_post, MultipartForm, eqLower_iff]
  simp only [multipartBytes, List.length_cons, List.length_nil, List.length_take]
  constructor
  · rintro ⟨h1, h2⟩
    refine ⟨by scalar_tac, fun k hk => ?_⟩
    rw [getElem!_pos _ _ (by simp; scalar_tac), List.getElem_take, ← getElem!_pos _ _ (by scalar_tac)]
    simpa using h2 k hk
  · rintro ⟨h1, h2⟩
    refine ⟨by scalar_tac, fun k hk => ?_⟩
    have := h2 k hk
    rw [getElem!_pos _ _ (by simp; scalar_tac), List.getElem_take,
      ← getElem!_pos _ _ (by scalar_tac)] at this
    simpa using this

end Logbook.Csrf

namespace Logbook
open Csrf

/-- The CSRF rule of T13, as a statement. -/
def CsrfRule (isRead originMatches hasBody : Bool) (ct : List Std.U8) (isUpload : Bool) :
    Prop :=
  isRead = true ∨
    (originMatches = true ∧
      (hasBody = false ∨ JsonType ct ∨ (isUpload = true ∧ MultipartForm ct)))

/-- T13 + T12: `write_allowed` allows a request if and only if `CsrfRule` holds. -/
theorem write_allowed_spec (isRead originMatches hasBody : Bool) (ct : Slice Std.U8)
    (isUpload : Bool) :
    csrf.write_allowed isRead originMatches hasBody ct isUpload ⦃ ok =>
      (ok ↔ CsrfRule isRead originMatches hasBody ct.val isUpload) ⦄ := by
  unfold csrf.write_allowed CsrfRule
  cases isRead <;> cases originMatches <;> cases hasBody <;> cases isUpload <;> step*

/-- T13: a write with an `Origin` that is not the site is always refused. -/
theorem csrf_cross_origin (hasBody : Bool) (ct : Slice Std.U8) (isUpload : Bool) :
    csrf.write_allowed false false hasBody ct isUpload ⦃ ok => ok = false ⦄ := by
  apply WP.spec_mono (write_allowed_spec false false hasBody ct isUpload)
  intro ok h
  cases ok <;> simp_all [CsrfRule]

/-- T13: a write with a body is allowed only as `application/json`, or as
`multipart/form-data` on the upload path. -/
theorem csrf_body (originMatches : Bool) (ct : Slice Std.U8) (isUpload : Bool) :
    csrf.write_allowed false originMatches true ct isUpload ⦃ ok =>
      (ok = true → JsonType ct.val ∨ (isUpload = true ∧ MultipartForm ct.val)) ⦄ := by
  apply WP.spec_mono (write_allowed_spec false originMatches true ct isUpload)
  intro ok h hok
  simp_all [CsrfRule]

/-- T13: a read (`GET`, `HEAD`) is always allowed. -/
theorem csrf_read (originMatches hasBody : Bool) (ct : Slice Std.U8) (isUpload : Bool) :
    csrf.write_allowed true originMatches hasBody ct isUpload ⦃ ok => ok = true ⦄ := by
  unfold csrf.write_allowed
  simp

end Logbook
