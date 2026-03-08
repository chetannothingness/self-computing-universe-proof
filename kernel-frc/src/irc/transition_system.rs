// Transition system builders for each problem.
//
// Each problem is modeled as a transition system (S, T, P):
//   S = state space (typically Nat or product)
//   T = transition (typically n → n+1)
//   P = property to prove ∀n, P(n)

use crate::frc_types::TransitionSystem;

/// Build the transition system for a given problem.
pub fn build_transition_system(problem_id: &str) -> TransitionSystem {
    match problem_id {
        "goldbach" => TransitionSystem::new(
            "Nat (even ≥ 4)".to_string(),
            "n → n + 2".to_string(),
            "isSumOfTwoPrimes(n)".to_string(),
            "goldbach".to_string(),
        ),
        "collatz" => TransitionSystem::new(
            "Nat (≥ 1)".to_string(),
            "n → n + 1".to_string(),
            "reachesOne(n)".to_string(),
            "collatz".to_string(),
        ),
        "twin_primes" => TransitionSystem::new(
            "Nat".to_string(),
            "n → n + 1".to_string(),
            "∃p ≤ n, isPrime(p) ∧ isPrime(p+2)".to_string(),
            "twin_primes".to_string(),
        ),
        "flt" => TransitionSystem::new(
            "Nat (exponent ≥ 3)".to_string(),
            "n → n + 1".to_string(),
            "∀a b c > 0, a^n + b^n ≠ c^n".to_string(),
            "flt".to_string(),
        ),
        "odd_perfect" => TransitionSystem::new(
            "Nat (odd)".to_string(),
            "n → n + 2".to_string(),
            "σ(n) ≠ 2n".to_string(),
            "odd_perfect".to_string(),
        ),
        "mersenne" => TransitionSystem::new(
            "Nat (prime index)".to_string(),
            "p → nextPrime(p)".to_string(),
            "∃q ≤ p, isPrime(q) ∧ isPrime(2^q - 1)".to_string(),
            "mersenne".to_string(),
        ),
        "zfc_zero_ne_one" => TransitionSystem::new(
            "Unit".to_string(),
            "id".to_string(),
            "0 ≠ 1".to_string(),
            "zfc_zero_ne_one".to_string(),
        ),
        "mertens" => TransitionSystem::new(
            "Nat (≥ 1)".to_string(),
            "n → n + 1".to_string(),
            "|M(n)| ≤ √n".to_string(),
            "mertens".to_string(),
        ),
        "legendre" => TransitionSystem::new(
            "Nat (≥ 1)".to_string(),
            "n → n + 1".to_string(),
            "∃p, n² < p ≤ (n+1)² ∧ isPrime(p)".to_string(),
            "legendre".to_string(),
        ),
        "erdos_straus" => TransitionSystem::new(
            "Nat (≥ 2)".to_string(),
            "n → n + 1".to_string(),
            "∃x y z, 4/n = 1/x + 1/y + 1/z".to_string(),
            "erdos_straus".to_string(),
        ),
        "bsd_ec" => TransitionSystem::new(
            "Nat (prime p)".to_string(),
            "p → nextPrime(p)".to_string(),
            "|#E(F_p) - (p+1)| ≤ 2√p".to_string(),
            "bsd_ec".to_string(),
        ),
        "weak_goldbach" => TransitionSystem::new(
            "Nat (odd ≥ 7)".to_string(),
            "n → n + 2".to_string(),
            "isSumOfThreePrimes(n)".to_string(),
            "weak_goldbach".to_string(),
        ),
        "bertrand" => TransitionSystem::new(
            "Nat (≥ 1)".to_string(),
            "n → n + 1".to_string(),
            "∃p, n < p ≤ 2n ∧ isPrime(p)".to_string(),
            "bertrand".to_string(),
        ),
        "lagrange" => TransitionSystem::new(
            "Nat".to_string(),
            "n → n + 1".to_string(),
            "∃a b c d, a²+b²+c²+d² = n".to_string(),
            "lagrange".to_string(),
        ),
        // Millennium frontier problems
        "p_vs_np" => TransitionSystem::new(
            "Nat (TM index)".to_string(),
            "n → n + 1".to_string(),
            "separation of complexity classes up to index n".to_string(),
            "p_vs_np".to_string(),
        ),
        "riemann_full" => TransitionSystem::new(
            "Nat (zero index)".to_string(),
            "n → n + 1".to_string(),
            "first n non-trivial zeros of ζ(s) lie on Re(s)=1/2".to_string(),
            "riemann_full".to_string(),
        ),
        "navier_stokes" => TransitionSystem::new(
            "Nat (discretization level)".to_string(),
            "n → n + 1".to_string(),
            "smooth solutions exist at resolution n".to_string(),
            "navier_stokes".to_string(),
        ),
        "yang_mills" => TransitionSystem::new(
            "Nat (lattice size)".to_string(),
            "n → n + 1".to_string(),
            "mass gap ≥ Δ at lattice size n".to_string(),
            "yang_mills".to_string(),
        ),
        "hodge" => TransitionSystem::new(
            "Nat (variety index)".to_string(),
            "n → n + 1".to_string(),
            "Hodge classes are algebraic for varieties up to index n".to_string(),
            "hodge".to_string(),
        ),
        "bsd_full" => TransitionSystem::new(
            "Nat (curve index)".to_string(),
            "n → n + 1".to_string(),
            "rank(E) = ord_{s=1} L(E,s) for curves up to index n".to_string(),
            "bsd_full".to_string(),
        ),
        _ => TransitionSystem::new(
            "Nat".to_string(),
            "n → n + 1".to_string(),
            format!("P(n) for {}", problem_id),
            problem_id.to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_system_all_problems() {
        let problems = [
            "goldbach", "collatz", "twin_primes", "flt", "odd_perfect",
            "mersenne", "zfc_zero_ne_one", "mertens", "legendre", "erdos_straus",
            "bsd_ec", "weak_goldbach", "bertrand", "lagrange",
            "p_vs_np", "riemann_full", "navier_stokes", "yang_mills", "hodge", "bsd_full",
        ];
        for id in &problems {
            let ts = build_transition_system(id);
            assert_eq!(ts.problem_id, *id);
            assert!(!ts.state_desc.is_empty());
            assert!(!ts.transition_desc.is_empty());
            assert!(!ts.property_desc.is_empty());
        }
    }

    #[test]
    fn transition_system_deterministic() {
        let ts1 = build_transition_system("goldbach");
        let ts2 = build_transition_system("goldbach");
        assert_eq!(ts1.ts_hash, ts2.ts_hash);
    }
}
