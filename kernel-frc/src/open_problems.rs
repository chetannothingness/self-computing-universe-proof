// Real VM programs for open mathematical problems — finite computational fragments.
//
// Each function builds a genuine VM program that performs real mathematical
// verification over a bounded range. The programs are deterministic, total,
// and produce truthful results.
//
// What each program proves:
//   Goldbach:       "Every even n in [4, N] is the sum of two primes"
//   Collatz:        "Every n in [1, N] reaches 1 under the 3n+1 map within M iterations"
//   Twin Primes:    "There exists a twin prime pair (p, p+2) with p in [2, N]"
//   FLT:            "No a^n + b^n = c^n for n in [3, E], a,b,c in [1, B]"
//   Odd Perfect:    "No odd perfect number in [1, N]"
//   Mersenne:       "There exists a Mersenne prime 2^p - 1 for prime p in [2, P]"
//   ZFC 0≠1:        "0 ≠ 1" (trivial disproof)
//
// Clay Millennium Prize fragments:
//   Mertens (RH):   "|M(n)| ≤ √n for all n ≤ N" (Riemann Hypothesis fragment)
//   BSD EC Count:    "#E(F_p) satisfies Hasse bound" (BSD fragment)
//
// Other major open problems:
//   Legendre:       "Prime between n² and (n+1)² for all n ≤ N"
//   Erdős–Straus:   "4/n = 1/x + 1/y + 1/z for all n in [2, N]"
//   Weak Goldbach:  "Every odd n > 5 is sum of three primes" (Helfgott 2013)
//
// Classical theorems (finite verification):
//   Bertrand:       "Prime between n and 2n for all n ≤ N" (Chebyshev 1852)
//   Lagrange:       "Every n is sum of four squares" (Lagrange 1770)

use crate::asm::Asm;
use crate::vm::Program;

// ─── Memory slot conventions ───
// Each program documents its memory layout in comments.
// Shared primality test uses slots passed as parameters.

/// Emit an inline primality test for the value in `val_slot`.
/// Uses `div_slot` as scratch for the trial divisor.
/// After execution: stack top = 1 if prime, 0 if not prime.
/// The val_slot value is preserved.
///
/// Algorithm:
///   if val < 2: not prime
///   if val == 2: prime
///   if val % 2 == 0: not prime
///   for d = 3, 5, 7, ... while d*d <= val:
///     if val % d == 0: not prime
///   prime
fn emit_primality_test(asm: &mut Asm, val_slot: usize, div_slot: usize,
                        prime_label: &str, not_prime_label: &str) {
    let check_2 = format!("{}_check2", prime_label);
    let check_even = format!("{}_checkeven", prime_label);
    let trial_loop = format!("{}_trial", prime_label);
    let trial_check = format!("{}_trial_check", prime_label);

    // if val < 2 → not prime
    asm.load(val_slot);
    asm.push(2);
    asm.lt();
    asm.jz(&check_2);
    asm.jmp(not_prime_label);

    // if val == 2 → prime
    asm.label(&check_2);
    asm.load(val_slot);
    asm.push(2);
    asm.eq();
    asm.jz(&check_even);
    asm.jmp(prime_label);

    // if val % 2 == 0 → not prime
    asm.label(&check_even);
    asm.load(val_slot);
    asm.push(2);
    asm.mod_();
    asm.push(0);
    asm.eq();
    asm.jz(&trial_loop);
    asm.jmp(not_prime_label);

    // Trial division: d = 3, step by 2
    asm.label(&trial_loop);
    // First time: initialize d = 3
    // This label is jumped to from check_even (d not yet set) and from trial_check (d already set)
    // We need a separate init. Let's restructure:

    // Actually, let's set d=3 before the loop.
    // Re-do: set d=3, then loop.
    asm.push(3);
    asm.store(div_slot);

    asm.label(&trial_check);
    // if d*d > val → prime
    asm.load(div_slot);
    asm.load(div_slot);
    asm.mul();
    asm.load(val_slot);
    // stack: d*d, val. Need d*d > val, i.e. val < d*d
    asm.swap();
    asm.lt();       // val < d*d?
    asm.jz(&format!("{}_try_div", prime_label));
    asm.jmp(prime_label);

    // val % d == 0 → not prime
    asm.label(&format!("{}_try_div", prime_label));
    asm.load(val_slot);
    asm.load(div_slot);
    asm.mod_();
    asm.push(0);
    asm.eq();
    asm.jz(&format!("{}_inc_d", prime_label));
    asm.jmp(not_prime_label);

    // d += 2, loop
    asm.label(&format!("{}_inc_d", prime_label));
    asm.load(div_slot);
    asm.push(2);
    asm.add();
    asm.store(div_slot);
    asm.jmp(&trial_check);
}

/// Goldbach verification: every even n in [4, n_max] is the sum of two primes.
///
/// Memory layout:
///   slot 0: n (current even number)
///   slot 1: limit (n_max)
///   slot 2: p (candidate prime)
///   slot 3: q (= n - p, checked for primality)
///   slot 10: val for primality test of p
///   slot 11: divisor scratch for p test
///   slot 12: val for primality test of q
///   slot 13: divisor scratch for q test
///
/// Returns: Halt(1) = all verified, Halt(0) = counterexample found.
pub fn build_goldbach(n_max: i64) -> (Program, u64, &'static str) {
    let mut asm = Asm::new();

    // n = 4
    asm.push(4);
    asm.store(0);
    // limit = n_max
    asm.push(n_max);
    asm.store(1);

    // Outer loop: for each even n
    asm.label("outer");
    asm.load(0);
    asm.load(1);
    // n > limit? → done (all verified)
    asm.swap();
    asm.lt();       // limit < n?
    asm.jz("outer_body");
    asm.halt(1);    // all verified

    asm.label("outer_body");
    // p = 2
    asm.push(2);
    asm.store(2);

    // Inner loop: try p from 2 to n/2
    asm.label("inner");
    asm.load(2);
    asm.load(0);
    asm.push(2);
    asm.div();      // n/2
    // p > n/2? → no decomposition found → counterexample
    asm.swap();
    asm.lt();       // n/2 < p?
    asm.jz("inner_body");
    asm.halt(0);    // counterexample: n has no Goldbach decomposition

    asm.label("inner_body");
    // Compute q = n - p
    asm.load(0);
    asm.load(2);
    asm.sub();
    asm.store(3);   // q = n - p

    // Test if p is prime
    asm.load(2);
    asm.store(10);  // val_slot for p test
    emit_primality_test(&mut asm, 10, 11, "p_prime", "p_not_prime");

    asm.label("p_not_prime");
    // p is not prime → try next p
    asm.jmp("next_p");

    asm.label("p_prime");
    // p is prime. Test if q is prime.
    asm.load(3);
    asm.store(12);  // val_slot for q test
    emit_primality_test(&mut asm, 12, 13, "q_prime", "q_not_prime");

    asm.label("q_not_prime");
    // q is not prime → try next p
    asm.jmp("next_p");

    asm.label("q_prime");
    // Both p and q are prime → n = p + q verified. Next n.
    asm.load(0);
    asm.push(2);
    asm.add();
    asm.store(0);   // n += 2
    asm.jmp("outer");

    asm.label("next_p");
    asm.load(2);
    asm.push(1);
    asm.add();
    asm.store(2);   // p += 1
    asm.jmp("inner");

    let instr_count = asm.len() as u64;
    // B* estimate: outer loop runs (n_max-4)/2 ≈ 500 iterations.
    // Inner loop runs up to n/2 per outer. Primality tests ~sqrt(n) each.
    // Conservative: 500 * 500 * 2 * 32 * (instr per primality) ≈ 45M
    let b_star = ((n_max as u64) / 2) * ((n_max as u64) / 2) * 2 * 40 * instr_count / 10 + 1_000_000;

    let prog = asm.build().expect("Goldbach program must assemble");
    (prog, b_star, "Goldbach verified for all even n in [4, N]")
}

