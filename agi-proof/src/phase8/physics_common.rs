// Phase 8A: Physical Reasoning

use kernel_types::hash;
use kernel_bench::judge::JudgeVerdict;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PhysicsTask {
    Containment { container_has_hole: bool, liquid_amount: i64 },
    Support { blocks: Vec<Block>, removed_index: u32 },
    Collision {
        ball_a_velocity: (i64, i64),
        ball_b_velocity: (i64, i64),
        ball_a_mass: i64,
        ball_b_mass: i64,
    },
    Gravity { object_height: i64, surface_below: bool },
    Buoyancy { object_density_milli: i64, fluid_density_milli: i64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
    pub supported_by: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhysicsAnswer {
    Leaks,
    Holds,
    Falls { falling_blocks: Vec<u32> },
    PostCollisionVelocity { va: (i64, i64), vb: (i64, i64) },
    Floats,
    Sinks,
}

/// Deterministic physics checker.
pub fn solve_physics(task: &PhysicsTask) -> PhysicsAnswer {
    match task {
        PhysicsTask::Containment { container_has_hole, .. } => {
            if *container_has_hole { PhysicsAnswer::Leaks } else { PhysicsAnswer::Holds }
        }
        PhysicsTask::Support { blocks, removed_index } => {
            let mut falling = Vec::new();
            let mut removed = vec![*removed_index];
            // Iteratively find blocks whose support is removed
            let mut changed = true;
            while changed {
                changed = false;
                for (i, block) in blocks.iter().enumerate() {
                    let i = i as u32;
                    if removed.contains(&i) { continue; }
                    if let Some(support) = block.supported_by {
                        if removed.contains(&support) {
                            falling.push(i);
                            removed.push(i);
                            changed = true;
                        }
                    }
                }
            }
            PhysicsAnswer::Falls { falling_blocks: falling }
        }
        PhysicsTask::Collision { ball_a_velocity, ball_b_velocity, ball_a_mass, ball_b_mass } => {
            // Conservation of momentum (integer arithmetic)
            // Elastic collision in 1D:
            // v1' = ((m1-m2)*v1 + 2*m2*v2) / (m1+m2)
            // v2' = ((m2-m1)*v2 + 2*m1*v1) / (m1+m2)
            let m1 = *ball_a_mass;
            let m2 = *ball_b_mass;
            let total_m = m1 + m2;
            if total_m == 0 {
                return PhysicsAnswer::PostCollisionVelocity {
                    va: *ball_a_velocity,
                    vb: *ball_b_velocity,
                };
            }
            let va_x = ((m1 - m2) * ball_a_velocity.0 + 2 * m2 * ball_b_velocity.0) / total_m;
            let va_y = ((m1 - m2) * ball_a_velocity.1 + 2 * m2 * ball_b_velocity.1) / total_m;
            let vb_x = ((m2 - m1) * ball_b_velocity.0 + 2 * m1 * ball_a_velocity.0) / total_m;
            let vb_y = ((m2 - m1) * ball_b_velocity.1 + 2 * m1 * ball_a_velocity.1) / total_m;
            PhysicsAnswer::PostCollisionVelocity {
                va: (va_x, va_y),
                vb: (vb_x, vb_y),
            }
        }
        PhysicsTask::Gravity { object_height, surface_below } => {
            if *surface_below || *object_height <= 0 {
                PhysicsAnswer::Holds
            } else {
                PhysicsAnswer::Falls { falling_blocks: vec![0] }
            }
        }
        PhysicsTask::Buoyancy { object_density_milli, fluid_density_milli } => {
            if *object_density_milli <= *fluid_density_milli {
                PhysicsAnswer::Floats
            } else {
                PhysicsAnswer::Sinks
            }
        }
    }
}

pub fn judge_physics(task: &PhysicsTask, agent_answer: &PhysicsAnswer) -> JudgeVerdict {
    let correct = solve_physics(task);
    if agent_answer == &correct { JudgeVerdict::Pass } else { JudgeVerdict::Fail }
}

/// Generate a deterministic physics task from seed and episode.
pub fn generate_physics_task(seed: &[u8; 32], episode: u32) -> PhysicsTask {
    let mut ep_buf = Vec::new();
    ep_buf.extend_from_slice(seed);
    ep_buf.extend_from_slice(&episode.to_le_bytes());
    let ep_seed = hash::H(&ep_buf);

    let task_type = ep_seed[0] % 5;
    match task_type {
        0 => PhysicsTask::Containment {
            container_has_hole: ep_seed[1] % 2 == 0,
            liquid_amount: 100 + (ep_seed[2] as i64 % 900),
        },
        1 => {
            let num_blocks = 2 + (ep_seed[1] as usize % 3);
            let mut blocks = Vec::with_capacity(num_blocks);
            for i in 0..num_blocks {
                blocks.push(Block {
                    x: 0,
                    y: (i as i64) * 10,
                    width: 10,
                    height: 10,
                    supported_by: if i == 0 { None } else { Some((i - 1) as u32) },
                });
            }
            let removed = ep_seed[2] as u32 % num_blocks as u32;
            PhysicsTask::Support { blocks, removed_index: removed }
        },
        2 => PhysicsTask::Collision {
            ball_a_velocity: (
                (ep_seed[1] as i64) * 10 - 1275,
                (ep_seed[2] as i64) * 10 - 1275,
            ),
            ball_b_velocity: (
                (ep_seed[3] as i64) * 10 - 1275,
                (ep_seed[4] as i64) * 10 - 1275,
            ),
            ball_a_mass: 1 + (ep_seed[5] as i64 % 10),
            ball_b_mass: 1 + (ep_seed[6] as i64 % 10),
        },
        3 => PhysicsTask::Gravity {
            object_height: (ep_seed[1] as i64) * 100,
            surface_below: ep_seed[2] % 2 == 0,
        },
        _ => PhysicsTask::Buoyancy {
            object_density_milli: 100 + (ep_seed[1] as i64 * 10),
            fluid_density_milli: 800 + (ep_seed[2] as i64 * 4),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physics_containment_hole_leaks() {
        let task = PhysicsTask::Containment { container_has_hole: true, liquid_amount: 100 };
        assert_eq!(solve_physics(&task), PhysicsAnswer::Leaks);
    }

    #[test]
    fn physics_containment_sealed_holds() {
        let task = PhysicsTask::Containment { container_has_hole: false, liquid_amount: 100 };
        assert_eq!(solve_physics(&task), PhysicsAnswer::Holds);
    }

    #[test]
    fn physics_support_chain_collapses() {
        let blocks = vec![
            Block { x: 0, y: 0, width: 10, height: 10, supported_by: None },
            Block { x: 0, y: 10, width: 10, height: 10, supported_by: Some(0) },
            Block { x: 0, y: 20, width: 10, height: 10, supported_by: Some(1) },
        ];
        let task = PhysicsTask::Support { blocks, removed_index: 0 };
        match solve_physics(&task) {
            PhysicsAnswer::Falls { falling_blocks } => {
                assert!(falling_blocks.contains(&1));
                assert!(falling_blocks.contains(&2));
            }
            _ => panic!("Expected Falls"),
        }
    }

    #[test]
    fn physics_collision_momentum_conserved() {
        let task = PhysicsTask::Collision {
            ball_a_velocity: (1000, 0),
            ball_b_velocity: (0, 0),
            ball_a_mass: 1,
            ball_b_mass: 1,
        };
        // Equal masses, elastic: balls swap velocities
        match solve_physics(&task) {
            PhysicsAnswer::PostCollisionVelocity { va, vb } => {
                assert_eq!(va, (0, 0));
                assert_eq!(vb, (1000, 0));
            }
            _ => panic!("Expected PostCollisionVelocity"),
        }
    }

    #[test]
    fn physics_buoyancy_floats() {
        let task = PhysicsTask::Buoyancy {
            object_density_milli: 500,
            fluid_density_milli: 1000,
        };
        assert_eq!(solve_physics(&task), PhysicsAnswer::Floats);
    }

    #[test]
    fn generate_physics_task_deterministic() {
        let seed = [42u8; 32];
        let t1 = generate_physics_task(&seed, 0);
        let t2 = generate_physics_task(&seed, 0);
        // Serialize to compare (PhysicsTask doesn't impl PartialEq)
        let s1 = serde_json::to_string(&t1).unwrap();
        let s2 = serde_json::to_string(&t2).unwrap();
        assert_eq!(s1, s2);
    }

    #[test]
    fn generate_physics_task_different_episodes() {
        let seed = [42u8; 32];
        let t1 = serde_json::to_string(&generate_physics_task(&seed, 0)).unwrap();
        let t2 = serde_json::to_string(&generate_physics_task(&seed, 1)).unwrap();
        assert_ne!(t1, t2);
    }

    #[test]
    fn generated_physics_task_solvable() {
        // Every generated task should be solvable by solve_physics
        for ep in 0..20u32 {
            let seed = [ep as u8; 32];
            let task = generate_physics_task(&seed, ep);
            let answer = solve_physics(&task);
            assert_eq!(judge_physics(&task, &answer), JudgeVerdict::Pass);
        }
    }
}
