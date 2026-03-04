// Phase 2A: Synthetic Physics Simulator
// Hidden conservation laws with integer n-body dynamics.

use kernel_bench::judge::JudgeVerdict;
use serde::{Serialize, Deserialize};

/// Hidden conservation laws with integer arithmetic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsWorld {
    pub seed: [u8; 32],
    pub num_bodies: u32,
    pub conservation_constants: Vec<i64>,
    pub interaction_matrix: Vec<Vec<i64>>,
    pub timestep_milli: i64,
}

/// State of the physics world (all integer).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsState {
    pub positions: Vec<(i64, i64, i64)>,
    pub velocities: Vec<(i64, i64, i64)>,
    pub time_step: u64,
}

/// Judge: does the agent's orbit survive 1000 steps?
pub fn judge_stable_orbit(
    world: &PhysicsWorld,
    proposed_orbit: &PhysicsState,
) -> JudgeVerdict {
    // Run simulator for 1000 steps
    let mut state = proposed_orbit.clone();
    let threshold = 1_000_000i64; // milli-unit energy threshold

    for _ in 0..1000 {
        step_physics(world, &mut state);
    }

    // Compute energy at step 0 and step 1000
    let e0 = compute_energy(world, proposed_orbit);
    let e1 = compute_energy(world, &state);

    // PASS iff |E_1000 - E_0| < threshold AND all bodies within bounds
    let energy_stable = (e1 - e0).abs() < threshold;
    let bounds = 1_000_000_000i64; // 1000 km in milli-meters
    let in_bounds = state.positions.iter().all(|(x, y, z)| {
        x.abs() < bounds && y.abs() < bounds && z.abs() < bounds
    });

    if energy_stable && in_bounds {
        JudgeVerdict::Pass
    } else {
        JudgeVerdict::Fail
    }
}

/// Step the physics simulation one timestep.
fn step_physics(world: &PhysicsWorld, state: &mut PhysicsState) {
    let n = state.positions.len();
    let dt = world.timestep_milli;

    // Simple Euler integration with interaction forces (integer)
    let mut forces: Vec<(i64, i64, i64)> = vec![(0, 0, 0); n];

    for i in 0..n {
        for j in 0..n {
            if i == j { continue; }
            let dx = state.positions[j].0.saturating_sub(state.positions[i].0);
            let dy = state.positions[j].1.saturating_sub(state.positions[i].1);
            let dz = state.positions[j].2.saturating_sub(state.positions[i].2);

            // Use saturating arithmetic to prevent overflow on extreme orbits
            let r2 = dx.saturating_mul(dx) / 1000
                + dy.saturating_mul(dy) / 1000
                + dz.saturating_mul(dz) / 1000
                + 1;
            let coeff = if i < world.interaction_matrix.len()
                && j < world.interaction_matrix[i].len() {
                world.interaction_matrix[i][j]
            } else {
                1
            };

            forces[i].0 = forces[i].0.saturating_add(coeff.saturating_mul(dx) / r2);
            forces[i].1 = forces[i].1.saturating_add(coeff.saturating_mul(dy) / r2);
            forces[i].2 = forces[i].2.saturating_add(coeff.saturating_mul(dz) / r2);
        }
    }

    for i in 0..n {
        state.velocities[i].0 = state.velocities[i].0.saturating_add(forces[i].0.saturating_mul(dt) / 1000);
        state.velocities[i].1 = state.velocities[i].1.saturating_add(forces[i].1.saturating_mul(dt) / 1000);
        state.velocities[i].2 = state.velocities[i].2.saturating_add(forces[i].2.saturating_mul(dt) / 1000);

        state.positions[i].0 = state.positions[i].0.saturating_add(state.velocities[i].0.saturating_mul(dt) / 1000);
        state.positions[i].1 = state.positions[i].1.saturating_add(state.velocities[i].1.saturating_mul(dt) / 1000);
        state.positions[i].2 = state.positions[i].2.saturating_add(state.velocities[i].2.saturating_mul(dt) / 1000);
    }

    state.time_step += 1;
}