/// Collatz verification: every n in [1, n_max] reaches 1 within max_iter steps.
///
/// Memory layout:
///   slot 0: n (starting value being tested)
///   slot 1: limit (n_max)
///   slot 2: current (value in the Collatz sequence)
///   slot 3: iter_count
///   slot 4: max_iter
///
/// Returns: Halt(1) = all converge, Halt(0) = some n did not converge.
pub fn build_collatz(n_max: i64, max_iter: i64) -> (Program, u64, &'static str) {
    let mut asm = Asm::new();

    // n = 1
    asm.push(1);
    asm.store(0);
    // limit = n_max
    asm.push(n_max);
    asm.store(1);
    // max_iter
    asm.push(max_iter);
    asm.store(4);

    // Outer loop: for each starting n
    asm.label("outer");
    asm.load(0);
    asm.load(1);
    asm.swap();
    asm.lt();       // limit < n?
    asm.jz("test_n");
    asm.halt(1);    // all verified

    asm.label("test_n");
    // current = n
    asm.load(0);
    asm.store(2);
    // iter_count = 0
    asm.push(0);
    asm.store(3);

    // Inner loop: iterate Collatz
    asm.label("collatz_step");
    // if current == 1 → converged
    asm.load(2);
    asm.push(1);
    asm.eq();
    asm.jz("check_budget");
    asm.jmp("converged");

    asm.label("check_budget");
    // if iter_count >= max_iter → did not converge
    asm.load(3);
    asm.load(4);
    // iter_count < max_iter?
    asm.lt();
    asm.jz("did_not_converge");

    // current % 2 == 0?
    asm.load(2);
    asm.push(2);
    asm.mod_();
    asm.push(0);
    asm.eq();
    asm.jz("odd_step");

    // Even: current = current / 2
    asm.load(2);
    asm.push(2);
    asm.div();
    asm.store(2);
    asm.jmp("inc_iter");

    asm.label("odd_step");
    // Odd: current = 3 * current + 1
    asm.load(2);
    asm.push(3);
    asm.mul();
    asm.push(1);
    asm.add();
    asm.store(2);

    asm.label("inc_iter");
    asm.load(3);
    asm.push(1);
    asm.add();
    asm.store(3);
    asm.jmp("collatz_step");

    asm.label("converged");
    // Next n
    asm.load(0);
    asm.push(1);
    asm.add();
    asm.store(0);
    asm.jmp("outer");

    asm.label("did_not_converge");
    asm.halt(0);

    let instr_count = asm.len() as u64;
    // B* estimate: n_max outer iterations * max_iter inner * ~10 instructions per step
    let b_star = (n_max as u64) * (max_iter as u64) * instr_count + 1_000_000;

    let prog = asm.build().expect("Collatz program must assemble");
    (prog, b_star, "Collatz verified for all n in [1, N]")
}

/// Twin prime search: find a twin prime pair (p, p+2) with p in [2, n_max].
///
/// Memory layout:
///   slot 0: p (candidate)
///   slot 1: limit (n_max)
///   slot 10: val for primality test of p
///   slot 11: divisor scratch for p test
///   slot 12: val for primality test of p+2
///   slot 13: divisor scratch for p+2 test
///
/// Returns: Halt(1) = found twin primes, Halt(0) = none found in range.
pub fn build_twin_prime_search(n_max: i64) -> (Program, u64, &'static str) {
    let mut asm = Asm::new();

    // p = 2
    asm.push(2);
    asm.store(0);
    // limit = n_max
    asm.push(n_max);
    asm.store(1);

    asm.label("search");
    asm.load(0);
    asm.load(1);
    asm.swap();
    asm.lt();       // limit < p?
    asm.jz("test_p");
    asm.halt(0);    // none found

    asm.label("test_p");
    // Test if p is prime
    asm.load(0);
    asm.store(10);
    emit_primality_test(&mut asm, 10, 11, "tp_p_prime", "tp_p_not_prime");

    asm.label("tp_p_not_prime");
    asm.jmp("next_p");

    asm.label("tp_p_prime");
    // p is prime. Test p+2.
    asm.load(0);
    asm.push(2);
    asm.add();
    asm.store(12);
    emit_primality_test(&mut asm, 12, 13, "tp_q_prime", "tp_q_not_prime");

    asm.label("tp_q_not_prime");
    asm.jmp("next_p");

    asm.label("tp_q_prime");
    // Found twin primes!
    asm.halt(1);

    asm.label("next_p");
    asm.load(0);
    asm.push(1);
    asm.add();
    asm.store(0);
    asm.jmp("search");

    let instr_count = asm.len() as u64;
    // Worst case: scan all p up to n_max, ~sqrt(p) per primality test, 2 tests per p
    let b_star = (n_max as u64) * 2 * 40 * instr_count + 1_000_000;

    let prog = asm.build().expect("Twin prime program must assemble");
    (prog, b_star, "Twin prime pair exists in [2, N]")
}

