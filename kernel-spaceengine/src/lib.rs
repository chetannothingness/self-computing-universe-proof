// Part A: Kernel-derived physics catalog
pub mod types;
pub mod catalog;
pub mod scenario;
pub mod verifier;
pub mod manifest;

// Part B: Real-universe exoplanet autopatch
pub mod exo_types;
pub mod exo_normalizer;
pub mod exo_catalog;
pub mod exo_scenario;
pub mod exo_verifier;
pub mod pak;

// Re-exports
pub use types::KernelCatalog;
pub use catalog::CatalogGenerator;
pub use scenario::{ScenarioGenerator, ScenarioScript};
pub use verifier::SpaceEngineVerifier;
pub use manifest::ManifestGenerator;
pub use exo_types::RealUniverseCatalog;
pub use exo_normalizer::ExoNormalizer;
pub use exo_catalog::ExoCatalogEmitter;
pub use exo_scenario::ExoScenarioGenerator;
pub use exo_verifier::ExoWitnessVerifier;
pub use pak::PakBuilder;
