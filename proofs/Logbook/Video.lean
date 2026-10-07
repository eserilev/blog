import LogbookCore.Funs
import Logbook.Media
open Aeneas Aeneas.Std Result logbook_core

/-! YouTube addresses in a video block (spec 7.6): T18, with T12 (no panic) built in.
`youtube_id` finds the video ID in exactly the accepted addresses. The ID is 11
bytes of `A-Z a-z 0-9 _ -`, so it holds no byte that can leave an HTML attribute
or change the path of the embed address. -/

namespace Logbook.Video

/-- `A-Z`, `a-z`, `0-9`, `_`, or `-`. -/
def IdByte (c : Std.U8) : Prop :=
  (65 ≤ c.val ∧ c.val ≤ 90) ∨ (97 ≤ c.val ∧ c.val ≤ 122) ∨ (48 ≤ c.val ∧ c.val ≤ 57) ∨
    c.val = 95 ∨ c.val = 45

/-- `https://www.youtube.com/watch?v=` -/
def pWatchWww : List Std.U8 := [104#u8, 116#u8, 116#u8, 112#u8, 115#u8, 58#u8, 47#u8, 47#u8, 119#u8, 119#u8, 119#u8, 46#u8, 121#u8, 111#u8, 117#u8, 116#u8, 117#u8, 98#u8, 101#u8, 46#u8, 99#u8, 111#u8, 109#u8, 47#u8, 119#u8, 97#u8, 116#u8, 99#u8, 104#u8, 63#u8, 118#u8, 61#u8]
/-- `https://youtube.com/watch?v=` -/
def pWatch : List Std.U8 := [104#u8, 116#u8, 116#u8, 112#u8, 115#u8, 58#u8, 47#u8, 47#u8, 121#u8, 111#u8, 117#u8, 116#u8, 117#u8, 98#u8, 101#u8, 46#u8, 99#u8, 111#u8, 109#u8, 47#u8, 119#u8, 97#u8, 116#u8, 99#u8, 104#u8, 63#u8, 118#u8, 61#u8]
/-- `https://m.youtube.com/watch?v=` -/
def pWatchM : List Std.U8 := [104#u8, 116#u8, 116#u8, 112#u8, 115#u8, 58#u8, 47#u8, 47#u8, 109#u8, 46#u8, 121#u8, 111#u8, 117#u8, 116#u8, 117#u8, 98#u8, 101#u8, 46#u8, 99#u8, 111#u8, 109#u8, 47#u8, 119#u8, 97#u8, 116#u8, 99#u8, 104#u8, 63#u8, 118#u8, 61#u8]
/-- `https://youtu.be/` -/
def pBe : List Std.U8 := [104#u8, 116#u8, 116#u8, 112#u8, 115#u8, 58#u8, 47#u8, 47#u8, 121#u8, 111#u8, 117#u8, 116#u8, 117#u8, 46#u8, 98#u8, 101#u8, 47#u8]
/-- `https://www.youtube.com/shorts/` -/
def pShortsWww : List Std.U8 := [104#u8, 116#u8, 116#u8, 112#u8, 115#u8, 58#u8, 47#u8, 47#u8, 119#u8, 119#u8, 119#u8, 46#u8, 121#u8, 111#u8, 117#u8, 116#u8, 117#u8, 98#u8, 101#u8, 46#u8, 99#u8, 111#u8, 109#u8, 47#u8, 115#u8, 104#u8, 111#u8, 114#u8, 116#u8, 115#u8, 47#u8]
/-- `https://youtube.com/shorts/` -/
def pShorts : List Std.U8 := [104#u8, 116#u8, 116#u8, 112#u8, 115#u8, 58#u8, 47#u8, 47#u8, 121#u8, 111#u8, 117#u8, 116#u8, 117#u8, 98#u8, 101#u8, 46#u8, 99#u8, 111#u8, 109#u8, 47#u8, 115#u8, 104#u8, 111#u8, 114#u8, 116#u8, 115#u8, 47#u8]

example : pWatchWww.map (·.val) = "https://www.youtube.com/watch?v=".toList.map Char.toNat := by decide
example : pWatch.map (·.val) = "https://youtube.com/watch?v=".toList.map Char.toNat := by decide
example : pWatchM.map (·.val) = "https://m.youtube.com/watch?v=".toList.map Char.toNat := by decide
example : pBe.map (·.val) = "https://youtu.be/".toList.map Char.toNat := by decide
example : pShortsWww.map (·.val) = "https://www.youtube.com/shorts/".toList.map Char.toNat := by decide
example : pShorts.map (·.val) = "https://youtube.com/shorts/".toList.map Char.toNat := by decide

/-- The accepted address parts before the ID. -/
def prefixes : List (List Std.U8) := [pWatchWww, pWatch, pWatchM, pBe, pShortsWww, pShorts]

/-- 11 ID bytes at `i`, then the end of the address, or `?`, `&`, or `#`. -/
def IdAt (u : List Std.U8) (i : Nat) : Prop :=
  i + 11 ≤ u.length ∧ (∀ j < 11, IdByte u[i + j]!) ∧
    (i + 11 = u.length ∨ u[i + 11]!.val = 63 ∨ u[i + 11]!.val = 38 ∨ u[i + 11]!.val = 35)

/-- `u` is an accepted YouTube address, and its ID starts at `i`. -/
def YouTube (u : List Std.U8) (i : Nat) : Prop :=
  ∃ p ∈ prefixes, p <+: u ∧ i = p.length ∧ IdAt u i

/-! Lemmas about lists. -/

theorem prefix_iff_get (p s : List Std.U8) :
    p <+: s ↔ p.length ≤ s.length ∧ ∀ k < p.length, s[k]! = p[k]! := by
  rw [List.prefix_iff_eq_take]
  constructor
  · intro h
    have hl : p.length ≤ s.length := by
      have := congrArg List.length h
      simp at this
      omega
    refine ⟨hl, fun k hk => ?_⟩
    have := congrArg (fun l => l[k]?) h
    simp only [List.getElem?_take, hk, if_true] at this
    rw [getElem!_pos s k (by omega), getElem!_pos p k hk]
    rw [List.getElem?_eq_getElem hk, List.getElem?_eq_getElem (by omega)] at this
    simp at this
    exact this.symm
  · rintro ⟨hl, h⟩
    apply List.ext_getElem (by simp; omega)
    intro k h1 h2
    have := h k h1
    rw [getElem!_pos s k (by omega), getElem!_pos p k h1] at this
    simp [this]

/-- No accepted part starts another one. -/
theorem prefixes_disjoint : ∀ p ∈ prefixes, ∀ q ∈ prefixes, p <+: q → p = q := by
  decide +kernel

theorem prefix_unique {u p q : List Std.U8} (hp : p ∈ prefixes) (hq : q ∈ prefixes)
    (h1 : p <+: u) (h2 : q <+: u) : p = q := by
  rcases le_total p.length q.length with h | h
  · exact prefixes_disjoint p hp q hq (List.prefix_of_prefix_length_le h1 h2 h)
  · exact (prefixes_disjoint q hq p hp (List.prefix_of_prefix_length_le h2 h1 h)).symm

/-! The Rust functions. -/

@[step]
theorem is_id_byte_spec (c : Std.U8) : video.is_id_byte c ⦃ b => (b ↔ IdByte c) ⦄ := by
  unfold video.is_id_byte IdByte
  split_ifs <;> simp_all <;> scalar_tac

theorem starts_loop_spec (s p : Slice Std.U8) (j : Std.Usize)
    (hlen : p.length ≤ s.length) (hj : j.val ≤ p.length)
    (hpre : ∀ k < j.val, s.val[k]! = p.val[k]!) :
    video.starts_loop s p j ⦃ b => (b ↔ ∀ k < p.length, s.val[k]! = p.val[k]!) ⦄ := by
  unfold video.starts_loop
  apply loop.spec_decr_nat
    (measure := fun (j : Std.Usize) => p.length - j.val)
    (inv := fun j => j.val ≤ p.length ∧ ∀ k < j.val, s.val[k]! = p.val[k]!)
  · rintro j ⟨hj, hpre⟩
    unfold video.starts_loop.body
    step*
    · rename_i hne
      simp only [Bool.false_eq_true, false_iff, not_forall]
      refine ⟨j.val, by scalar_tac, ?_⟩
      simp only [bne_iff_ne, ne_eq] at hne
      rw [getElem!_pos _ _ (by scalar_tac), getElem!_pos _ _ (by scalar_tac)]
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
theorem starts_spec (s p : Slice Std.U8) :
    video.starts s p ⦃ b => (b ↔ p.val <+: s.val) ⦄ := by
  unfold video.starts
  step*
  · simp only [Bool.false_eq_true, false_iff]
    intro h
    have := ((prefix_iff_get _ _).mp h).1
    scalar_tac
  · apply WP.spec_mono (starts_loop_spec s p 0#usize (by scalar_tac) (by scalar_tac)
      (by intro k hk; simp at hk))
    intro b hb
    rw [hb, prefix_iff_get]
    constructor
    · intro h; exact ⟨by scalar_tac, h⟩
    · intro h; exact h.2

@[step]
theorem youtube_prefix_spec (url : Slice Std.U8) :
    video.youtube_prefix url ⦃ n =>
      (n.val = 0 ∧ ∀ p ∈ prefixes, ¬ p <+: url.val) ∨
      (∃ p ∈ prefixes, p <+: url.val ∧ n.val = p.length) ⦄ := by
  unfold video.youtube_prefix
  step*
  all_goals simp_all [prefixes, pWatchWww, pWatch, pWatchM, pBe, pShortsWww, pShorts]

theorem id_at_loop_spec (url : Slice Std.U8) (at' j : Std.Usize)
    (hlen : at'.val + 11 ≤ url.length) (hj : j.val ≤ 11)
    (hpre : ∀ k < j.val, IdByte url.val[at'.val + k]!) :
    video.id_at_loop url at' j ⦃ b => (b ↔ IdAt url.val at'.val) ⦄ := by
  unfold video.id_at_loop
  apply loop.spec_decr_nat
    (measure := fun (j : Std.Usize) => 11 - j.val)
    (inv := fun j => j.val ≤ 11 ∧ ∀ k < j.val, IdByte url.val[at'.val + k]!)
  · rintro j ⟨hj, hpre⟩
    unfold video.id_at_loop.body video.YOUTUBE_ID_LEN
    step*
    -- The loop step: one more ID byte.
    · refine ⟨by scalar_tac, ?_, by scalar_tac⟩
      intro k hk
      by_cases hkj : k < j.val
      · exact hpre k hkj
      · have : k = j.val := by scalar_tac
        subst this
        rw [getElem!_pos _ _ (by scalar_tac)]
        simp_all
    -- Not an ID byte: no ID.
    · simp only [Bool.false_eq_true, false_iff]
      intro ⟨_, hid, _⟩
      have := hid j.val (by scalar_tac)
      rw [getElem!_pos _ _ (by scalar_tac)] at this
      simp_all
    -- 11 ID bytes, then the end.
    · simp only [true_iff]
      exact ⟨by scalar_tac, fun k hk => hpre k (by scalar_tac), Or.inl (by scalar_tac)⟩
    -- 11 ID bytes, then `?`.
    · simp only [true_iff]
      refine ⟨by scalar_tac, fun k hk => hpre k (by scalar_tac), Or.inr (Or.inl ?_)⟩
      have hat : url.val[at'.val + 11]! = i1 := by
        rw [getElem!_pos _ _ (by scalar_tac)]; simp_all
      rw [hat, ‹i1 = 63#u8›]; rfl
    -- 11 ID bytes, then `&`.
    · simp only [true_iff]
      refine ⟨by scalar_tac, fun k hk => hpre k (by scalar_tac), Or.inr (Or.inr (Or.inl ?_))⟩
      have hat : url.val[at'.val + 11]! = i1 := by
        rw [getElem!_pos _ _ (by scalar_tac)]; simp_all
      rw [hat, ‹i1 = 38#u8›]; rfl
    -- 11 ID bytes, then `#` or another byte.
    · simp only [decide_eq_true_eq]
      have hat : url.val[at'.val + 11]! = i1 := by
        rw [getElem!_pos _ _ (by scalar_tac)]; simp_all
      constructor
      · intro h
        refine ⟨by scalar_tac, fun k hk => hpre k (by scalar_tac), Or.inr (Or.inr (Or.inr ?_))⟩
        rw [hat, h]; rfl
      · rintro ⟨_, _, h | h | h | h⟩
        · scalar_tac
        · rw [hat] at h; exact absurd (UScalar.eq_of_val_eq (by rw [h]; rfl)) ‹¬i1 = 63#u8›
        · rw [hat] at h; exact absurd (UScalar.eq_of_val_eq (by rw [h]; rfl)) ‹¬i1 = 38#u8›
        · rw [hat] at h; exact UScalar.eq_of_val_eq (by rw [h]; rfl)
  · exact ⟨hj, hpre⟩

@[step]
theorem id_at_spec (url : Slice Std.U8) (at' : Std.Usize) :
    video.id_at url at' ⦃ b => (b ↔ IdAt url.val at'.val) ⦄ := by
  unfold video.id_at video.YOUTUBE_ID_LEN
  step*
  · simp only [Bool.false_eq_true, false_iff]
    intro ⟨h, _⟩
    scalar_tac
  · simp only [Bool.false_eq_true, false_iff]
    intro ⟨h, _⟩
    scalar_tac
  · exact id_at_loop_spec url at' 0#usize (by scalar_tac) (by simp) (by intro k hk; simp at hk)

theorem prefixes_nonempty : ∀ p ∈ prefixes, 0 < p.length := by decide

/-- T18 + T12: `youtube_id` gives the start of the ID for exactly the accepted
addresses, and never fails. -/
theorem youtube_id_spec (url : Slice Std.U8) :
    video.youtube_id url ⦃ r =>
      (∀ i, r = some i → YouTube url.val i.val) ∧ (r = none → ∀ i, ¬ YouTube url.val i) ⦄ := by
  unfold video.youtube_id
  step*
  -- An accepted part and an ID: the result is the start of the ID.
  · refine ⟨fun i hi => ?_, fun h => by simp at h⟩
    simp only [Option.some.injEq] at hi
    subst hi
    rcases at_post with ⟨h0, _⟩ | ⟨p, hp, hpre, hlen⟩
    · scalar_tac
    · exact ⟨p, hp, hpre, hlen, b_post.mp ‹_›⟩
  -- An accepted part and no ID: no other part matches, so no address.
  · refine ⟨fun i hi => by simp at hi, fun _ i ⟨q, hq, hqpre, hi, hid⟩ => ?_⟩
    rcases at_post with ⟨h0, _⟩ | ⟨p, hp, hpre, hlen⟩
    · scalar_tac
    · have := prefix_unique hp hq hpre hqpre
      subst this
      apply ‹¬b = true›
      rw [b_post, hlen, ← hi]
      exact hid
  -- No accepted part: no address.
  · refine ⟨fun i hi => by simp at hi, fun _ i ⟨q, hq, hqpre, _, _⟩ => ?_⟩
    rcases at_post with ⟨_, hno⟩ | ⟨p, hp, _, hlen⟩
    · exact hno q hq hqpre
    · have := prefixes_nonempty p hp
      scalar_tac

/-- The ID bytes of an accepted address: 11 bytes, each `A-Z a-z 0-9 _ -`. -/
theorem youtube_id_plain (url : Slice Std.U8) (i : Std.Usize)
    (h : video.youtube_id url = ok (some i)) :
    i.val + 11 ≤ url.length ∧ ∀ j < 11, IdByte url.val[i.val + j]! := by
  have := youtube_id_spec url
  rw [h] at this
  simp only [WP.spec_ok] at this
  obtain ⟨_, _, _, _, hid⟩ := this.1 i rfl
  exact ⟨hid.1, hid.2.1⟩

end Logbook.Video