/// FLT small cases: verify no a^n + b^n = c^n for n in [3, max_exp], a,b,c in [1, max_base].
///
/// Memory layout:
///   slot 0: n (exponent)
///   slot 1: a
///   slot 2: b
///   slot 3: c
///   slot 4: max_exp
///   slot 5: max_base
///   slot 6: a^n (computed)
///   slot 7: b^n (computed)
///   slot 8: c^n (computed)
///   slot 9: scratch for power computation
///   slot 10: power loop counter
///
/// Returns: Halt(1) = no counterexample (FLT verified), Halt(0) = counterexample found.
pub fn build_flt(max_exp: i64, max_base: i64) -> (Program, u64, &'static str) {
    let mut asm = Asm::new();

    asm.push(max_exp);
    asm.store(4);
    asm.push(max_base);
    asm.store(5);

    // n = 3
    asm.push(3);
    asm.store(0);

    asm.label("loop_n");
    asm.load(0);
    asm.load(4);
    asm.swap();
    asm.lt();       // max_exp < n?
    asm.jz("body_n");
    asm.halt(1);    // all verified

    asm.label("body_n");
    // a = 1
    asm.push(1);
    asm.store(1);

    asm.label("loop_a");
    asm.load(1);
    asm.load(5);
    asm.swap();
    asm.lt();       // max_base < a?
    asm.jz("body_a");
    asm.jmp("next_n");

    asm.label("body_a");
    // Compute a^n → slot 6
    // result = 1, counter = n
    asm.push(1);
    asm.store(6);
    asm.load(0);
    asm.store(10);
    asm.label("pow_a");
    asm.load(10);
    asm.push(0);
    asm.eq();
    asm.jz("pow_a_step");
    asm.jmp("pow_a_done");
    asm.label("pow_a_step");
    asm.load(6);
    asm.load(1);
    asm.mul();
    asm.store(6);
    asm.load(10);
    asm.push(1);
    asm.sub();
    asm.store(10);
    asm.jmp("pow_a");
    asm.label("pow_a_done");

    // b = 1
    asm.push(1);
    asm.store(2);

    asm.label("loop_b");
    asm.load(2);
    asm.load(5);
    asm.swap();
    asm.lt();       // max_base < b?
    asm.jz("body_b");
    asm.jmp("next_a");

    asm.label("body_b");
    // Compute b^n → slot 7
    asm.push(1);
    asm.store(7);
    asm.load(0);
    asm.store(10);
    asm.label("pow_b");
    asm.load(10);
    asm.push(0);
    asm.eq();
    asm.jz("pow_b_step");
    asm.jmp("pow_b_done");
    asm.label("pow_b_step");
    asm.load(7);
    asm.load(2);
    asm.mul();
    asm.store(7);
    asm.load(10);
    asm.push(1);
    asm.sub();
    asm.store(10);
    asm.jmp("pow_b");
    asm.label("pow_b_done");

    // c = 1
    asm.push(1);
    asm.store(3);

    asm.label("loop_c");
    asm.load(3);
    asm.load(5);
    asm.swap();
    asm.lt();       // max_base < c?
    asm.jz("body_c");
    asm.jmp("next_b");

    asm.label("body_c");
    // Compute c^n → slot 8
    asm.push(1);
    asm.store(8);
    asm.load(0);
    asm.store(10);
    asm.label("pow_c");
    asm.load(10);
    asm.push(0);
    asm.eq();
    asm.jz("pow_c_step");
    asm.jmp("pow_c_done");
    asm.label("pow_c_step");
    asm.load(8);
    asm.load(3);
    asm.mul();
    asm.store(8);
    asm.load(10);
    asm.push(1);
    asm.sub();
    asm.store(10);
    asm.jmp("pow_c");
    asm.label("pow_c_done");

    // Check: a^n + b^n == c^n?
    asm.load(6);
    asm.load(7);
    asm.add();
    asm.load(8);
    asm.eq();
    asm.jz("next_c");
    asm.halt(0);    // counterexample found!

    asm.label("next_c");
    asm.load(3);
    asm.push(1);
    asm.add();
    asm.store(3);
    asm.jmp("loop_c");

    asm.label("next_b");
    asm.load(2);
    asm.push(1);
    asm.add();
    asm.store(2);
    asm.jmp("loop_b");

    asm.label("next_a");
    asm.load(1);
    asm.push(1);
    asm.add();
    asm.store(1);
    asm.jmp("loop_a");

    asm.label("next_n");
    asm.load(0);
    asm.push(1);
    asm.add();
    asm.store(0);
    asm.jmp("loop_n");

    let instr_count = asm.len() as u64;
    // 4-nested loop: (max_exp-2) * max_base^3 * ~10 instructions per power computation * n
    let exp_range = (max_exp - 2) as u64;
    let base_cubed = (max_base as u64).pow(3);
    let b_star = exp_range * base_cubed * (max_exp as u64) * instr_count + 1_000_000;

    let prog = asm.build().expect("FLT program must assemble");
    (prog, b_star, "FLT verified for small cases")
}

/// Odd perfect number search: verify no odd perfect number in [1, n_max].
///
/// A perfect number n has sigma(n) = 2n where sigma is the sum of divisors.
///
/// Memory layout:
///   slot 0: n (current odd number)
///   slot 1: limit (n_max)
///   slot 2: d (divisor being tested)
///   slot 3: sigma (sum of divisors)
///
/// Returns: Halt(1) = none found, Halt(0) = found one.
pub fn build_odd_perfect(n_max: i64) -> (Program, u64, &'static str) {
    let mut asm = Asm::new();

    // n = 1
    asm.push(1);
    asm.store(0);
    asm.push(n_max);
    asm.store(1);

    asm.label("outer");
    asm.load(0);
    asm.load(1);
    asm.swap();
    asm.lt();       // limit < n?
    asm.jz("test_n");
    asm.halt(1);    // none found

    asm.label("test_n");
    // sigma = 0, d = 1
    asm.push(0);
    asm.store(3);
    asm.push(1);
    asm.store(2);

    asm.label("div_loop");
    asm.load(2);
    asm.load(0);
    // d > n? (n < d?)
    asm.swap();
    asm.lt();       // n < d?
    asm.jz("div_check");
    asm.jmp("check_perfect");

    asm.label("div_check");
    // if n % d == 0, add d to sigma
    asm.load(0);
    asm.load(2);
    asm.mod_();
    asm.push(0);
    asm.eq();
    asm.jz("div_next");
    // n % d == 0 → sigma += d
    asm.load(3);
    asm.load(2);
    asm.add();
    asm.store(3);

    asm.label("div_next");
    asm.load(2);
    asm.push(1);
    asm.add();
    asm.store(2);
    asm.jmp("div_loop");

    asm.label("check_perfect");
    // sigma == 2*n?
    asm.load(3);
    asm.load(0);
    asm.push(2);
    asm.mul();
    asm.eq();
    asm.jz("next_odd");
    asm.halt(0);    // found an odd perfect number!

    asm.label("next_odd");
    // n += 2 (only check odd numbers)
    asm.load(0);
    asm.push(2);
    asm.add();
    asm.store(0);
    asm.jmp("outer");

    let instr_count = asm.len() as u64;
    // Outer: n_max/2 odd numbers. Inner: up to n divisors per number.
    // Conservative: (n_max/2) * n_max * ~10 instructions
    let b_star = ((n_max as u64) / 2) * (n_max as u64) * instr_count + 1_000_000;

    let prog = asm.build().expect("Odd perfect program must assemble");
    (prog, b_star, "No odd perfect number in [1, N]")
}

/// Mersenne prime search: find a Mersenne prime 2^p - 1 for prime p in [2, p_max].
///
/// Memory layout:
///   slot 0: p (candidate exponent)
///   slot 1: limit (p_max)
///   slot 2: 2^p - 1 (Mersenne candidate)
///   slot 10: val for primality test of p
///   slot 11: divisor scratch for p test
///   slot 12: val for primality test of Mersenne candidate
///   slot 13: divisor scratch for Mersenne test
///   slot 14: power computation scratch
///
/// Returns: Halt(1) = found Mersenne prime, Halt(0) = none found.
pub fn build_mersenne(p_max: i64) -> (Program, u64, &'static str) {
    let mut asm = Asm::new();

    asm.push(2);
    asm.store(0);
    asm.push(p_max);
    asm.store(1);

    asm.label("search");
    asm.load(0);
    asm.load(1);
    asm.swap();
    asm.lt();       // limit < p?
    asm.jz("test_p");
    asm.halt(0);    // none found

    asm.label("test_p");
    // Check if p is prime
    asm.load(0);
    asm.store(10);
    emit_primality_test(&mut asm, 10, 11, "mp_p_prime", "mp_p_not_prime");

    asm.label("mp_p_not_prime");
    asm.jmp("next_p");

    asm.label("mp_p_prime");
    // p is prime. Compute 2^p - 1.
    // result = 1, counter = p
    asm.push(1);
    asm.store(2);
    asm.load(0);
    asm.store(14);
    asm.label("pow2_loop");
    asm.load(14);
    asm.push(0);
    asm.eq();
    asm.jz("pow2_step");
    asm.jmp("pow2_done");
    asm.label("pow2_step");
    asm.load(2);
    asm.push(2);
    asm.mul();
    asm.store(2);
    asm.load(14);
    asm.push(1);
    asm.sub();
    asm.store(14);
    asm.jmp("pow2_loop");
    asm.label("pow2_done");
    // 2^p is in slot 2, subtract 1
    asm.load(2);
    asm.push(1);
    asm.sub();
    asm.store(2);

    // Check if 2^p - 1 is prime
    asm.load(2);
    asm.store(12);
    emit_primality_test(&mut asm, 12, 13, "mp_mersenne_prime", "mp_mersenne_not_prime");

    asm.label("mp_mersenne_not_prime");
    asm.jmp("next_p");

    asm.label("mp_mersenne_prime");
    asm.halt(1);    // found a Mersenne prime!

    asm.label("next_p");
    asm.load(0);
    asm.push(1);
    asm.add();
    asm.store(0);
    asm.jmp("search");

    let instr_count = asm.len() as u64;
    // Worst case: p_max candidates, primality test ~sqrt(2^p) each, power computation ~p
    let b_star = (p_max as u64) * (p_max as u64) * 40 * instr_count + 1_000_000;

    let prog = asm.build().expect("Mersenne program must assemble");
    (prog, b_star, "Mersenne prime exists for p in [2, P]")
}

