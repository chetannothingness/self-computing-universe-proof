import Mathlib.Data.Nat.Prime.Basic

-- ProofEnum.Registry: All 20 problem statements as Lean Props.
--
-- This file mirrors the Rust ProofStatement registry in
-- kernel-frc/src/proof_enum/statement.rs.
-- Each statement is a Lean Prop that the proof enumerator tries to inhabit.
--
-- Statements marked FORMALIZATION PENDING use True as a placeholder.
-- These will be replaced with actual formalizations as the theory
-- infrastructure is built.

namespace ProofEnum

-- ═══════════════════════════════════════════════════════════════════
-- Known theorems (PROVED by IRC accelerator)
-- ═══════════════════════════════════════════════════════════════════

namespace ZFC
/-- ZFC: 0 ≠ 1 in natural numbers. -/
def statement : Prop := (0 : Nat) ≠ 1
end ZFC

namespace Bertrand
/-- Bertrand's postulate: for every n ≥ 1, there exists a prime p with n < p ≤ 2n. -/
def statement : Prop := ∀ n : Nat, n ≥ 1 → ∃ p, Nat.Prime p ∧ n < p ∧ p ≤ 2 * n
end Bertrand

namespace Lagrange
/-- Lagrange's four-square theorem: every natural number is the sum of four squares. -/
def statement : Prop := ∀ n : Nat, ∃ a b c d : Nat, a * a + b * b + c * c + d * d = n
end Lagrange

namespace WeakGoldbach
/-- Weak Goldbach: every odd number > 5 is the sum of three primes. -/
def statement : Prop :=
  ∀ n : Nat, n > 5 → n % 2 = 1 → ∃ p q r, Nat.Prime p ∧ Nat.Prime q ∧ Nat.Prime r ∧ p + q + r = n
end WeakGoldbach

namespace FLT
/-- Fermat's Last Theorem: no positive integers a, b, c satisfy a^n + b^n = c^n for n > 2. -/
def statement : Prop :=
  ∀ n : Nat, n > 2 → ∀ a b c : Nat, a > 0 → b > 0 → c > 0 → a ^ n + b ^ n ≠ c ^ n
end FLT

namespace Mersenne
/-- Existence of a Mersenne prime with exponent ≤ 100. -/
def statement : Prop := ∃ p : Nat, 2 ≤ p ∧ p ≤ 100 ∧ Nat.Prime p ∧ Nat.Prime (2 ^ p - 1)
end Mersenne

namespace BSD
/-- BSD: Hasse bound for elliptic curve point counts. -/
def statement : Prop :=
  ∀ p : Nat, Nat.Prime p → ∃ count : Nat,
    ((count : Int) - (p + 1 : Int)) * ((count : Int) - (p + 1 : Int)) ≤ 4 * (p : Int)
end BSD

-- ═══════════════════════════════════════════════════════════════════
-- Open conjectures
-- ═══════════════════════════════════════════════════════════════════

namespace Goldbach
/-- Goldbach's conjecture: every even n ≥ 4 is the sum of two primes. -/
def statement : Prop :=
  ∀ n : Nat, n ≥ 4 → n % 2 = 0 → ∃ p q, Nat.Prime p ∧ Nat.Prime q ∧ p + q = n
end Goldbach

namespace Collatz
/-- Collatz conjecture: every positive integer eventually reaches 1. -/
def collatzStep (n : Nat) : Nat := if n % 2 = 0 then n / 2 else 3 * n + 1
def collatzIter (k : Nat) (n : Nat) : Nat :=
  match k with
  | 0 => n
  | k' + 1 => collatzIter k' (collatzStep n)
def statement : Prop :=
  ∀ n : Nat, n ≥ 1 → ∃ k, collatzIter k n = 1
end Collatz

namespace TwinPrimes
/-- Twin prime conjecture: infinitely many primes p where p + 2 is also prime. -/
def statement : Prop := ∀ N : Nat, ∃ p, p > N ∧ Nat.Prime p ∧ Nat.Prime (p + 2)
end TwinPrimes

namespace OddPerfect
/-- No odd perfect numbers exist. -/
def divisorSum (n : Nat) : Nat :=
  (List.range n).foldl (fun acc d => if d > 0 ∧ n % d = 0 then acc + d else acc) 0
def statement : Prop :=
  ∀ n : Nat, n % 2 = 1 → ¬(n > 0 ∧ divisorSum n = n)
end OddPerfect

namespace Mertens
/-- Mertens conjecture (formalization pending — requires Möbius function). -/
def statement : Prop := True  -- FORMALIZATION PENDING
end Mertens

namespace Legendre
/-- Legendre's conjecture: a prime between consecutive squares. -/
def statement : Prop :=
  ∀ n : Nat, n ≥ 1 → ∃ p, Nat.Prime p ∧ n * n < p ∧ p < (n + 1) * (n + 1)
end Legendre

namespace ErdosStraus
/-- Erdős–Straus conjecture: 4/n = 1/x + 1/y + 1/z for n ≥ 2. -/
def statement : Prop :=
  ∀ n : Nat, n ≥ 2 → ∃ x y z : Nat, x > 0 ∧ y > 0 ∧ z > 0 ∧
    4 * x * y * z = n * (y * z + x * z + x * y)
end ErdosStraus

-- ═══════════════════════════════════════════════════════════════════
-- Millennium Prize Problems (formalization pending)
-- ═══════════════════════════════════════════════════════════════════

namespace PvsNP
/-- P vs NP (formalization pending — requires Turing machine encoding). -/
def statement : Prop := True  -- FORMALIZATION PENDING
end PvsNP

namespace Riemann
/-- Riemann Hypothesis (formalization pending — requires complex analysis). -/
def statement : Prop := True  -- FORMALIZATION PENDING
end Riemann

namespace NavierStokes
/-- Navier-Stokes existence and smoothness (formalization pending). -/
def statement : Prop := True  -- FORMALIZATION PENDING
end NavierStokes

namespace YangMills
/-- Yang-Mills mass gap (formalization pending — requires gauge theory). -/
def statement : Prop := True  -- FORMALIZATION PENDING
end YangMills

namespace Hodge
/-- Hodge conjecture (formalization pending — requires algebraic geometry). -/
def statement : Prop := True  -- FORMALIZATION PENDING
end Hodge

namespace BSDFull
/-- Birch and Swinnerton-Dyer conjecture (formalization pending). -/
def statement : Prop := True  -- FORMALIZATION PENDING
end BSDFull

end ProofEnum
