import Logbook.Paint.Basic
import Mathlib.Logic.Relation
open Aeneas Aeneas.Std Result logbook_paint

/-! Fill With Color (spec 7.6): P1 and P2, for `flood_fill`. -/

namespace Logbook.Paint

/-! The region that a fill changes. Pixels are indices into `px`, row by row. -/

/-- Neighbor kind `d` in a grid `w` wide with `n` pixels: `j` is left of `i` (0),
right of `i` (1), above `i` (2), or below `i` (3). -/
def Nb (w n : Nat) : Nat → Nat → Nat → Prop
  | 0, i, j => 0 < i % w ∧ j + 1 = i
  | 1, i, j => i % w + 1 < w ∧ j = i + 1
  | 2, i, j => w ≤ i ∧ j + w = i
  | 3, i, j => j = i + w ∧ j < n
  | _, _, _ => False

/-- `j` is a neighbor of `i` of one of the kinds `0..d`. -/
def Near (w n d i j : Nat) : Prop := ∃ e < d, Nb w n e i j

/-- `j` is a 4-way neighbor of `i`. -/
def Adj (w n i j : Nat) : Prop := Near w n 4 i j

/-- One step of the fill: from `i` to a 4-way neighbor `j` that has color `t` in `o`. -/
def FillStep (w : Nat) (o : List Std.U32) (t : Std.U32) (i j : Nat) : Prop :=
  Adj w o.length i j ∧ j < o.length ∧ o[j]! = t

/-- P1: the region of `s`. These are the pixels that connect to `s` through 4-way
neighbors that have the color of `s`. -/
def Region (w : Nat) (o : List Std.U32) (s k : Nat) : Prop :=
  Relation.ReflTransGen (FillStep w o o[s]!) s k

/-- Facts about `w * a` and `w * b` that `omega` cannot find alone. -/
theorem mul_facts (w a b : Nat) :
    (a + 1 ≤ b → w * a + w ≤ w * b) ∧ (b + 1 ≤ a → w * b + w ≤ w * a) ∧
    (a + 2 ≤ b → w * a + 2 * w ≤ w * b) ∧ (b + 2 ≤ a → w * b + 2 * w ≤ w * a) ∧
    (a = b + 1 → w * a = w * b + w) ∧ (b = a + 1 → w * b = w * a + w) ∧
    (a = b → w * a = w * b) := by
  refine ⟨fun h => ?_, fun h => ?_, fun h => ?_, fun h => ?_, fun h => ?_, fun h => ?_,
    fun h => by rw [h]⟩
  · have := Nat.mul_le_mul_left w h; rw [Nat.mul_add, Nat.mul_one] at this; exact this
  · have := Nat.mul_le_mul_left w h; rw [Nat.mul_add, Nat.mul_one] at this; exact this
  · have := Nat.mul_le_mul_left w h; rw [Nat.mul_add] at this; omega
  · have := Nat.mul_le_mul_left w h; rw [Nat.mul_add] at this; omega
  · subst h; rw [Nat.mul_add, Nat.mul_one]
  · subst h; rw [Nat.mul_add, Nat.mul_one]

/-- `Adj` in coordinates: in a grid `w` wide, pixel `i` is at column `i % w` and row
`i / w`. Two pixels are 4-way neighbors if they are in the same row and next columns,
or in the same column and next rows. -/
theorem adj_iff {w n i j : Nat} (hw : 0 < w) (hj : j < n) :
    Adj w n i j ↔ (j / w = i / w ∧ (j % w + 1 = i % w ∨ j % w = i % w + 1)) ∨
      (j % w = i % w ∧ (j / w + 1 = i / w ∨ j / w = i / w + 1)) := by
  have hadj : Adj w n i j ↔ (0 < i % w ∧ j + 1 = i) ∨ (i % w + 1 < w ∧ j = i + 1) ∨
      (w ≤ i ∧ j + w = i) ∨ (j = i + w ∧ j < n) := by
    unfold Adj Near
    constructor
    · rintro ⟨d, hd, hn⟩
      match d, hd, hn with
      | 0, _, hn => exact Or.inl hn
      | 1, _, hn => exact Or.inr (Or.inl hn)
      | 2, _, hn => exact Or.inr (Or.inr (Or.inl hn))
      | 3, _, hn => exact Or.inr (Or.inr (Or.inr hn))
    · rintro (hn | hn | hn | hn)
      · exact ⟨0, by omega, hn⟩
      · exact ⟨1, by omega, hn⟩
      · exact ⟨2, by omega, hn⟩
      · exact ⟨3, by omega, hn⟩
  rw [hadj]
  have hi1 := Nat.mod_add_div i w
  have hj1 := Nat.mod_add_div j w
  have hi2 := Nat.mod_lt i hw
  have hj2 := Nat.mod_lt j hw
  have hf := mul_facts w (i / w) (j / w)
  generalize i / w = a at *
  generalize j / w = b at *
  generalize i % w = p at *
  generalize j % w = q at *
  generalize w * a = A at *
  generalize w * b = B at *
  omega