/// Mertens function verification (Riemann Hypothesis finite fragment).
///
/// Computes M(n) = Σ_{k=1}^{n} μ(k) where μ is the Möbius function.
/// Verifies |M(n)| ≤ √n (i.e., M(n)² ≤ n) for all n in [1, n_max].
///
/// The bound M(x) = O(x^{1/2+ε}) is equivalent to the Riemann Hypothesis.
/// This program verifies the finite fragment: M(n)² ≤ n for all n ≤ N.
///
/// Memory layout:
///   slot 0: n (current number, 1 to n_max)
///   slot 1: n_max
///   slot 2: mertens_sum M(n)
///   slot 3: remaining (n being factored)
///   slot 4: d (trial divisor)
///   slot 5: factor_count
///   slot 6: has_square flag
///
/// Returns: Halt(1) = bound verified for all n, Halt(0) = violation found.
pub fn build_mertens(n_max: i64) -> (Program, u64, &'static str) {
    let mut asm = Asm::new();

    asm.push(n_max);
    asm.store(1);
    // Handle n=1: μ(1)=1, M(1)=1
    asm.push(1);
    asm.store(2);   // M = 1
    asm.push(2);
    asm.store(0);   // start from n=2

    // Main outer loop
    asm.label("mert_outer");
    asm.load(0);
    asm.load(1);
    asm.swap();
    asm.lt();       // n_max < n?
    asm.jz("mert_factor_start");
    asm.halt(1);    // all verified

    asm.label("mert_factor_start");
    // Initialize factoring for current n
    asm.load(0);
    asm.store(3);   // remaining = n
    asm.push(0);
    asm.store(5);   // factor_count = 0
    asm.push(0);
    asm.store(6);   // has_square = 0
    asm.push(2);
    asm.store(4);   // d = 2

    asm.label("mert_floop");
    // Skip if has_square already set
    asm.load(6);
    asm.push(1);
    asm.eq();
    asm.jz("mert_check_dd");
    asm.jmp("mert_mu_zero");

    asm.label("mert_check_dd");
    // if d*d > remaining: done factoring
    asm.load(4);
    asm.dup();
    asm.mul();      // d*d
    asm.load(3);    // remaining
    asm.swap();
    asm.lt();       // remaining < d*d?
    asm.jz("mert_try_div");
    asm.jmp("mert_fdone");

    asm.label("mert_try_div");
    // if remaining % d == 0
    asm.load(3);
    asm.load(4);
    asm.mod_();
    asm.push(0);
    asm.eq();
    asm.jz("mert_inc_d");

    // d divides remaining: divide once
    asm.load(3);
    asm.load(4);
    asm.div();
    asm.store(3);   // remaining /= d
    asm.load(5);
    asm.push(1);
    asm.add();
    asm.store(5);   // factor_count += 1

    // Check if d still divides (squared factor)
    asm.load(3);
    asm.load(4);
    asm.mod_();
    asm.push(0);
    asm.eq();
    asm.jz("mert_inc_d");
    // Squared factor found
    asm.push(1);
    asm.store(6);
    asm.jmp("mert_mu_zero");

    asm.label("mert_inc_d");
    asm.load(4);
    asm.push(1);
    asm.add();
    asm.store(4);   // d += 1
    asm.jmp("mert_floop");

    asm.label("mert_fdone");
    // If remaining > 1: one more prime factor
    asm.push(1);
    asm.load(3);
    asm.lt();       // 1 < remaining?
    asm.jz("mert_compute_mu");
    asm.load(5);
    asm.push(1);
    asm.add();
    asm.store(5);   // factor_count += 1

    asm.label("mert_compute_mu");
    // Check has_square (defensive)
    asm.load(6);
    asm.push(1);
    asm.eq();
    asm.jz("mert_mu_real");
    asm.jmp("mert_mu_zero");

    asm.label("mert_mu_real");
    // μ = (-1)^factor_count: even → +1, odd → -1
    asm.load(5);
    asm.push(2);
    asm.mod_();
    asm.push(0);
    asm.eq();
    asm.jz("mert_mu_neg");
    // Even: M += 1
    asm.load(2);
    asm.push(1);
    asm.add();
    asm.store(2);
    asm.jmp("mert_check");

    asm.label("mert_mu_neg");
    // Odd: M -= 1
    asm.load(2);
    asm.push(1);
    asm.sub();
    asm.store(2);
    asm.jmp("mert_check");

    asm.label("mert_mu_zero");
    // μ = 0: M unchanged, fall through

    asm.label("mert_check");
    // Verify M² ≤ n (equivalently: NOT(n < M²))
    asm.load(0);    // n
    asm.load(2);    // M
    asm.dup();
    asm.mul();      // M²
    // stack: n, M²
    asm.lt();       // n < M²?
    asm.jz("mert_next");
    asm.halt(0);    // Mertens bound violated!

    asm.label("mert_next");
    asm.load(0);
    asm.push(1);
    asm.add();
    asm.store(0);   // n += 1
    asm.jmp("mert_outer");

    let instr_count = asm.len() as u64;
    // B*: n_max iterations, each ~√n trial divisions, ~15 instr per step
    let mut sqrt_n: u64 = 1;
    while sqrt_n * sqrt_n <= n_max as u64 { sqrt_n += 1; }
    let b_star = (n_max as u64) * sqrt_n * 20 * instr_count / 10 + 1_000_000;

    let prog = asm.build().expect("Mertens program must assemble");
    (prog, b_star, "Mertens |M(n)| ≤ √n verified for all n ≤ N (Riemann Hypothesis fragment)")
}