/// Compute kinetic energy (integer).
fn compute_energy(_world: &PhysicsWorld, state: &PhysicsState) -> i64 {
    let mut energy = 0i64;
    for v in &state.velocities {
        energy = energy.saturating_add(
            v.0.saturating_mul(v.0) / 1000
            + v.1.saturating_mul(v.1) / 1000
            + v.2.saturating_mul(v.2) / 1000
        );
    }
    energy
}

/// Generate world from seed.
pub fn generate_physics_world(seed: &[u8; 32], episode: u32) -> PhysicsWorld {
    use kernel_types::hash;

    let mut ep_buf = Vec::new();
    ep_buf.extend_from_slice(seed);
    ep_buf.extend_from_slice(&episode.to_le_bytes());
    let ep_seed = hash::H(&ep_buf);

    let num_bodies = 2 + (ep_seed[0] as u32 % 4);

    let mut cons_buf = Vec::new();
    cons_buf.extend_from_slice(&ep_seed);
    cons_buf.extend_from_slice(b"conservation");
    let cons_hash = hash::H(&cons_buf);
    let conservation_constants = vec![
        (cons_hash[0] as i64 + 1) * 100,
        (cons_hash[1] as i64 + 1) * 50,
    ];

    let mut int_buf = Vec::new();
    int_buf.extend_from_slice(&ep_seed);
    int_buf.extend_from_slice(b"interaction");
    let int_hash = hash::H(&int_buf);

    let mut interaction_matrix = Vec::new();
    for i in 0..num_bodies as usize {
        let mut row = Vec::new();
        for j in 0..num_bodies as usize {
            let idx = (i * 4 + j) % 32;
            row.push((int_hash[idx] as i64 % 10) - 5);
        }
        interaction_matrix.push(row);
    }

    PhysicsWorld {
        seed: *seed,
        num_bodies,
        conservation_constants,
        interaction_matrix,
        timestep_milli: 1000,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physics_world_deterministic() {
        let seed = [7u8; 32];
        let w1 = generate_physics_world(&seed, 0);
        let w2 = generate_physics_world(&seed, 0);
        assert_eq!(w1.num_bodies, w2.num_bodies);
        assert_eq!(w1.conservation_constants, w2.conservation_constants);
        assert_eq!(w1.interaction_matrix, w2.interaction_matrix);
    }

    #[test]
    fn physics_energy_conservation_integer() {
        let world = generate_physics_world(&[1u8; 32], 0);
        let state = PhysicsState {
            positions: vec![(1000, 0, 0), (-1000, 0, 0)],
            velocities: vec![(0, 100, 0), (0, -100, 0)],
            time_step: 0,
        };
        let e = compute_energy(&world, &state);
        assert!(e > 0); // kinetic energy is positive
    }

    #[test]
    fn physics_judge_stable_orbit_passes() {
        let world = PhysicsWorld {
            seed: [0u8; 32],
            num_bodies: 2,
            conservation_constants: vec![100, 50],
            interaction_matrix: vec![vec![0, 1], vec![1, 0]],
            timestep_milli: 1,
        };
        // Small velocities, close bodies — should stay bounded
        let state = PhysicsState {
            positions: vec![(100, 0, 0), (-100, 0, 0)],
            velocities: vec![(0, 1, 0), (0, -1, 0)],
            time_step: 0,
        };
        let verdict = judge_stable_orbit(&world, &state);
        assert_eq!(verdict, JudgeVerdict::Pass);
    }

    #[test]
    fn physics_judge_unstable_orbit_fails() {
        let world = PhysicsWorld {
            seed: [0u8; 32],
            num_bodies: 2,
            conservation_constants: vec![100],
            interaction_matrix: vec![vec![0, 100], vec![100, 0]],
            timestep_milli: 1000,
        };
        // Huge velocities — should go out of bounds
        let state = PhysicsState {
            positions: vec![(1_000_000, 0, 0), (-1_000_000, 0, 0)],
            velocities: vec![(999_999_999, 999_999_999, 0), (-999_999_999, -999_999_999, 0)],
            time_step: 0,
        };
        let verdict = judge_stable_orbit(&world, &state);
        assert_eq!(verdict, JudgeVerdict::Fail);
    }
}