theorem nb_unique {w n d i j j' : Nat} (h : Nb w n d i j) (h' : Nb w n d i j') : j = j' := by
  match d, h, h' with
  | 0, h, h' => simp only [Nb] at h h'; omega
  | 1, h, h' => simp only [Nb] at h h'; omega
  | 2, h, h' => simp only [Nb] at h h'; omega
  | 3, h, h' => simp only [Nb] at h h'; omega

theorem near_succ {w n d i j : Nat} : Near w n (d + 1) i j ↔ Near w n d i j ∨ Nb w n d i j := by
  unfold Near
  constructor
  · rintro ⟨e, he, hn⟩
    by_cases hed : e = d
    · subst hed; exact Or.inr hn
    · exact Or.inl ⟨e, by omega, hn⟩
  · rintro (⟨e, he, hn⟩ | hn)
    · exact ⟨e, by omega, hn⟩
    · exact ⟨d, by omega, hn⟩

theorem not_near_zero {w n i j : Nat} : ¬ Near w n 0 i j := by
  rintro ⟨e, he, _⟩; omega

theorem region_color {w : Nat} {o : List Std.U32} {s k : Nat} (h : Region w o s k) :
    o[k]! = o[s]! := by
  induction h with
  | refl => rfl
  | tail _ hs _ => exact hs.2.2

theorem getElem!_set_eq' {α : Type} [Inhabited α] (v : List α) (j x : Nat) (a : α)
    (hj : j < v.length) : (v.set j a)[x]! = if x = j then a else v[x]! := by
  simp only [List.getElem!_eq_getElem?_getD, List.getElem?_set]
  split_ifs with h1 h2 h2
  · simp
  · exact absurd h1.symm h2
  · exact absurd h2.symm h1
  · rfl

/-! The loop invariant. `o` is the canvas before the fill, `s` the start pixel, `C`
the new color, `v` the pixels now, `st` the stack, and `t` the stack top. -/

/-- What the loop has done so far is right. -/
structure Core (w : Nat) (o : List Std.U32) (s : Nat) (C : Std.U32)
    (v : List Std.U32) (st : List Std.Usize) (t : Nat) : Prop where
  len : v.length = o.length
  slen : st.length = o.length
  px : ∀ k < o.length, v[k]! = o[k]! ∨ (o[k]! = o[s]! ∧ v[k]! = C ∧ Region w o s k)
  bound : t + v.count o[s]! ≤ o.length
  stack : ∀ m < t, st[m]!.val < o.length ∧ o[st[m]!.val]! = o[s]! ∧ v[st[m]!.val]! = C
  start : v[s]! = C

/-- Each changed pixel is on the stack, or its neighbors of the old color changed
too. For the pixel `i` that the loop works on, only the neighbor kinds `0..d` count. -/
def Closed (w : Nat) (o : List Std.U32) (s : Nat) (C : Std.U32) (v : List Std.U32)
    (st : List Std.Usize) (t d i : Nat) : Prop :=
  ∀ k < o.length, o[k]! = o[s]! → v[k]! = C →
    (∃ m < t, st[m]!.val = k) ∨
    ∀ j, Near w o.length (if k = i then d else 4) k j → j < o.length → o[j]! = o[s]! → v[j]! = C

/-- The state inside the loop body, after the checks of neighbor kinds `0..d` of `i`. -/
structure Work (w : Nat) (o : List Std.U32) (s : Nat) (C : Std.U32) (v : List Std.U32)
    (st : List Std.Usize) (t d i : Nat) : Prop where
  core : Core w o s C v st t
  closed : Closed w o s C v st t d i
  lt : i < o.length
  old : o[i]! = o[s]!
  now : v[i]! = C

section
variable {w : Nat} {o : List Std.U32} {s : Nat} {C : Std.U32} {v : List Std.U32}
  {st : List Std.Usize} {t d i : Nat}

theorem Core.region (hc : Core w o s C v st t) (hTC : o[s]! ≠ C) {k : Nat} (hk : k < o.length)
    (hko : o[k]! = o[s]!) (hv : v[k]! = C) : Region w o s k := by
  rcases hc.px k hk with h | ⟨_, _, h⟩
  · exact absurd (hko ▸ h ▸ hv) hTC
  · exact h

theorem pop_spec (hc : Core w o s C v st t) (hcl : Closed w o s C v st t 4 i) (ht : 0 < t) :
    Work w o s C v st (t - 1) 0 st[t - 1]!.val := by
  have hst := hc.stack (t - 1) (by omega)
  refine ⟨⟨hc.len, hc.slen, hc.px, by have := hc.bound; omega, fun m hm => hc.stack m (by omega),
    hc.start⟩, ?_, hst.1, hst.2.1, hst.2.2⟩
  intro k hk hko hkv
  rcases hcl k hk hko hkv with ⟨m, hm, hmk⟩ | hall
  · by_cases hmt : m < t - 1
    · exact Or.inl ⟨m, hmt, hmk⟩
    · have : m = t - 1 := by omega
      subst this
      right; intro j hj
      rw [if_pos hmk.symm] at hj
      exact absurd hj not_near_zero
  · right; intro j hj
    split_ifs at hj
    · exact absurd hj not_near_zero
    · exact hall j (by split_ifs at * <;> exact hj)

theorem closed_four {i' : Nat} (h : Closed w o s C v st t 4 i) : Closed w o s C v st t 4 i' := by
  intro k hk hko hkv
  rcases h k hk hko hkv with h1 | h1
  · exact Or.inl h1
  · right; intro j hj; apply h1; split_ifs at * <;> exact hj

theorem closed_succ (h : Closed w o s C v st t d i)
    (hd : ∀ j, Nb w o.length d i j → j < o.length → o[j]! = o[s]! → v[j]! = C) :
    Closed w o s C v st t (d + 1) i := by
  intro k hk hko hkv
  rcases h k hk hko hkv with h1 | h1
  · exact Or.inl h1
  · right; intro j hj
    by_cases hki : k = i
    · subst hki
      rw [if_pos rfl] at hj
      rcases near_succ.mp hj with hj | hj
      · exact h1 j (by rw [if_pos rfl]; exact hj)
      · exact hd j hj
    · rw [if_neg hki] at hj
      exact h1 j (by rw [if_neg hki]; exact hj)

/-- No neighbor of kind `d`: nothing to do. -/
theorem work_none (h : Work w o s C v st t d i) (hn : ∀ j, ¬ Nb w o.length d i j) :
    Work w o s C v st t (d + 1) i :=
  ⟨h.core, closed_succ h.closed (fun j hj => absurd hj (hn j)), h.lt, h.old, h.now⟩

/-- A neighbor of kind `d` that does not have the old color now: nothing to do. -/
theorem work_skip (h : Work w o s C v st t d i) {j : Nat} (hnb : Nb w o.length d i j)
    (hvj : v[j]! ≠ o[s]!) : Work w o s C v st t (d + 1) i := by
  refine ⟨h.core, closed_succ h.closed (fun j' hj' hjl hoj => ?_), h.lt, h.old, h.now⟩
  have := nb_unique hj' hnb
  subst this
  rcases h.core.px j' hjl with h1 | ⟨_, h1, _⟩
  · exact absurd (h1.trans hoj) hvj
  · exact h1

theorem set_C {v : List Std.U32} {j x : Nat} (hj : j < v.length) (h : x = j ∨ v[x]! = C) :
    (v.set j C)[x]! = C := by
  rw [getElem!_set_eq' v j x C hj]
  split_ifs with h1
  · rfl
  · exact h.resolve_left h1

/-- A neighbor `j` of kind `d` that has the old color: it takes the new color and goes
on the stack. The stack has room (P2), and the count of old-color pixels goes down. -/
theorem work_push (h : Work w o s C v st t d i) (hTC : o[s]! ≠ C) (hd : d < 4) {j : Nat}
    (hnb : Nb w o.length d i j) (hj : j < o.length) (hvj : v[j]! = o[s]!) (jj : Std.Usize)
    (hjj : jj.val = j) :
    t < o.length ∧ (v.set j C).count o[s]! + 1 = v.count o[s]! ∧
      Work w o s C (v.set j C) (st.set t jj) (t + 1) (d + 1) i := by
  have hc := h.core
  have hjv : j < v.length := by rw [hc.len]; exact hj
  have hvj' : v[j] = o[s]! := by rw [← getElem!_pos v j hjv]; exact hvj
  have hcount : (v.set j C).count o[s]! + 1 = v.count o[s]! := by
    rw [List.count_set hjv, hvj', beq_self_eq_true,
      beq_eq_false_iff_ne.mpr (Ne.symm hTC)]
    have : 1 ≤ v.count o[s]! := List.one_le_count_iff.mpr (hvj' ▸ List.getElem_mem hjv)
    simp only [if_true, Bool.false_eq_true, if_false]
    omega
  have ht : t < o.length := by have := hc.bound; omega
  have hst : t < st.length := by rw [hc.slen]; exact ht
  have hoj : o[j]! = o[s]! := by
    rcases hc.px j hj with h1 | ⟨h1, _, _⟩
    · rw [← h1]; exact hvj
    · exact h1
  have hreg : Region w o s j :=
    Relation.ReflTransGen.tail (hc.region hTC h.lt h.old h.now) ⟨⟨d, hd, hnb⟩, hj, hoj⟩
  refine ⟨ht, hcount, ⟨⟨by simp [hc.len], by simp [hc.slen], ?_, ?_, ?_, ?_⟩, ?_, h.lt, h.old, ?_⟩⟩
  · intro k hk
    rw [getElem!_set_eq' v j k C hjv]
    split_ifs with hkj
    · subst hkj; exact Or.inr ⟨hoj, rfl, hreg⟩
    · exact hc.px k hk
  · have := hc.bound; omega
  · intro m hm
    rw [getElem!_set_eq' st t m jj hst]
    split_ifs with hmt
    · rw [hjj]; exact ⟨hj, hoj, set_C hjv (Or.inl rfl)⟩
    · have := hc.stack m (by omega)
      exact ⟨this.1, this.2.1, set_C hjv (Or.inr this.2.2)⟩
  · exact set_C hjv (Or.inr hc.start)
  · intro k hk hko hkv
    by_cases hkj : k = j
    · left; refine ⟨t, by omega, ?_⟩
      rw [getElem!_set_eq' st t t jj hst, if_pos rfl, hjj, hkj]
    · rw [getElem!_set_eq' v j k C hjv, if_neg hkj] at hkv
      rcases h.closed k hk hko hkv with ⟨m, hm, hmk⟩ | h1
      · left; refine ⟨m, by omega, ?_⟩
        rw [getElem!_set_eq' st t m jj hst, if_neg (by omega), hmk]
      · right; intro j' hj' hj'l hoj'
        by_cases hjj' : j' = j
        · exact set_C hjv (Or.inl hjj')
        · apply set_C hjv (Or.inr _)
          by_cases hki : k = i
          · subst hki
            rw [if_pos rfl] at hj'
            rcases near_succ.mp hj' with hj' | hj'
            · exact h1 j' (by rw [if_pos rfl]; exact hj') hj'l hoj'
            · exact absurd (nb_unique hj' hnb) hjj'
          · rw [if_neg hki] at hj'
            exact h1 j' (by rw [if_neg hki]; exact hj') hj'l hoj'
  · exact set_C hjv (Or.inr h.now)

end

/-- The state before the loop: the start pixel has the new color and is on the stack. -/
theorem init_spec {w : Nat} {o : List Std.U32} {s : Nat} {C : Std.U32} (hs : s < o.length)
    (hTC : o[s]! ≠ C) (ss : Std.Usize) (hss : ss.val = s) :
    Core w o s C (o.set s C) ((List.replicate o.length 0#usize).set 0 ss) 1 ∧
      Closed w o s C (o.set s C) ((List.replicate o.length 0#usize).set 0 ss) 1 4 0 := by
  have hs' : o[s] = o[s]! := (getElem!_pos o s hs).symm
  have hst : 0 < (List.replicate o.length 0#usize).length := by simp; omega
  have hv : ∀ k, (o.set s C)[k]! = if k = s then C else o[k]! :=
    fun k => getElem!_set_eq' o s k C hs
  refine ⟨⟨by simp, by simp, ?_, ?_, ?_, ?_⟩, ?_⟩
  · intro k hk
    rw [hv k]
    split_ifs with hks
    · subst hks; exact Or.inr ⟨rfl, rfl, Relation.ReflTransGen.refl⟩
    · exact Or.inl rfl
  · rw [List.count_set hs, hs', beq_self_eq_true, beq_eq_false_iff_ne.mpr (Ne.symm hTC)]
    have : 1 ≤ o.count o[s]! := List.one_le_count_iff.mpr (hs' ▸ List.getElem_mem hs)
    have := List.count_le_length (a := o[s]!) (l := o)
    simp only [if_true, Bool.false_eq_true, if_false]
    omega
  · intro m hm
    have : m = 0 := by omega
    subst this
    rw [getElem!_set_eq' _ 0 0 ss hst, if_pos rfl, hss, hv s, if_pos rfl]
    exact ⟨hs, rfl, rfl⟩
  · rw [hv s, if_pos rfl]
  · intro k hk hko hkv
    rw [hv k] at hkv
    split_ifs at hkv with hks
    · left; refine ⟨0, by omega, ?_⟩
      rw [getElem!_set_eq' _ 0 0 ss hst, if_pos rfl, hss, hks]
    · exact absurd (hko ▸ hkv) hTC

/-- The state after the loop: the stack is empty, so the changed pixels are exactly
the region. -/
theorem final_spec {w : Nat} {o : List Std.U32} {s : Nat} {C : Std.U32} {v : List Std.U32}
    {st : List Std.Usize} (hs : s < o.length) (hc : Core w o s C v st 0)
    (hcl : Closed w o s C v st 0 4 0) (k : Nat) (hk : k < o.length) :
    (Region w o s k → v[k]! = C) ∧ (¬ Region w o s k → v[k]! = o[k]!) := by
  refine ⟨fun hr => ?_, fun hr => ?_⟩
  · induction hr with
    | refl => exact hc.start
    | @tail a b hab hstep ih =>
      have hal : a < o.length := by
        rcases Relation.ReflTransGen.cases_tail hab with h | ⟨_, _, h⟩
        · rw [h]; exact hs
        · exact h.2.1
      rcases hcl a hal (region_color hab) (ih hal) with ⟨m, hm, _⟩ | h
      · omega
      · exact h b (by split_ifs <;> exact hstep.1) hstep.2.1 hstep.2.2
  · rcases hc.px k hk with h1 | ⟨_, _, h1⟩
    · exact h1
    · exact absurd h1 hr

/-! The Rust loop. -/

/-- The loop state inside the body, with the measure `M` of P2: old-color pixels plus
the stack top. -/
def Mid (w : Nat) (o : List Std.U32) (s : Nat) (C : Std.U32) (i d M : Nat)
    (r : alloc.vec.Vec Std.U32 × alloc.vec.Vec Std.Usize × Std.Usize) : Prop :=
  Work w o s C r.1.val r.2.1.val r.2.2.val d i ∧ r.1.val.count o[s]! + r.2.2.val = M

/-- One neighbor check of the loop body, with the neighbor `j` of kind `d`. -/
theorem push_code {w : Nat} {o : List Std.U32} {s : Nat} {C : Std.U32} (hTC : o[s]! ≠ C)
    {d i M : Nat} (hd : d < 4) (v : alloc.vec.Vec Std.U32) (stack : alloc.vec.Vec Std.Usize)
    (top j : Std.Usize) (target color : Std.U32) (ht : target = o[s]!) (hcol : color = C)
    (h : Mid w o s C i d M (v, stack, top)) (hnb : Nb w o.length d i j.val)
    (hj : j.val < o.length) :
    (do
      let i1 ← v.index_usize j
      if i1 = target then
        let (_, index_mut_back) ← v.index_mut_usize j
        let (_, index_mut_back1) ← stack.index_mut_usize top
        let top3 ← top + 1#usize
        ok (index_mut_back color, index_mut_back1 j, top3)
      else ok (v, stack, top)) ⦃ r => Mid w o s C i (d + 1) M r ⦄ := by
  obtain ⟨hw, hM⟩ := h
  simp only at hw hM
  have hlen := hw.core.len
  have hslen := hw.core.slen
  have hjv : j.val < v.val.length := by omega
  step
  by_cases hi : i1 = target
  · rw [if_pos hi]
    have hvj : v.val[j.val]! = o[s]! := by rw [getElem!_pos v.val j.val hjv, ← i1_post, hi, ht]
    obtain ⟨ht', hcnt, hW'⟩ := work_push hw hTC hd hnb hj hvj j rfl
    have : top.val < stack.val.length := by omega
    step*
    unfold Mid
    simp only [index_mut_back_post, index_mut_back1_post, alloc.vec.Vec.set_val_eq, top3_post, hcol]
    exact ⟨hW', by omega⟩
  · rw [if_neg hi]
    simp only [WP.spec_ok]
    refine ⟨work_skip hw hnb ?_, hM⟩
    rw [getElem!_pos v.val j.val hjv, ← i1_post, ← ht]; exact hi

/-- The last neighbor check, which also ends the body: the same as `push_code`. -/
theorem push_code_cont {w : Nat} {o : List Std.U32} {s : Nat} {C : Std.U32} (hTC : o[s]! ≠ C)
    {d i M : Nat} (hd : d < 4) (v : alloc.vec.Vec Std.U32) (stack : alloc.vec.Vec Std.Usize)
    (top j : Std.Usize) (target color : Std.U32) (ht : target = o[s]!) (hcol : color = C)
    (h : Mid w o s C i d M (v, stack, top)) (hnb : Nb w o.length d i j.val)
    (hj : j.val < o.length) :
    (do
      let i2 ← v.index_usize j
      if i2 = target then
        let (_, index_mut_back) ← v.index_mut_usize j
        let (_, index_mut_back1) ← stack.index_mut_usize top
        let top5 ← top + 1#usize
        ok (ControlFlow.cont (index_mut_back color, index_mut_back1 j, top5))
      else ok (ControlFlow.cont (v, stack, top)) :
      Result (ControlFlow (alloc.vec.Vec Std.U32 × alloc.vec.Vec Std.Usize × Std.Usize)
        (alloc.vec.Vec Std.U32))) ⦃ r =>
        ∃ x, r = ControlFlow.cont x ∧ Mid w o s C i (d + 1) M x ⦄ := by
  obtain ⟨hw, hM⟩ := h
  simp only at hw hM
  have hlen := hw.core.len
  have hslen := hw.core.slen
  have hjv : j.val < v.val.length := by omega
  step
  by_cases hi : i2 = target
  · rw [if_pos hi]
    have hvj : v.val[j.val]! = o[s]! := by rw [getElem!_pos v.val j.val hjv, ← i2_post, hi, ht]
    obtain ⟨ht', hcnt, hW'⟩ := work_push hw hTC hd hnb hj hvj j rfl
    have : top.val < stack.val.length := by omega
    step*
    refine ⟨_, rfl, ?_⟩
    unfold Mid
    simp only [index_mut_back_post, index_mut_back1_post, alloc.vec.Vec.set_val_eq, top5_post, hcol]
    exact ⟨hW', by omega⟩
  · rw [if_neg hi]
    simp only [WP.spec_ok]
    refine ⟨_, rfl, work_skip hw hnb ?_, hM⟩
    rw [getElem!_pos v.val j.val hjv, ← i2_post, ← ht]; exact hi

theorem right_lt {i w n : Nat} (hi : i < n) (hn : n % w = 0) (h : i % w + 1 < w) :
    i + 1 < n := by
  have h1 := Nat.div_add_mod i w
  have h2 := Nat.div_add_mod n w
  rw [hn, Nat.add_zero] at h2
  have hq : i / w < n / w := by
    by_contra hc
    have : w * (n / w) ≤ w * (i / w) := Nat.mul_le_mul_left _ (by omega)
    omega
  have : w * (i / w + 1) ≤ w * (n / w) := Nat.mul_le_mul_left _ hq
  rw [Nat.mul_add, Nat.mul_one] at this
  omega

/-- The loop of `flood_fill` (P1, P2). It ends (`ok`), with the region in the new color
and every other pixel as before. -/
theorem flood_fill_loop_spec {o : List Std.U32} {s : Nat} {C : Std.U32} (hTC : o[s]! ≠ C)
    (hs : s < o.length) (w n : Std.Usize) (hw0 : 0 < w.val) (hw : w.val ≤ 4096)
    (hrow : o.length % w.val = 0) (hn : n.val = o.length) (hon : o.length ≤ 4096 * 4096)
    (target color : Std.U32) (ht : target = o[s]!) (hcol : color = C)
    (v : alloc.vec.Vec Std.U32) (stack : alloc.vec.Vec Std.Usize) (top : Std.Usize)
    (hc : Core w.val o s C v.val stack.val top.val)
    (hcl : Closed w.val o s C v.val stack.val top.val 4 0) :
    flood_fill_loop v color target w n stack top ⦃ r => r.val.length = o.length ∧
      ∀ k < o.length, (Region w.val o s k → r.val[k]! = C) ∧
        (¬ Region w.val o s k → r.val[k]! = o[k]!) ⦄ := by
  unfold flood_fill_loop
  apply loop.spec_decr_nat
    (measure := fun (x : alloc.vec.Vec Std.U32 × alloc.vec.Vec Std.Usize × Std.Usize) =>
      x.1.val.count o[s]! + x.2.2.val)
    (inv := fun x => Core w.val o s C x.1.val x.2.1.val x.2.2.val ∧
      Closed w.val o s C x.1.val x.2.1.val x.2.2.val 4 0)
  · rintro ⟨v, stack, top⟩ ⟨hc, hcl⟩
    simp only at hc hcl
    have hlen := hc.len
    have hslen := hc.slen
    have hb := hc.bound
    dsimp only
    unfold flood_fill_loop.body
    split
    · have hW := pop_spec hc hcl (by scalar_tac)
      step
      step
      step
      have hi : i.val = stack.val[top.val - 1]!.val := by
        rw [i_post, getElem!_pos stack.val (top.val - 1) (by omega)]
        simp [top1_post]
      rw [← hi] at hW
      have hil := hW.lt
      have h0 : Mid w.val o s C i.val 0 (v.val.count o[s]! + top.val - 1) (v, stack, top1) :=
        ⟨by simpa [top1_post] using hW, by simp only [top1_post]; omega⟩
      apply WP.spec_bind (Pₘ := Mid w.val o s C i.val 1 (v.val.count o[s]! + top.val - 1))
      · split
        · have : 0 < i.val % w.val := by scalar_tac
          have : 1 ≤ i.val := by
            rcases Nat.eq_zero_or_pos i.val with h | h
            · rw [h, Nat.zero_mod] at this; omega
            · omega
          step as ⟨j, hjp⟩
          exact push_code hTC (by omega) v stack top1 j target color ht hcol h0
            (by simp only [Nb]; scalar_tac) (by scalar_tac)
        · simp only [WP.spec_ok]
          exact ⟨work_none h0.1 (fun j hj => by simp only [Nb] at hj; scalar_tac), h0.2⟩
      rintro ⟨v1, stack1, top2⟩ h1
      step as ⟨c1, hc1⟩
      apply WP.spec_bind (Pₘ := Mid w.val o s C i.val 2 (v.val.count o[s]! + top.val - 1))
      · split
        · have : i.val + 1 < o.length := right_lt hil hrow (by scalar_tac)
          step as ⟨j, hjp⟩
          exact push_code hTC (by omega) v1 stack1 top2 j target color ht hcol h1
            (by simp only [Nb]; scalar_tac) (by scalar_tac)
        · simp only [WP.spec_ok]
          exact ⟨work_none h1.1 (fun j hj => by simp only [Nb] at hj; scalar_tac), h1.2⟩
      rintro ⟨v2, stack2, top3⟩ h2
      apply WP.spec_bind (Pₘ := Mid w.val o s C i.val 3 (v.val.count o[s]! + top.val - 1))
      · split
        · have : w.val ≤ i.val := by scalar_tac
          step as ⟨j, hjp⟩
          exact push_code hTC (by omega) v2 stack2 top3 j target color ht hcol h2
            (by simp only [Nb]; scalar_tac) (by scalar_tac)
        · simp only [WP.spec_ok]
          exact ⟨work_none h2.1 (fun j hj => by simp only [Nb] at hj; scalar_tac), h2.2⟩
      rintro ⟨v3, stack3, top4⟩ h3
      step as ⟨j, hjp⟩
      have hfin : ∀ x, Mid w.val o s C i.val 4 (v.val.count o[s]! + top.val - 1) x →
          Core w.val o s C x.1.val x.2.1.val x.2.2.val ∧
            Closed w.val o s C x.1.val x.2.1.val x.2.2.val 4 0 ∧
          x.1.val.count o[s]! + x.2.2.val < v.val.count o[s]! + top.val := by
        intro x hx
        exact ⟨hx.1.core, closed_four hx.1.closed, by have := hx.2; scalar_tac⟩
      split
      · apply WP.spec_mono (push_code_cont hTC (by omega) v3 stack3 top4 j target color ht hcol h3
          (by simp only [Nb]; scalar_tac) (by scalar_tac))
        rintro r ⟨x, rfl, hx⟩
        exact hfin x hx
      · simp only [WP.spec_ok]
        exact hfin _ ⟨work_none h3.1 (fun j hj => by simp only [Nb] at hj; scalar_tac), h3.2⟩
    · simp only [WP.spec_ok]
      have h0 : top.val = 0 := by scalar_tac
      rw [h0] at hc hcl
      exact ⟨hlen, fun k hk => final_spec hs hc hcl k hk⟩
  · exact ⟨hc, hcl⟩


/-- The pixels that `flood_fill c x y color` changes (P1): `(x, y)` is a pixel, its color
is not `color`, and the pixel is in the region of `(x, y)`. -/
def Filled (c : Canvas) (x y : Int) (color : Std.U32) (k : Nat) : Prop :=
  Inside c x y ∧ c.px.val[y.toNat * c.w.val + x.toNat]! ≠ color ∧
    Region c.w.val c.px.val (y.toNat * c.w.val + x.toNat) k

/-- P1 and P2: `flood_fill` ends without a panic. The pixels in the region of `(x, y)`
take the new color, and every other pixel stays the same. -/
theorem flood_fill_spec (c : Canvas) (x y : Std.I32) (color : Std.U32) :
    flood_fill c x y color ⦃ r => r.w = c.w ∧ r.h = c.h ∧
      r.px.val.length = c.px.val.length ∧
      ∀ k < c.px.val.length, (Filled c x.val y.val color k → r.px.val[k]! = color) ∧
        (¬ Filled c x.val y.val color k → r.px.val[k]! = c.px.val[k]!) ⦄ := by
  unfold flood_fill
  step as ⟨o, ho⟩
  cases o with
  | none =>
    simp only at ho
    simp only [WP.spec_ok, true_and]
    intro k _
    exact ⟨fun h => absurd h.1 ho, fun _ => trivial⟩
  | some start =>
    simp only at ho
    obtain ⟨hin, hstart⟩ := ho
    have hsl := hin.lt
    rw [← hstart] at hsl
    step as ⟨target, ht⟩
    split
    · rename_i htc
      simp only [WP.spec_ok, true_and]
      intro k _
      refine ⟨fun h => absurd ?_ h.2.1, fun _ => trivial⟩
      rw [← hstart, getElem!_pos c.px.val start.val hsl, ← ht, htc]
    · rename_i htc
      step*
      obtain ⟨hw1, -, hw4, hh4, hlen⟩ := hin.1
      have hTC : c.px.val[start.val]! ≠ color := by
        rw [getElem!_pos c.px.val start.val hsl, ← ht]; exact htc
      obtain ⟨hc, hcl⟩ := init_spec (w := c.w.val) hsl hTC start rfl
      have hrow : c.px.val.length % c.w.val = 0 := by rw [hlen]; exact Nat.mul_mod_right _ _
      have hon : c.px.val.length ≤ 4096 * 4096 := by
        rw [hlen]; exact Nat.mul_le_mul hw4 hh4
      subst index_mut_back_post index_mut_back1_post
      have hsv : (stack.set 0#usize start).val =
          (List.replicate c.px.val.length 0#usize).set 0 start := by simp [stack_post]
      apply WP.spec_bind (flood_fill_loop_spec hTC hsl c.w c.px.len hw1 hw4 hrow (by simp) hon
        target color (by rw [ht, getElem!_pos c.px.val start.val hsl]) rfl
        (c.px.set start color) (stack.set 0#usize start) 1#usize
        (by simpa [hsv] using hc) (by simpa [hsv] using hcl))
      rintro v1 ⟨hl, hk⟩
      simp only [WP.spec_ok]
      refine ⟨trivial, trivial, hl, fun k hk' => ?_⟩
      have hF : Filled c x.val y.val color k ↔ Region c.w.val c.px.val start.val k := by
        unfold Filled
        rw [← hstart]
        exact ⟨fun h => h.2.2, fun h => ⟨hin, hTC, h⟩⟩
      rw [hF]
      exact hk k hk' 

/-- P2: `flood_fill` ends without a panic for every input. -/
theorem flood_fill_ok (c : Canvas) (x y : Std.I32) (color : Std.U32) :
    flood_fill c x y color ⦃ _ => True ⦄ :=
  WP.spec_mono (flood_fill_spec c x y color) (fun _ _ => trivial)

/-- P2: the stack top never passes the number of pixels. -/
theorem stack_bound {w : Nat} {o : List Std.U32} {s : Nat} {C : Std.U32} {v : List Std.U32}
    {st : List Std.Usize} {t : Nat} (h : Core w o s C v st t) : t ≤ o.length := by
  have := h.bound; omega

/-- P1: if no pixel is in the region (the canvas is not valid, `(x, y)` is outside, or
it has the color `color` already), the canvas does not change. -/
theorem flood_fill_unchanged (c : Canvas) (x y : Std.I32) (color : Std.U32)
    (h : ∀ k, ¬ Filled c x.val y.val color k) : flood_fill c x y color ⦃ r => r = c ⦄ := by
  apply WP.spec_mono (flood_fill_spec c x y color)
  rintro ⟨w, h', px⟩ ⟨hw, hh, hl, hk⟩
  simp only at hw hh hl hk
  subst hw hh
  congr
  apply alloc.vec.Vec.ext
  apply List.ext_getElem hl
  intro k h1 h2
  have := (hk k h2).2 (h k)
  rwa [getElem!_pos px.val k h1, getElem!_pos c.px.val k h2] at this

/-- `flood_fill` does not change a canvas that is not valid. -/
theorem flood_fill_invalid (c : Canvas) (x y : Std.I32) (color : Std.U32) (h : ¬ Valid c) :
    flood_fill c x y color ⦃ r => r = c ⦄ :=
  flood_fill_unchanged c x y color (fun _ hk => h hk.1.1)

end Logbook.Paint