/// Legendre's conjecture verification: prime between n² and (n+1)² for all n.
///
/// Legendre's conjecture (open): for every positive integer n, there exists
/// a prime p such that n² < p < (n+1)². Verified computationally up to ~10^9.
///
/// Memory layout:
///   slot 0: n (1 to n_max)
///   slot 1: n_max
///   slot 2: lo (n² + 1)
///   slot 3: hi ((n+1)² - 1 = n² + 2n)
///   slot 4: candidate
///   slot 10: val for primality test
///   slot 11: divisor scratch
///
/// Returns: Halt(1) = verified for all n, Halt(0) = counterexample found.
pub fn build_legendre(n_max: i64) -> (Program, u64, &'static str) {
    let mut asm = Asm::new();

    asm.push(n_max);
    asm.store(1);
    asm.push(1);
    asm.store(0);   // n = 1

    asm.label("leg_outer");
    asm.load(0);
    asm.load(1);
    asm.swap();
    asm.lt();       // n_max < n?
    asm.jz("leg_body");
    asm.halt(1);

    asm.label("leg_body");
    // lo = n*n + 1
    asm.load(0);
    asm.dup();
    asm.mul();
    asm.push(1);
    asm.add();
    asm.store(2);

    // hi = n*n + 2*n
    asm.load(0);
    asm.dup();
    asm.mul();
    asm.load(0);
    asm.push(2);
    asm.mul();
    asm.add();
    asm.store(3);

    // candidate = lo
    asm.load(2);
    asm.store(4);

    asm.label("leg_search");
    // candidate > hi?
    asm.load(4);
    asm.load(3);
    asm.swap();
    asm.lt();       // hi < candidate?
    asm.jz("leg_test");
    asm.halt(0);    // no prime found between n² and (n+1)²

    asm.label("leg_test");
    asm.load(4);
    asm.store(10);
    emit_primality_test(&mut asm, 10, 11, "leg_found", "leg_not_prime");

    asm.label("leg_not_prime");
    asm.load(4);
    asm.push(1);
    asm.add();
    asm.store(4);
    asm.jmp("leg_search");

    asm.label("leg_found");
    // Found a prime, next n
    asm.load(0);
    asm.push(1);
    asm.add();
    asm.store(0);
    asm.jmp("leg_outer");

    let instr_count = asm.len() as u64;
    // B*: n_max outer * ~2n inner search * ~√(n²)=n primality per candidate
    let b_star = (n_max as u64) * (n_max as u64) * 2 * 40 * instr_count / 10 + 1_000_000;

    let prog = asm.build().expect("Legendre program must assemble");
    (prog, b_star, "Legendre: prime between n² and (n+1)² for all n ≤ N")
}

/// Erdős–Straus conjecture verification: 4/n = 1/x + 1/y + 1/z for all n ≥ 2.
///
/// For each n, finds positive integers x, y, z such that 4/n = 1/x + 1/y + 1/z.
/// Equivalent: find x with 4x > n, then solve 1/y + 1/z = (4x-n)/(nx).
///
/// Let A = 4x-n, B = nx. Then y ranges over [⌈B/A⌉, ⌊2B/A⌋]
/// and z = By/(Ay-B) must be a positive integer.
///
/// Memory layout:
///   slot 0: n (2 to n_max)
///   slot 1: n_max
///   slot 2: x
///   slot 3: A (= 4x - n)
///   slot 4: B (= n*x)
///   slot 5: y
///   slot 6: y_hi (= 2*B/A)
///   slot 7: z_den (= A*y - B)
///   slot 8: z_num (= B*y)
///
/// Returns: Halt(1) = decomposition found for all n, Halt(0) = counterexample.
pub fn build_erdos_straus(n_max: i64) -> (Program, u64, &'static str) {
    let mut asm = Asm::new();

    asm.push(n_max);
    asm.store(1);
    asm.push(2);
    asm.store(0);   // n = 2

    asm.label("es_outer");
    asm.load(0);
    asm.load(1);
    asm.swap();
    asm.lt();       // n_max < n?
    asm.jz("es_body");
    asm.halt(1);    // all verified

    asm.label("es_body");
    // x starts at 1
    asm.push(1);
    asm.store(2);

    asm.label("es_x_loop");
    // x > n → no decomposition found
    asm.load(2);
    asm.load(0);
    asm.swap();
    asm.lt();       // n < x?
    asm.jz("es_x_body");
    asm.halt(0);    // counterexample

    asm.label("es_x_body");
    // A = 4*x - n
    asm.load(2);
    asm.push(4);
    asm.mul();
    asm.load(0);
    asm.sub();
    asm.store(3);   // A = 4x - n

    // If A <= 0: next x
    asm.load(3);
    asm.push(1);
    asm.lt();       // A < 1?
    asm.jz("es_compute_b");
    asm.jmp("es_next_x");

    asm.label("es_compute_b");
    // B = n * x
    asm.load(0);
    asm.load(2);
    asm.mul();
    asm.store(4);   // B = n*x

    // y_lo = ceil(B/A) = (B + A - 1) / A
    asm.load(4);
    asm.load(3);
    asm.add();
    asm.push(1);
    asm.sub();      // B + A - 1
    asm.load(3);
    asm.div();
    asm.store(5);   // y = y_lo = ceil(B/A)

    // y_hi = 2*B/A
    asm.load(4);
    asm.push(2);
    asm.mul();
    asm.load(3);
    asm.div();
    asm.store(6);   // y_hi = floor(2B/A)

    asm.label("es_y_loop");
    // y > y_hi?
    asm.load(5);
    asm.load(6);
    asm.swap();
    asm.lt();       // y_hi < y?
    asm.jz("es_y_body");
    asm.jmp("es_next_x");

    asm.label("es_y_body");
    // z_den = A*y - B
    asm.load(3);
    asm.load(5);
    asm.mul();
    asm.load(4);
    asm.sub();
    asm.store(7);   // z_den = A*y - B

    // If z_den <= 0: next y
    asm.load(7);
    asm.push(1);
    asm.lt();       // z_den < 1?
    asm.jz("es_check_z");
    asm.jmp("es_next_y");

    asm.label("es_check_z");
    // z_num = B*y
    asm.load(4);
    asm.load(5);
    asm.mul();
    asm.store(8);   // z_num = B*y

    // If z_num % z_den == 0: found!
    asm.load(8);
    asm.load(7);
    asm.mod_();
    asm.push(0);
    asm.eq();
    asm.jz("es_next_y");
    asm.jmp("es_found");

    asm.label("es_next_y");
    asm.load(5);
    asm.push(1);
    asm.add();
    asm.store(5);
    asm.jmp("es_y_loop");

    asm.label("es_next_x");
    asm.load(2);
    asm.push(1);
    asm.add();
    asm.store(2);
    asm.jmp("es_x_loop");

    asm.label("es_found");
    // Decomposition found for this n, next n
    asm.load(0);
    asm.push(1);
    asm.add();
    asm.store(0);
    asm.jmp("es_outer");

    let instr_count = asm.len() as u64;
    // B*: n_max outer * n inner x * small y range * ~10 instr
    let b_star = (n_max as u64) * (n_max as u64) * 10 * instr_count + 1_000_000;

    let prog = asm.build().expect("Erdos-Straus program must assemble");
    (prog, b_star, "Erdős–Straus: 4/n = 1/x + 1/y + 1/z for all n in [2, N]")
}

