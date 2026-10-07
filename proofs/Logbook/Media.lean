import LogbookCore.Funs
open Aeneas Aeneas.Std Result logbook_core

/-! Media keys (spec 7.6): T10, with T12 (no panic) built in. -/

namespace Logbook

/-- `0-9` or `a-f`. -/
def isHex (c : Std.U8) : Prop :=
  (48 ≤ c.val ∧ c.val ≤ 57) ∨ (97 ≤ c.val ∧ c.val ≤ 102)

@[step]
theorem is_lower_hex_spec (c : Std.U8) :
    media.is_lower_hex c ⦃ b => (b ↔ isHex c) ⦄ := by
  unfold media.is_lower_hex isHex
  split <;> (try split) <;> (try split) <;> simp <;> scalar_tac

/-- `key[at..]` is exactly `ext`, as a statement about indices. -/
def tailIs (key : List Std.U8) (at' : Nat) (ext : List Std.U8) : Prop :=
  key.length = at' + ext.length ∧ ∀ j < ext.length, key[at' + j]! = ext[j]!

theorem tail_is_loop_spec (key ext : Slice Std.U8) (at' i : Std.Usize)
    (hlen : key.length = at'.val + ext.length) (hi : i.val ≤ ext.length)
    (hpre : ∀ j < i.val, key.val[at'.val + j]! = ext.val[j]!) :
    media.tail_is_loop key at' ext i ⦃ b => (b ↔ ∀ j < ext.length, key.val[at'.val + j]! = ext.val[j]!) ⦄ := by
  unfold media.tail_is_loop
  apply loop.spec_decr_nat
    (measure := fun (i : Std.Usize) => ext.length - i.val)
    (inv := fun i => i.val ≤ ext.length ∧ ∀ j < i.val, key.val[at'.val + j]! = ext.val[j]!)
  · rintro i ⟨hi, hpre⟩
    unfold media.tail_is_loop.body
    simp only
    split
    · step*
      · rename_i hne
        simp only [Bool.false_eq_true, false_iff, not_forall]
        refine ⟨i.val, by scalar_tac, ?_⟩
        simp only [bne_iff_ne, ne_eq] at hne
        rw [getElem!_pos _ _ (by scalar_tac), getElem!_pos _ _ (by scalar_tac)]
        simp_all
      · rename_i heq
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        intro j hj
        by_cases hji : j < i.val
        · exact hpre j hji
        · have : j = i.val := by scalar_tac
          subst this
          rw [getElem!_pos _ _ (by scalar_tac), getElem!_pos _ _ (by scalar_tac)]
          simp_all
          exact UScalar.eq_of_val_eq heq
    · simp only [WP.spec_ok]
      have : ext.length ≤ i.val := by scalar_tac
      constructor
      · intro _ j hj; exact hpre j (by omega)
      · intro _; trivial
  · exact ⟨hi, hpre⟩

@[step]
theorem tail_is_spec (key ext : Slice Std.U8) (at' : Std.Usize)
    (hsum : at'.val + ext.length ≤ Usize.max) :
    media.tail_is key at' ext ⦃ b => (b ↔ tailIs key.val at'.val ext.val) ⦄ := by
  unfold media.tail_is tailIs
  step*
  all_goals first
    | (simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_and]
       intro h
       simp_all [bne_iff_ne]
       scalar_tac)
    | (rename_i heq
       simp only [bne_iff_ne, ne_eq, Decidable.not_not] at heq
       apply WP.spec_mono (tail_is_loop_spec key ext at' 0#usize (by scalar_tac) (by simp) (by simp))
       intro b hb
       constructor
       · intro h; exact ⟨by scalar_tac, hb.mp h⟩
       · intro ⟨_, h⟩; exact hb.mpr h)

@[simp]
theorem make_slice_val (n : Std.Usize) (l : List Std.U8) (h : l.length = n.val) :
    (Array.make n l h).to_slice.val = l := by
  simp [Array.to_slice]

/-- The extensions, as bytes. -/
def png : List Std.U8 := [112#u8, 110#u8, 103#u8]
def jpg : List Std.U8 := [106#u8, 112#u8, 103#u8]
def gif : List Std.U8 := [103#u8, 105#u8, 102#u8]
def webp : List Std.U8 := [119#u8, 101#u8, 98#u8, 112#u8]
def mp4 : List Std.U8 := [109#u8, 112#u8, 52#u8]
def webm : List Std.U8 := [119#u8, 101#u8, 98#u8, 109#u8]

example : mp4.map (·.val) = "mp4".toList.map Char.toNat := by decide
example : webm.map (·.val) = "webm".toList.map Char.toNat := by decide

/-- 64 lowercase hex digits, then a `.`. -/
def hexDot (k : List Std.U8) : Prop :=
  65 ≤ k.length ∧ (∀ j < 64, isHex k[j]!) ∧ k[64]! = 46#u8

/-- The only keys that `/media/{key}` accepts: 64 lowercase hex digits, `.`, and
`png`, `jpg`, `gif`, `webp`, `mp4`, or `webm`. Nothing else, so no key can leave
its folder. -/
def keyShape (k : List Std.U8) : Prop :=
  hexDot k ∧ (tailIs k 65 png ∨ tailIs k 65 jpg ∨ tailIs k 65 gif ∨ tailIs k 65 webp ∨
    tailIs k 65 mp4 ∨ tailIs k 65 webm)

/-- The keys of uploaded videos: 64 lowercase hex digits, `.`, and `mp4` or `webm`. -/
def videoKeyShape (k : List Std.U8) : Prop :=
  hexDot k ∧ (tailIs k 65 mp4 ∨ tailIs k 65 webm)

theorem hex_dot_loop_spec (key : Slice Std.U8) (i : Std.Usize)
    (hlen : 65 ≤ key.length) (hi : i.val ≤ 64) (hpre : ∀ j < i.val, isHex key.val[j]!) :
    media.hex_dot_loop key i ⦃ b => (b ↔ hexDot key.val) ⦄ := by
  unfold media.hex_dot_loop
  apply loop.spec_decr_nat
    (measure := fun (i : Std.Usize) => 64 - i.val)
    (inv := fun i => i.val ≤ 64 ∧ ∀ j < i.val, isHex key.val[j]!)
  · rintro i ⟨hi, hpre⟩
    unfold media.hex_dot_loop.body
    step*
    -- The loop step: one more hex digit.
    · refine ⟨by scalar_tac, ?_, by scalar_tac⟩
      intro j hj
      by_cases hji : j < i.val
      · exact hpre j hji
      · have : j = i.val := by scalar_tac
        subst this
        rw [getElem!_pos _ _ (by scalar_tac)]
        simp_all
    -- Not a hex digit: no key.
    · simp only [Bool.false_eq_true, false_iff]
      intro ⟨_, hhex, _⟩
      have := hhex i.val (by scalar_tac)
      rw [getElem!_pos _ _ (by scalar_tac)] at this
      simp_all
    -- All 64 digits: the `.` decides it.
    · have h64 : i.val = 64 := by scalar_tac
      simp only [hexDot, decide_eq_true_eq]
      have hdot : key.val[64]! = i1 := by rw [getElem!_pos _ _ (by scalar_tac)]; simp_all
      constructor
      · intro h; exact ⟨hlen, fun j hj => hpre j (by omega), by rw [hdot]; exact h⟩
      · rintro ⟨_, _, h⟩; rw [← hdot]; exact h
  · exact ⟨hi, hpre⟩

@[step]
theorem hex_dot_spec (key : Slice Std.U8) :
    media.hex_dot key ⦃ b => (b ↔ hexDot key.val) ⦄ := by
  unfold media.hex_dot
  dsimp only
  split
  · simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
    intro ⟨h, _⟩
    simp_all
    scalar_tac
  · apply hex_dot_loop_spec <;> simp_all

@[step]
theorem video_ext_spec (key : Slice Std.U8) :
    media.video_ext key ⦃ b => (b ↔ tailIs key.val 65 mp4 ∨ tailIs key.val 65 webm) ⦄ := by
  unfold media.video_ext
  step*
  · simp_all [mp4]
  · simp_all [mp4, webm]

/-- T10 + T12: `media_key_ok` accepts exactly the keys of `keyShape`, and never fails. -/
theorem media_key_spec (key : Slice Std.U8) :
    media.media_key_ok key ⦃ b => (b ↔ keyShape key.val) ⦄ := by
  unfold media.media_key_ok keyShape
  step*
  all_goals simp_all [png, jpg, gif, webp]

/-- T18 (files) + T12: `video_key_ok` accepts exactly the keys of `videoKeyShape`,
and never fails. -/
theorem video_key_spec (key : Slice Std.U8) :
    media.video_key_ok key ⦃ b => (b ↔ videoKeyShape key.val) ⦄ := by
  unfold media.video_key_ok videoKeyShape
  step*

/-- Every video key is a media key, so `/media/{key}` serves it. -/
theorem video_key_is_media_key (k : List Std.U8) (h : videoKeyShape k) : keyShape k := by
  obtain ⟨hd, h⟩ := h
  exact ⟨hd, by tauto⟩

end Logbook
