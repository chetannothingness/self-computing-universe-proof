import KernelVm.UCert.Universe
import KernelVm.UCert.Cert
import KernelVm.UCert.Check
import KernelVm.UCert.CheckSound
import KernelVm.UCert.Completeness
import KernelVm.UCert.NF

/-!
# Universal Certificate Calculus — Module Root

The UCert module provides a universal, complete certificate calculus
for the kernel's proof system. It extends the existing IRC framework
with a universal statement language, certificate types, and a complete
enumerator.

Pipeline:
  Compile S to U → Enumerate Cert by rank → Check(S, cert) → PROVED(S, π)

Components:
  Universe.lean     — Statement type (the universal object language)
  Cert.lean         — Certificate types (finite, checkable proof witnesses)
  Check.lean        — Universal checker (total, decidable, the ONLY judge)
  CheckSound.lean   — Soundness theorem (Check = true → holds)
  Completeness.lean — Completeness theorem (∃ cert → ∃ rank, E(rank) passes)
  NF.lean           — Normal form computation (PROVED or FRONTIER)
-/