/// BSD finite fragment: elliptic curve point counting over F_p with Hasse bound.
///
/// Counts points on y² = x³ + ax + b over F_p (including point at infinity)
/// and verifies the Hasse bound: |#E(F_p) - (p+1)| ≤ 2√p.
///
/// Curve selection via curve_id:
///   0: y² = x³ - x   (a=-1, b=0)  — conductor 32, rank 0
///   1: y² = x³ + 1    (a=0, b=1)   — conductor 36, rank 0
///   2: y² = x³ - x + 1 (a=-1, b=1) — conductor 37, rank 1
///
/// Memory layout:
///   slot 0: x (0 to p-1)
///   slot 1: p
///   slot 2: a (curve parameter)
///   slot 3: b (curve parameter)
///   slot 4: count
///   slot 5: rhs (x³+ax+b mod p)
///   slot 6: y (0 to p-1)
///   slot 7: temp
///
/// Returns: Halt(1) = count computed, Hasse bound verified.
pub fn build_bsd_ec_count(p: i64, curve_id: i64) -> (Program, u64, &'static str) {
    let (a_val, b_val) = match curve_id {
        1 => (0i64, 1i64),
        2 => (-1i64, 1i64),
        _ => (-1i64, 0i64),   // default: y² = x³ - x
    };

    let mut asm = Asm::new();

    asm.push(p);
    asm.store(1);
    asm.push(a_val);
    asm.store(2);
    asm.push(b_val);
    asm.store(3);
    asm.push(1);    // start with 1 for point at infinity
    asm.store(4);
    asm.push(0);
    asm.store(0);   // x = 0

    asm.label("ec_x_loop");
    asm.load(0);
    asm.load(1);
    asm.lt();       // x < p?
    asm.jz("ec_hasse");

    // Compute rhs = (x³ + a*x + b) mod p
    // x³
    asm.load(0);
    asm.load(0);
    asm.mul();
    asm.load(0);
    asm.mul();      // x³
    // + a*x
    asm.load(2);
    asm.load(0);
    asm.mul();
    asm.add();      // x³ + ax
    // + b
    asm.load(3);
    asm.add();      // x³ + ax + b
    // mod p
    asm.load(1);
    asm.mod_();
    // Fix negative mod
    asm.dup();
    asm.push(0);
    asm.lt();
    asm.jz("ec_rhs_ok");
    asm.load(1);
    asm.add();
    asm.label("ec_rhs_ok");
    asm.store(5);   // rhs

    // Count y values where y² ≡ rhs (mod p)
    asm.push(0);
    asm.store(6);   // y = 0

    asm.label("ec_y_loop");
    asm.load(6);
    asm.load(1);
    asm.lt();       // y < p?
    asm.jz("ec_next_x");

    // y² mod p
    asm.load(6);
    asm.load(6);
    asm.mul();
    asm.load(1);
    asm.mod_();
    // Fix negative mod
    asm.dup();
    asm.push(0);
    asm.lt();
    asm.jz("ec_y2_ok");
    asm.load(1);
    asm.add();
    asm.label("ec_y2_ok");
    // Compare to rhs
    asm.load(5);
    asm.eq();
    asm.jz("ec_y_inc");
    // Match! count += 1
    asm.load(4);
    asm.push(1);
    asm.add();
    asm.store(4);

    asm.label("ec_y_inc");
    asm.load(6);
    asm.push(1);
    asm.add();
    asm.store(6);
    asm.jmp("ec_y_loop");

    asm.label("ec_next_x");
    asm.load(0);
    asm.push(1);
    asm.add();
    asm.store(0);
    asm.jmp("ec_x_loop");

    asm.label("ec_hasse");
    // Verify Hasse bound: (count - (p+1))² ≤ 4p
    asm.load(4);    // count
    asm.load(1);    // p
    asm.sub();
    asm.push(1);
    asm.sub();      // count - p - 1
    asm.dup();
    asm.mul();      // (count-p-1)²
    asm.store(7);   // save diff²
    // Compute 4p
    asm.load(1);
    asm.push(4);
    asm.mul();      // 4p
    // stack: 4p. Load diff².
    asm.load(7);    // diff²
    // stack: 4p, diff²
    // Lt: pop b=diff², pop a=4p, push (4p < diff²) ? 1 : 0
    asm.lt();       // 4p < diff²?
    asm.jz("ec_ok");
    asm.halt(0);    // Hasse bound violated

    asm.label("ec_ok");
    asm.halt(1);    // Point count verified with Hasse bound

    let instr_count = asm.len() as u64;
    let b_star = (p as u64) * (p as u64) * instr_count + 1_000_000;

    let prog = asm.build().expect("EC point count program must assemble");
    (prog, b_star, "BSD: elliptic curve point count over F_p with Hasse bound")
}

/// Weak Goldbach verification (Helfgott 2013, proved):
/// every odd integer n > 5 is the sum of three primes.
///
/// Memory layout:
///   slot 0: n (current odd number, 7 to n_max)
///   slot 1: n_max
///   slot 2: p1 (first prime candidate)
///   slot 3: p2 (second prime candidate)
///   slot 4: p3 (= n - p1 - p2)
///   slot 10-11: primality test for p1
///   slot 12-13: primality test for p2
///   slot 14-15: primality test for p3
///
/// Returns: Halt(1) = verified, Halt(0) = counterexample.
pub fn build_weak_goldbach(n_max: i64) -> (Program, u64, &'static str) {
    let mut asm = Asm::new();

    asm.push(n_max);
    asm.store(1);
    asm.push(7);
    asm.store(0);   // n = 7 (first odd > 5)

    asm.label("wg_outer");
    asm.load(0);
    asm.load(1);
    asm.swap();
    asm.lt();       // n_max < n?
    asm.jz("wg_body");
    asm.halt(1);

    asm.label("wg_body");
    // p1 = 2
    asm.push(2);
    asm.store(2);

    asm.label("wg_p1_loop");
    // p1 > n/3? → give up (should not happen for odd n > 5)
    asm.load(2);
    asm.load(0);
    asm.push(3);
    asm.div();
    asm.swap();
    asm.lt();       // n/3 < p1?
    asm.jz("wg_p1_test");
    asm.halt(0);    // no decomposition found

    asm.label("wg_p1_test");
    asm.load(2);
    asm.store(10);
    emit_primality_test(&mut asm, 10, 11, "wg_p1_prime", "wg_p1_next");

    asm.label("wg_p1_next");
    asm.load(2);
    asm.push(1);
    asm.add();
    asm.store(2);
    asm.jmp("wg_p1_loop");

    asm.label("wg_p1_prime");
    // p1 is prime, try p2
    asm.load(2);
    asm.store(3);   // p2 = p1

    asm.label("wg_p2_loop");
    // p3 = n - p1 - p2
    asm.load(0);
    asm.load(2);
    asm.sub();
    asm.load(3);
    asm.sub();
    asm.store(4);   // p3 = n - p1 - p2

    // p3 < p2? → next p1
    asm.load(4);
    asm.load(3);
    asm.lt();       // p3 < p2?
    asm.jz("wg_p2_test");
    asm.jmp("wg_p1_next");

    asm.label("wg_p2_test");
    // p3 < 2? → next p2
    asm.load(4);
    asm.push(2);
    asm.lt();       // p3 < 2?
    asm.jz("wg_check_p2");
    asm.jmp("wg_p2_next");

    asm.label("wg_check_p2");
    asm.load(3);
    asm.store(12);
    emit_primality_test(&mut asm, 12, 13, "wg_p2_prime", "wg_p2_next");

    asm.label("wg_p2_next");
    asm.load(3);
    asm.push(1);
    asm.add();
    asm.store(3);
    asm.jmp("wg_p2_loop");

    asm.label("wg_p2_prime");
    // p2 is prime. Check p3.
    asm.load(4);
    asm.store(14);
    emit_primality_test(&mut asm, 14, 15, "wg_found", "wg_p2_next");

    asm.label("wg_found");
    // n = p1 + p2 + p3 verified! Next odd n.
    asm.load(0);
    asm.push(2);
    asm.add();
    asm.store(0);   // n += 2 (next odd)
    asm.jmp("wg_outer");

    let instr_count = asm.len() as u64;
    // B*: n/2 outer * n/3 p1 * n/3 p2 * primality
    let n = n_max as u64;
    let b_star = (n / 2) * (n / 3 + 1) * (n / 3 + 1) * 3 * 40 * instr_count / 100 + 1_000_000;

    let prog = asm.build().expect("Weak Goldbach program must assemble");
    (prog, b_star, "Weak Goldbach: every odd n > 5 is sum of three primes (Helfgott)")
}

