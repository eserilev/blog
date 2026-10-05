import LogbookCore.Funs
open Aeneas Aeneas.Std Result logbook_core

/-! HTML escaping (spec 7.6): T14, with T12 (no panic) built in. -/

namespace Logbook

/-- The five entities, as bytes: `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&#39;`. -/
def entAmp : List Std.U8 := [38#u8, 97#u8, 109#u8, 112#u8, 59#u8]
def entLt : List Std.U8 := [38#u8, 108#u8, 116#u8, 59#u8]
def entGt : List Std.U8 := [38#u8, 103#u8, 116#u8, 59#u8]
def entQuot : List Std.U8 := [38#u8, 113#u8, 117#u8, 111#u8, 116#u8, 59#u8]
def entApos : List Std.U8 := [38#u8, 35#u8, 51#u8, 57#u8, 59#u8]

def entities : List (List Std.U8) := [entAmp, entLt, entGt, entQuot, entApos]

/-- The escape of one byte. `&` is 38, `<` is 60, `>` is 62, `"` is 34, `'` is 39. -/
def escByte (c : Std.U8) : List Std.U8 :=
  if c = 38#u8 then entAmp
  else if c = 60#u8 then entLt
  else if c = 62#u8 then entGt
  else if c = 34#u8 then entQuot
  else if c = 39#u8 then entApos
  else [c]

/-- The escape of a byte string: each byte escaped, in order. -/
def escape (s : List Std.U8) : List Std.U8 := s.flatMap escByte

/-- The inverse: each entity becomes its byte again. Other bytes stay the same. -/
def unescape : List Std.U8 → List Std.U8
  | [] => []
  | c :: r =>
    if c = 38#u8 ∧ r.take 4 = entAmp.tail then 38#u8 :: unescape (r.drop 4)
    else if c = 38#u8 ∧ r.take 3 = entLt.tail then 60#u8 :: unescape (r.drop 3)
    else if c = 38#u8 ∧ r.take 3 = entGt.tail then 62#u8 :: unescape (r.drop 3)
    else if c = 38#u8 ∧ r.take 5 = entQuot.tail then 34#u8 :: unescape (r.drop 5)
    else if c = 38#u8 ∧ r.take 4 = entApos.tail then 39#u8 :: unescape (r.drop 4)
    else c :: unescape r
termination_by l => l.length

theorem escByte_length (c : Std.U8) : (escByte c).length ≤ 6 := by
  unfold escByte; split_ifs <;> simp [entAmp, entLt, entGt, entQuot, entApos]

theorem escape_length (s : List Std.U8) : (escape s).length ≤ 6 * s.length := by
  induction s with
  | nil => simp [escape]
  | cons c s ih =>
    simp only [escape, List.flatMap_cons, List.length_append, List.length_cons] at *
    have := escByte_length c
    omega

theorem escape_html_loop_spec (s : Slice Std.U8) (out : alloc.vec.Vec Std.U8) (i : Std.Usize)
    (hmax : 6 * s.length ≤ Usize.max) (hi : i.val ≤ s.length)
    (hout : out.val = escape (s.val.take i.val)) :
    html.escape_html_loop s out i ⦃ r => r.val = escape s.val ⦄ := by
  unfold html.escape_html_loop
  apply loop.spec_decr_nat
    (measure := fun (x : alloc.vec.Vec Std.U8 × Std.Usize) => s.length - x.2.val)
    (inv := fun x => x.2.val ≤ s.length ∧ x.1.val = escape (s.val.take x.2.val))
  · rintro ⟨out, i⟩ ⟨hi, hout⟩
    simp only at hi hout
    unfold html.escape_html_loop.body
    simp only
    split
    · have hlen : out.val.length ≤ 6 * i.val := by
        rw [hout]
        exact (escape_length _).trans (by simp)
      step*
      have htake : escape (s.val.take (i.val + 1)) = out.val ++ escByte c := by
        rw [List.take_succ_eq_append_getElem (show i.val < s.val.length by scalar_tac), c_post,
          hout, escape, escape, List.flatMap_append]
        simp
      have hroom : out.val.length + 6 ≤ Usize.max := by scalar_tac
      split_ifs <;> step*
      all_goals first
        | (simp_all only [List.length_append, List.length_singleton]; scalar_tac)
        | (refine ⟨by scalar_tac, ?_, by scalar_tac⟩
           simp_all [escByte, entAmp, entLt, entGt, entQuot, entApos])
    · simp only [WP.spec_ok]
      have : i.val = s.length := by scalar_tac
      simp [hout, this]
  · exact ⟨hi, hout⟩

/-- T14 + T12: `escape_html` returns `escape s`. It never fails for an input of at
most `Usize.max / 6` bytes. A longer input does not fit in memory as output. -/
theorem escape_html_spec (s : Slice Std.U8) (hmax : 6 * s.length ≤ Usize.max) :
    html.escape_html s ⦃ r => r.val = escape s.val ⦄ := by
  unfold html.escape_html
  apply escape_html_loop_spec <;> simp_all [escape]

/-- Each byte escapes to itself (if it is not one of the five) or to an entity. -/
theorem escByte_cases (c : Std.U8) :
    (escByte c = [c] ∧ c ≠ 38#u8 ∧ c ≠ 60#u8 ∧ c ≠ 62#u8 ∧ c ≠ 34#u8 ∧ c ≠ 39#u8) ∨
      escByte c ∈ entities := by
  unfold escByte
  split_ifs <;> simp_all [entities]

/-- An entity starts with `&`, has no other `&`, and has no `<`, `>`, `"`, or `'`. -/
theorem entity_bytes (e : List Std.U8) (he : e ∈ entities) :
    e.head? = some 38#u8 ∧ 38#u8 ∉ e.tail ∧
      ∀ x ∈ e, x ≠ 60#u8 ∧ x ≠ 62#u8 ∧ x ≠ 34#u8 ∧ x ≠ 39#u8 := by
  simp only [entities, List.mem_cons, List.not_mem_nil, or_false] at he
  rcases he with rfl | rfl | rfl | rfl | rfl <;> decide

/-- T14: the output has no `<`, `>`, `"`, or `'`. -/
theorem escape_no_special (s : List Std.U8) :
    ∀ c ∈ escape s, c ≠ 60#u8 ∧ c ≠ 62#u8 ∧ c ≠ 34#u8 ∧ c ≠ 39#u8 := by
  intro c hc
  simp only [escape, List.mem_flatMap] at hc
  obtain ⟨x, _, hx⟩ := hc
  rcases escByte_cases x with ⟨h, h1, h2, h3, h4, h5⟩ | h
  · rw [h, List.mem_singleton] at hx
    subst hx
    exact ⟨h2, h3, h4, h5⟩
  · exact (entity_bytes _ h).2.2 c hx

/-- T14: every `&` in the output starts one of the five entities. -/
theorem escape_amp (s : List Std.U8) :
    ∀ i, (escape s)[i]? = some 38#u8 → ∃ e ∈ entities, e <+: (escape s).drop i := by
  induction s with
  | nil => simp [escape]
  | cons c s ih =>
    intro i hamp
    simp only [escape, List.flatMap_cons] at *
    by_cases hlt : i < (escByte c).length
    · rw [List.getElem?_append_left hlt] at hamp
      rcases escByte_cases c with ⟨h, h1, -⟩ | h
      · rw [h] at hamp hlt
        simp only [List.length_singleton, Nat.lt_one_iff] at hlt
        subst hlt
        simp_all
      · obtain ⟨_, htail, -⟩ := entity_bytes _ h
        cases i with
        | zero => exact ⟨escByte c, h, by simp⟩
        | succ j =>
          exfalso
          apply htail
          rw [← List.getElem?_cons_succ (a := (escByte c).head!)] at hamp
          cases he : escByte c with
          | nil => simp [he] at hlt
          | cons x t =>
            rw [he] at hamp
            simp only [List.tail_cons]
            simp only [List.head!_cons, List.getElem?_cons_succ] at hamp
            exact List.mem_of_getElem? hamp
    · have hge : (escByte c).length ≤ i := by omega
      rw [List.getElem?_append_right hge] at hamp
      obtain ⟨e, he, hp⟩ := ih _ hamp
      refine ⟨e, he, ?_⟩
      rw [List.drop_append, List.drop_eq_nil_of_le hge, List.nil_append]
      exact hp

theorem unescape_byte (c : Std.U8) (r : List Std.U8) (h : c ≠ 38#u8) :
    unescape (c :: r) = c :: unescape r := by
  rw [unescape]; simp [h]

theorem unescape_entity (c : Std.U8) (r : List Std.U8) (h : escByte c ∈ entities) :
    unescape (escByte c ++ r) = c :: unescape r := by
  unfold escByte at *
  split_ifs at * <;> simp_all [entities, entAmp, entLt, entGt, entQuot, entApos] <;>
    (rw [unescape]; simp [entAmp, entLt, entGt, entQuot, entApos])

/-- T14: un-escaping the output gives back the input. -/
theorem unescape_escape (s : List Std.U8) : unescape (escape s) = s := by
  induction s with
  | nil => simp [escape, unescape]
  | cons c s ih =>
    simp only [escape, List.flatMap_cons] at *
    rcases escByte_cases c with ⟨h, h1, -⟩ | h
    · rw [h, List.singleton_append, unescape_byte _ _ h1, ih]
    · rw [unescape_entity _ _ h, ih]

end Logbook
