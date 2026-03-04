// Phase 8A: Physical Reasoning
// Full implementation in Week 2.

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
}