/// Bertrand's postulate verification (Chebyshev, proved 1852):
/// for every n ≥ 1, there exists a prime p with n < p ≤ 2n.
///
/// Memory layout:
///   slot 0: n (1 to n_max)
///   slot 1: n_max
///   slot 2: candidate (n+1 to 2n)
///   slot 3: hi (= 2*n)
///   slot 10: val for primality test
///   slot 11: divisor scratch
///
/// Returns: Halt(1) = verified, Halt(0) = counterexample.
pub fn build_bertrand(n_max: i64) -> (Program, u64, &'static str) {
    let mut asm = Asm::new();

    asm.push(n_max);
    asm.store(1);
    asm.push(1);
    asm.store(0);   // n = 1

    asm.label("bert_outer");
    asm.load(0);
    asm.load(1);
    asm.swap();
    asm.lt();       // n_max < n?
    asm.jz("bert_body");
    asm.halt(1);

    asm.label("bert_body");
    // candidate = n + 1
    asm.load(0);
    asm.push(1);
    asm.add();
    asm.store(2);

    // hi = 2 * n
    asm.load(0);
    asm.push(2);
    asm.mul();
    asm.store(3);

    asm.label("bert_search");
    // candidate > hi?
    asm.load(2);
    asm.load(3);
    asm.swap();
    asm.lt();       // hi < candidate?
    asm.jz("bert_test");
    asm.halt(0);    // no prime between n and 2n

    asm.label("bert_test");
    asm.load(2);
    asm.store(10);
    emit_primality_test(&mut asm, 10, 11, "bert_found", "bert_next");

    asm.label("bert_next");
    asm.load(2);
    asm.push(1);
    asm.add();
    asm.store(2);
    asm.jmp("bert_search");

    asm.label("bert_found");
    asm.load(0);
    asm.push(1);
    asm.add();
    asm.store(0);
    asm.jmp("bert_outer");

    let instr_count = asm.len() as u64;
    let b_star = (n_max as u64) * (n_max as u64) * 40 * instr_count / 10 + 1_000_000;

    let prog = asm.build().expect("Bertrand program must assemble");
    (prog, b_star, "Bertrand: prime between n and 2n for all n ≤ N (Chebyshev)")
}

/// Lagrange's four-square theorem verification (proved 1770):
/// every positive integer n is the sum of four squares: n = a² + b² + c² + d².
///
/// Memory layout:
///   slot 0: n (1 to n_max)
///   slot 1: n_max
///   slot 2: a
///   slot 3: b
///   slot 4: c
///   slot 5: remainder (n - a² - b² - c²)
///   slot 6: d (candidate for √remainder)
///
/// Returns: Halt(1) = verified, Halt(0) = counterexample.
pub fn build_lagrange_four_squares(n_max: i64) -> (Program, u64, &'static str) {
    let mut asm = Asm::new();

    asm.push(n_max);
    asm.store(1);
    asm.push(1);
    asm.store(0);   // n = 1

    asm.label("lfs_outer");
    asm.load(0);
    asm.load(1);
    asm.swap();
    asm.lt();       // n_max < n?
    asm.jz("lfs_body");
    asm.halt(1);

    asm.label("lfs_body");
    // a = 0
    asm.push(0);
    asm.store(2);

    asm.label("lfs_a_loop");
    // a² > n? → counterexample
    asm.load(2);
    asm.dup();
    asm.mul();      // a²
    asm.load(0);
    asm.swap();
    asm.lt();       // n < a²?
    asm.jz("lfs_a_body");
    asm.halt(0);    // no representation found

    asm.label("lfs_a_body");
    // b = 0
    asm.push(0);
    asm.store(3);

    asm.label("lfs_b_loop");
    // a² + b² > n?
    asm.load(2);
    asm.dup();
    asm.mul();
    asm.load(3);
    asm.dup();
    asm.mul();
    asm.add();      // a² + b²
    asm.load(0);
    asm.swap();
    asm.lt();       // n < a²+b²?
    asm.jz("lfs_b_body");
    asm.jmp("lfs_next_a");

    asm.label("lfs_b_body");
    // c = 0
    asm.push(0);
    asm.store(4);

    asm.label("lfs_c_loop");
    // remainder = n - a² - b² - c²
    asm.load(0);
    asm.load(2);
    asm.dup();
    asm.mul();
    asm.sub();      // n - a²
    asm.load(3);
    asm.dup();
    asm.mul();
    asm.sub();      // n - a² - b²
    asm.load(4);
    asm.dup();
    asm.mul();
    asm.sub();      // n - a² - b² - c²
    asm.store(5);   // remainder

    // remainder < 0?
    asm.load(5);
    asm.push(0);
    asm.lt();
    asm.jz("lfs_check_sq");
    asm.jmp("lfs_next_b");

    asm.label("lfs_check_sq");
    // Check if remainder is a perfect square
    // Find d such that d² = remainder
    asm.push(0);
    asm.store(6);   // d = 0

    asm.label("lfs_d_loop");
    asm.load(6);
    asm.dup();
    asm.mul();      // d²
    asm.load(5);    // remainder
    asm.eq();       // d² == remainder?
    asm.jz("lfs_d_check_over");
    asm.jmp("lfs_found");  // Found! n = a²+b²+c²+d²

    asm.label("lfs_d_check_over");
    // d² > remainder?
    asm.load(6);
    asm.dup();
    asm.mul();
    asm.load(5);
    asm.swap();
    asm.lt();       // remainder < d²?
    asm.jz("lfs_d_inc");
    asm.jmp("lfs_next_c");  // d² > remainder, no perfect square

    asm.label("lfs_d_inc");
    asm.load(6);
    asm.push(1);
    asm.add();
    asm.store(6);
    asm.jmp("lfs_d_loop");

    asm.label("lfs_next_c");
    asm.load(4);
    asm.push(1);
    asm.add();
    asm.store(4);
    asm.jmp("lfs_c_loop");

    asm.label("lfs_next_b");
    asm.load(3);
    asm.push(1);
    asm.add();
    asm.store(3);
    asm.jmp("lfs_b_loop");

    asm.label("lfs_next_a");
    asm.load(2);
    asm.push(1);
    asm.add();
    asm.store(2);
    asm.jmp("lfs_a_loop");

    asm.label("lfs_found");
    // Found: next n
    asm.load(0);
    asm.push(1);
    asm.add();
    asm.store(0);
    asm.jmp("lfs_outer");

    let instr_count = asm.len() as u64;
    // B*: n_max outer * √n * √n * √n inner * ~20 instr per d check
    let mut sqrt_n: u64 = 1;
    while sqrt_n * sqrt_n <= n_max as u64 { sqrt_n += 1; }
    let b_star = (n_max as u64) * sqrt_n * sqrt_n * sqrt_n * 20 * instr_count / 10 + 1_000_000;

    let prog = asm.build().expect("Lagrange four squares program must assemble");
    (prog, b_star, "Lagrange: every n is sum of four squares")
}

