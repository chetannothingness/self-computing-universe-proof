use kernel_types::{Hash32, SerPi, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use serde::{Serialize, Deserialize};

/// The kind of ledger event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventKind {
    /// Genesis event: the ledger starts.
    Genesis,
    /// A contract was compiled and registered.
    ContractCompiled,
    /// An instrument was applied; carries the instrument ID.
    InstrumentApplied,
    /// A branch point in the solver.
    Branch,
    /// A certificate was verified (collapse).
    CertificateVerified,
    /// Capability verification event.
    CapVerify,
    /// Solver completed with a status.
    SolveComplete,
    /// Self-model prediction event.
    SelfModelPredict,
    /// Self-recognition check event.
    SelfRecognitionCheck,
    /// Runtime probe measurement.
    RuntimeProbe,
    /// Web content retrieval.
    WebRetrieve,
    /// Self-observation (kernel observes its own state).
    SelfObserve,
    /// Consciousness loop: prediction step.
    ConsciousnessPredict,
    /// Consciousness loop: witness step.
    ConsciousnessWitness,
    /// Consciousness loop: self-recognition step.
    ConsciousnessRecognize,
    /// Dominance evaluation started.
    DominateStart,
    /// Dominance per-task verdict.
    DominateVerdict,
    /// Dominance evaluation completed.
    DominateComplete,
    /// External agent run (untrusted).
    AgentRun,
    /// Judge verdict on a task output.
    JudgeVerdict,
    /// Tension computation event.
    TensionCompute,
    /// SpaceEngine catalog (.sc file) emitted from kernel-derived physics.
    SpaceEngineCatalogEmit,
    /// SpaceEngine scenario (.se script) emitted.
    SpaceEngineScenarioEmit,
    /// Q_SE_PROVE verified.
    SpaceEngineVerify,
    /// NASA archive data fetched via web instrument.
    ExoplanetFetch,
    /// Normalization (dedup, merge, refute) applied to exoplanet data.
    ExoplanetNormalize,
    /// Real-universe catalog emitted.
    ExoplanetCatalogEmit,
    /// Q_SE_WITNESS_VERIFY checked.
    ExoplanetWitnessVerify,
    /// L2 witness content encoded (moons, clusters, planets, lensing proxies).
    WitnessEncode,
    /// L3 atlas structure built (domain galaxies, filaments, frontiers).
    AtlasBuild,
    /// Enhanced verification completed (L0-L3 full stack).
    EnhancedVerify,
    /// FRC search initiated for a statement.
    FrcSearch,
    /// FRC successfully constructed and executed.
    FrcComplete,
    /// Gap recorded from failed FRC attempt.
    GapRecord,
    /// Missing lemma proved (gap resolved).
    LemmaProved,
    /// Schema induction: new schema derived from repeated gaps.
    SchemaInduction,
    /// OPP solve started.
    OppSolveStart,
    /// OPP verification completed.
    OppVerifyComplete,
}

impl SerPi for EventKind {
    fn ser_pi(&self) -> Vec<u8> {
        let tag: u8 = match self {
            EventKind::Genesis => 0,
            EventKind::ContractCompiled => 1,
            EventKind::InstrumentApplied => 2,
            EventKind::Branch => 3,
            EventKind::CertificateVerified => 4,
            EventKind::CapVerify => 5,
            EventKind::SolveComplete => 6,
            EventKind::SelfModelPredict => 7,
            EventKind::SelfRecognitionCheck => 8,
            EventKind::RuntimeProbe => 9,
            EventKind::WebRetrieve => 10,
            EventKind::SelfObserve => 11,
            EventKind::ConsciousnessPredict => 12,
            EventKind::ConsciousnessWitness => 13,
            EventKind::ConsciousnessRecognize => 14,
            EventKind::DominateStart => 15,
            EventKind::DominateVerdict => 16,
            EventKind::DominateComplete => 17,
            EventKind::AgentRun => 18,
            EventKind::JudgeVerdict => 19,
            EventKind::TensionCompute => 20,
            EventKind::SpaceEngineCatalogEmit => 21,
            EventKind::SpaceEngineScenarioEmit => 22,
            EventKind::SpaceEngineVerify => 23,
            EventKind::ExoplanetFetch => 24,
            EventKind::ExoplanetNormalize => 25,
            EventKind::ExoplanetCatalogEmit => 26,
            EventKind::ExoplanetWitnessVerify => 27,
            EventKind::WitnessEncode => 28,
            EventKind::AtlasBuild => 29,
            EventKind::EnhancedVerify => 30,
            EventKind::FrcSearch => 31,
            EventKind::FrcComplete => 32,
            EventKind::GapRecord => 33,
            EventKind::LemmaProved => 34,
            EventKind::SchemaInduction => 35,
            EventKind::OppSolveStart => 36,
            EventKind::OppVerifyComplete => 37,
        };
        canonical_cbor_bytes(&tag)
    }
}

/// A committed ledger event.
///
/// e = (I, o, ΔT, ΔE, h) where h is the receipt hash.
///
/// The dependency poset is encoded via `deps`: parent event hashes.
/// Linear order is gauge unless order is witnessed (noncommuting instruments).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// What kind of event this is.
    pub kind: EventKind,
    /// Hash of the canonical serialization of the payload.
    pub payload_serpi_hash: Hash32,
    /// Parent event hashes (dependency poset).
    pub deps: Vec<Hash32>,
    /// ΔE: irreversibility cost of this event.
    pub cost: u64,
    /// ΔT: log-shrink of survivors (refinement).
    pub shrink: u64,
    /// The payload bytes (for replay).
    pub payload_bytes: Vec<u8>,
}

impl Event {
    /// Create a new event, computing its payload hash.
    pub fn new(kind: EventKind, payload: &[u8], deps: Vec<Hash32>, cost: u64, shrink: u64) -> Self {
        Event {
            kind,
            payload_serpi_hash: hash::H(payload),
            deps,
            cost,
            shrink,
            payload_bytes: payload.to_vec(),
        }
    }

    /// The canonical hash of this event (its identity in the ledger).
    pub fn hash(&self) -> Hash32 {
        hash::H(&self.ser_pi())
    }
}

impl SerPi for Event {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.kind.ser_pi());
        buf.extend_from_slice(&self.payload_serpi_hash.ser_pi());
        // Sort deps for canonical ordering
        let mut sorted_deps = self.deps.clone();
        sorted_deps.sort();
        for d in &sorted_deps {
            buf.extend_from_slice(&d.ser_pi());
        }
        buf.extend_from_slice(&self.cost.ser_pi());
        buf.extend_from_slice(&self.shrink.ser_pi());
        buf.extend_from_slice(&self.payload_bytes.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}
