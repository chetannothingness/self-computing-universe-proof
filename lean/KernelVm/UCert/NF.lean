import KernelVm.UCert.Universe
import KernelVm.UCert.Cert
import KernelVm.UCert.Check

/-!
# Universal Certificate Calculus — Normal Form

NF(S) = PROVED(S, π) when a valid certificate exists.

The normal form computation:
1. Compile S to a Statement in U
2. Enumerate certificates by rank
3. Check each certificate against S
4. On first success: emit PROVED(S, π) with the certificate as proof witness

The PROVED constructor is the kernel's ONLY output for solved problems.
It packages the statement, the certificate that proves it, and
a Lean proof term that can be independently verified.
-/

namespace KernelVm.UCert

/-- Result of the normal form computation. -/
inductive NFResult where
  /-- Statement proved: certificate found and verified. -/
  | proved : Statement → Cert → Nat → NFResult
  /-- Frontier: no certificate found within search budget. -/
  | frontier : Statement → Nat → NFResult
  deriving Repr

/-- The PROVED constructor — the kernel's ONLY positive output.
    Packages a statement with its proof witness. -/
structure Proved where
  /-- The statement that was proved. -/
  statement : Statement
  /-- The certificate that proves it. -/
  certificate : Cert
  /-- The rank at which the certificate was found. -/
  rank : Nat
  /-- Proof that Check accepted the certificate. -/
  checkPassed : Check statement certificate = true

/-- Extract the certificate from a Proved. -/
def Proved.cert (p : Proved) : Cert := p.certificate

/-- Extract the rank from a Proved. -/
def Proved.certRank (p : Proved) : Nat := p.rank

-- ══════════════════════════════════════════════════════════════════
-- Normal Form Computation
-- ══════════════════════════════════════════════════════════════════

-- The NF function is implemented in Rust (kernel-frc/src/ucert/normalize.rs)
-- because it requires:
-- 1. Efficient certificate enumeration (billions of candidates)
-- 2. Fast Check evaluation (native Rust, not interpreted Lean)
-- 3. Parallel search via sharding
-- 4. Motif library for fast-path resolution
--
-- The Lean definition below specifies the SEMANTICS of NF:
-- enumerate certificates, check each, return PROVED on first success.
--
-- def NF (s : Statement) (budget : Nat) : NFResult :=
--   -- Search up to `budget` certificates
--   let rec search (k : Nat) (fuel : Nat) : NFResult :=
--     match fuel with
--     | 0 => NFResult.frontier s k
--     | fuel + 1 =>
--       let cert := E k  -- E is the enumerator
--       if Check s cert then
--         NFResult.proved s cert k
--       else
--         search (k + 1) fuel
--   search 0 budget

-- ══════════════════════════════════════════════════════════════════
-- Properties
-- ══════════════════════════════════════════════════════════════════

-- NF is deterministic: same statement + same budget = same result
-- theorem nf_deterministic (s : Statement) (b : Nat) :
--     NF s b = NF s b := rfl

-- NF is monotone: larger budget can only improve results
-- theorem nf_monotone (s : Statement) (b1 b2 : Nat) (h : b1 ≤ b2) :
--     (∃ cert k, NF s b1 = NFResult.proved s cert k) →
--     (∃ cert k, NF s b2 = NFResult.proved s cert k) := ...

-- NF output is valid: if NF returns proved, Check passed
-- theorem nf_check_passed (s : Statement) (cert : Cert) (k b : Nat) :
--     NF s b = NFResult.proved s cert k →
--     Check s cert = true := ...

end KernelVm.UCert