/// ZFC 0≠1: trivially disproves "0=1" — Push(0), Push(1), Eq → 0 → Halt(1).
///
/// Returns: Halt(1) = 0≠1 confirmed (statement disproved).
pub fn build_zero_ne_one() -> (Program, u64, &'static str) {
    let mut asm = Asm::new();

    asm.push(0);
    asm.push(1);
    asm.eq();       // 0 == 1? → pushes 0
    // If 0 (not equal) → jz takes the jump → Halt(1) (disproved)
    asm.jz("disproved");
    asm.halt(0);    // would mean 0==1, impossible
    asm.label("disproved");
    asm.halt(1);    // 0 ≠ 1 confirmed

    let prog = asm.build().expect("ZFC program must assemble");
    (prog, 10, "0 ≠ 1 (ZFC consistency)")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::{Vm, VmOutcome};

    #[test]
    fn goldbach_small() {
        let (prog, b_star, desc) = build_goldbach(100);
        let (outcome, state) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1),
            "Goldbach failed for n_max=100: {:?} after {} steps", outcome, state.steps_taken);
        assert!(desc.contains("Goldbach"));
    }

    #[test]
    fn goldbach_1000() {
        let (prog, b_star, _) = build_goldbach(1000);
        let (outcome, state) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1),
            "Goldbach failed for n_max=1000 after {} steps (B*={})", state.steps_taken, b_star);
    }

    #[test]
    fn collatz_small() {
        let (prog, b_star, desc) = build_collatz(100, 500);
        let (outcome, _) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1));
        assert!(desc.contains("Collatz"));
    }

    #[test]
    fn collatz_5000() {
        let (prog, b_star, _) = build_collatz(5000, 1000);
        let (outcome, state) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1),
            "Collatz failed for n_max=5000 after {} steps (B*={})", state.steps_taken, b_star);
    }

    #[test]
    fn twin_primes_found() {
        let (prog, b_star, desc) = build_twin_prime_search(10000);
        let (outcome, state) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1)); // 3,5 are twin primes
        // Should exit very quickly (p=3)
        assert!(state.steps_taken < 200, "Should find twin primes quickly, took {} steps", state.steps_taken);
        assert!(desc.contains("Twin prime"));
    }

    #[test]
    fn flt_small() {
        let (prog, b_star, desc) = build_flt(5, 20);
        let (outcome, state) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1),
            "FLT failed for exp=5,base=20 after {} steps", state.steps_taken);
        assert!(desc.contains("FLT"));
    }

    #[test]
    fn flt_7_40() {
        let (prog, b_star, _) = build_flt(7, 40);
        let (outcome, state) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1),
            "FLT failed for exp=7,base=40 after {} steps (B*={})", state.steps_taken, b_star);
    }

    #[test]
    fn odd_perfect_small() {
        let (prog, b_star, desc) = build_odd_perfect(100);
        let (outcome, _) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1));
        assert!(desc.contains("odd perfect"));
    }

    #[test]
    fn odd_perfect_5000() {
        let (prog, b_star, _) = build_odd_perfect(5000);
        let (outcome, state) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1),
            "Odd perfect failed for n_max=5000 after {} steps (B*={})", state.steps_taken, b_star);
    }

    #[test]
    fn mersenne_found() {
        let (prog, b_star, desc) = build_mersenne(31);
        let (outcome, state) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1)); // 2^2-1=3 is prime
        assert!(state.steps_taken < 200, "Should find Mersenne prime quickly, took {} steps", state.steps_taken);
        assert!(desc.contains("Mersenne"));
    }

    #[test]
    fn zfc_zero_ne_one() {
        let (prog, b_star, desc) = build_zero_ne_one();
        let (outcome, state) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1));
        assert!(state.steps_taken <= 6);
        assert!(desc.contains("0 ≠ 1"));
    }

    #[test]
    fn mertens_small() {
        let (prog, b_star, desc) = build_mertens(100);
        let (outcome, state) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1),
            "Mertens failed for n_max=100: {:?} after {} steps", outcome, state.steps_taken);
        assert!(desc.contains("Mertens"));
    }

    #[test]
    fn mertens_1000() {
        let (prog, b_star, _) = build_mertens(1000);
        let (outcome, state) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1),
            "Mertens failed for n_max=1000 after {} steps (B*={})", state.steps_taken, b_star);
    }

    #[test]
    fn legendre_small() {
        let (prog, b_star, desc) = build_legendre(50);
        let (outcome, state) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1),
            "Legendre failed for n_max=50: {:?} after {} steps", outcome, state.steps_taken);
        assert!(desc.contains("Legendre"));
    }

    #[test]
    fn legendre_100() {
        let (prog, b_star, _) = build_legendre(100);
        let (outcome, state) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1),
            "Legendre failed for n_max=100 after {} steps (B*={})", state.steps_taken, b_star);
    }

    #[test]
    fn erdos_straus_small() {
        let (prog, b_star, desc) = build_erdos_straus(100);
        let (outcome, state) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1),
            "Erdos-Straus failed for n_max=100: {:?} after {} steps", outcome, state.steps_taken);
        assert!(desc.contains("Straus"));
    }

    #[test]
    fn erdos_straus_1000() {
        let (prog, b_star, _) = build_erdos_straus(1000);
        let (outcome, state) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1),
            "Erdos-Straus failed for n_max=1000 after {} steps (B*={})", state.steps_taken, b_star);
    }

    #[test]
    fn bsd_ec_count_p7() {
        let (prog, b_star, desc) = build_bsd_ec_count(7, 0);
        let (outcome, state) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1),
            "BSD EC count failed for p=7: {:?} after {} steps", outcome, state.steps_taken);
        assert!(desc.contains("BSD"));
    }

    #[test]
    fn bsd_ec_count_p97() {
        let (prog, b_star, _) = build_bsd_ec_count(97, 0);
        let (outcome, state) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1),
            "BSD EC count failed for p=97 after {} steps (B*={})", state.steps_taken, b_star);
    }

    #[test]
    fn bsd_ec_count_curve1() {
        // y² = x³ + 1
        let (prog, b_star, _) = build_bsd_ec_count(97, 1);
        let (outcome, _) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1));
    }

    #[test]
    fn bsd_ec_count_curve2() {
        // y² = x³ - x + 1 (conductor 37, rank 1)
        let (prog, b_star, _) = build_bsd_ec_count(97, 2);
        let (outcome, _) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1));
    }

    #[test]
    fn weak_goldbach_small() {
        let (prog, b_star, desc) = build_weak_goldbach(101);
        let (outcome, state) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1),
            "Weak Goldbach failed for n_max=101: {:?} after {} steps", outcome, state.steps_taken);
        assert!(desc.contains("Goldbach"));
    }

    #[test]
    fn bertrand_small() {
        let (prog, b_star, desc) = build_bertrand(100);
        let (outcome, state) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1),
            "Bertrand failed for n_max=100: {:?} after {} steps", outcome, state.steps_taken);
        assert!(desc.contains("Bertrand"));
    }

    #[test]
    fn bertrand_1000() {
        let (prog, b_star, _) = build_bertrand(1000);
        let (outcome, state) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1),
            "Bertrand failed for n_max=1000 after {} steps (B*={})", state.steps_taken, b_star);
    }

    #[test]
    fn lagrange_four_squares_small() {
        let (prog, b_star, desc) = build_lagrange_four_squares(100);
        let (outcome, state) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1),
            "Lagrange failed for n_max=100: {:?} after {} steps", outcome, state.steps_taken);
        assert!(desc.contains("Lagrange"));
    }

    #[test]
    fn lagrange_four_squares_500() {
        let (prog, b_star, _) = build_lagrange_four_squares(500);
        let (outcome, state) = Vm::run(&prog, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1),
            "Lagrange failed for n_max=500 after {} steps (B*={})", state.steps_taken, b_star);
    }

    #[test]
    fn programs_are_deterministic() {
        use kernel_types::SerPi;

        let (p1, b1, _) = build_goldbach(100);
        let (p2, b2, _) = build_goldbach(100);
        assert_eq!(p1.ser_pi_hash(), p2.ser_pi_hash());
        assert_eq!(b1, b2);

        let (p1, b1, _) = build_collatz(100, 500);
        let (p2, b2, _) = build_collatz(100, 500);
        assert_eq!(p1.ser_pi_hash(), p2.ser_pi_hash());
        assert_eq!(b1, b2);
    }
}
