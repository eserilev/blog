import LogbookCore.Funs
open Aeneas Aeneas.Std Result logbook_core

/-! Client IP selection (spec 7.6): T15, with T12 (no panic) built in.
`hops[j]` is true if hop `j` of `X-Forwarded-For`, from the left, is a trusted proxy. -/

namespace Logbook

/-- The rule of T15. An untrusted peer is the result. For a trusted peer, the result
is the rightmost untrusted hop, or the peer if every hop is trusted. -/
def ClientRule (peerTrusted : Bool) (hops : List Bool) (r : ip.ClientAddr) : Prop :=
  if peerTrusted then
    (r = .Peer ∧ ∀ j < hops.length, hops[j]! = true) ∨
    (∃ i : Std.Usize, r = .Hop i ∧ i.val < hops.length ∧ hops[i.val]! = false ∧
      ∀ j, i.val < j → j < hops.length → hops[j]! = true)
  else r = .Peer

theorem client_addr_loop_spec (hops : Slice Bool) (i : Std.Usize) (hi : i.val ≤ hops.length)
    (hpre : ∀ j, i.val ≤ j → j < hops.length → hops.val[j]! = true) :
    ip.client_addr_loop hops i ⦃ r => ClientRule true hops.val r ⦄ := by
  unfold ip.client_addr_loop
  apply loop.spec_decr_nat
    (measure := fun (i : Std.Usize) => i.val)
    (inv := fun i => i.val ≤ hops.length ∧ ∀ j, i.val ≤ j → j < hops.length → hops.val[j]! = true)
  · rintro i ⟨hi, hpre⟩
    unfold ip.client_addr_loop.body
    step*
    · -- A trusted hop: go on to the left.
      refine ⟨by scalar_tac, ?_, by scalar_tac⟩
      intro j hj hjl
      by_cases hji : j = i1.val
      · subst hji
        rw [getElem!_pos _ _ (by scalar_tac)]
        simp_all
      · exact hpre j (by scalar_tac) hjl
    · -- An untrusted hop: the result.
      simp only [ClientRule, if_true]
      refine Or.inr ⟨i1, rfl, by scalar_tac, ?_, fun j hj hjl => hpre j (by scalar_tac) hjl⟩
      rw [getElem!_pos _ _ (by scalar_tac)]
      simp_all
    · -- No hop left: the peer.
      simp only [ClientRule, if_true]
      exact Or.inl ⟨by simp, fun j hj => hpre j (by scalar_tac) hj⟩
  · exact ⟨hi, hpre⟩

/-- T15 + T12: `client_addr` follows `ClientRule`. -/
theorem client_addr_spec (peerTrusted : Bool) (hops : Slice Bool) :
    ip.client_addr peerTrusted hops ⦃ r => ClientRule peerTrusted hops.val r ⦄ := by
  unfold ip.client_addr
  cases peerTrusted
  · simp [ClientRule]
  · simp only [if_true]
    apply client_addr_loop_spec <;> simp_all [Slice.length]

/-- T15: a peer that is not a trusted proxy is always the result. The header has no
effect, so a visitor cannot choose the IP. -/
theorem client_addr_untrusted_peer (hops : Slice Bool) :
    ip.client_addr false hops ⦃ r => r = .Peer ⦄ := by
  unfold ip.client_addr
  simp

end Logbook
