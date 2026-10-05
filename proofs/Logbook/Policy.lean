import LogbookCore.Funs
open Aeneas Aeneas.Std Result logbook_core

/-! Access rules (spec 7.6): T1, T2, T3, with T12 (no panic) built in.
`f x ⦃ r => P r ⦄` means: `f x` returns `ok r` and `P r` holds. -/

namespace Logbook

/-- A post is public. -/
def isPublic (p : post.Post) : Bool :=
  match p.state with
  | .Public => true
  | _ => false

/-- Cloning a byte vector gives the same vector. -/
@[step]
theorem vec_u8_clone_spec (v : alloc.vec.Vec Std.U8) :
    alloc.vec.CloneVec.clone core.clone.CloneU8 v ⦃ w => w = v ⦄ := by
  unfold alloc.vec.CloneVec.clone
  apply WP.spec_bind (Slice.clone_spec (by intro x _; rfl))
  intro s hs
  simp [← hs]

@[step]
theorem clone_opt_spec (v : Option (alloc.vec.Vec Std.U8)) :
    post.clone_opt v ⦃ w => w = v ⦄ := by
  unfold post.clone_opt
  cases v <;> step*

/-- The hand-written `Clone` for `Post` returns the same post. -/
@[step]
theorem post_clone_spec (p : post.Post) :
    post.Post.Insts.CoreCloneClone.clone p ⦃ q => q = p ⦄ := by
  unfold post.Post.Insts.CoreCloneClone.clone
  step*
  subst_vars
  rfl

/-- T1 and T2: `reveal` passes exactly the public posts, unchanged. -/
@[step]
theorem reveal_spec (p : post.Post) :
    policy.reveal p ⦃ r => r = (if isPublic p then some p else none) ⦄ := by
  unfold policy.reveal isPublic
  cases h : p.state <;> simp [h]

theorem reveal_iff (p : post.Post) :
    policy.reveal p ⦃ r => (r.isSome ↔ p.state = .Public) ∧ ∀ q, r = some q → q = p ⦄ := by
  unfold policy.reveal
  cases h : p.state <;> simp [h]

theorem filter_public_loop_spec (ps : Slice post.Post) (out : alloc.vec.Vec policy.PublicPost)
    (i : Std.Usize) (hi : i.val ≤ ps.length)
    (hout : out.val = (ps.val.take i.val).filter isPublic) :
    policy.filter_public_loop ps out i ⦃ r => r.val = ps.val.filter isPublic ⦄ := by
  unfold policy.filter_public_loop
  apply loop.spec_decr_nat
    (measure := fun (x : alloc.vec.Vec policy.PublicPost × Std.Usize) => ps.length - x.2.val)
    (inv := fun x => x.2.val ≤ ps.length ∧ x.1.val = (ps.val.take x.2.val).filter isPublic)
  · rintro ⟨out, i⟩ ⟨hi, hout⟩
    unfold policy.filter_public_loop.body
    simp only
    split
    · step*
      simp only at hi hout
      have hlen : out.val.length ≤ i.val := by
        rw [hout]
        exact (List.length_filter_le _ _).trans (by simp)
      have htake : List.take (i.val + 1) ps.val = List.take i.val ps.val ++ [p] := by
        rw [p_post]
        exact List.take_succ_eq_append_getElem _
      rw [o_post]
      by_cases hp : isPublic p1 = true
      · simp only [hp, if_true]
        step*
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [i2_post, htake, List.filter_append, out1_post, hout, ← p1_post]
        simp [hp]
      · simp only [hp, Bool.false_eq_true, if_false]
        step*
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [i2_post, htake, List.filter_append, hout, ← p1_post]
        simp [hp]
    · simp only at hi hout
      simp only [WP.spec_ok]
      have : i.val = ps.length := by scalar_tac
      simp [hout, this]
  · exact ⟨hi, hout⟩

/-- T3 + T12: the guest list is exactly the public posts, in order. -/
theorem filter_public_spec (ps : Slice post.Post) :
    policy.filter_public ps ⦃ qs => qs.val = ps.val.filter isPublic ⦄ := by
  unfold policy.filter_public
  apply filter_public_loop_spec <;> simp

end Logbook
