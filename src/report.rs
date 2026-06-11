use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct Report {
    pub schema_version: u32,
    pub session_id: String,
    pub binary: Binary,
    pub workload: Workload,
    pub environment: Environment,
    pub sampling: Sampling,
    pub hotspots: Vec<Hotspot>,
    pub stacks: Vec<WeightedStack>,
    #[serde(default)]
    pub bottlenecks: Vec<Bottleneck>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Binary {
    pub path: PathBuf,
    pub build_id: Option<String>,
    pub demangled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Workload {
    pub name: String,
    pub args: Vec<String>,
    pub duration_s: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Environment {
    pub kernel: String,
    pub perf_paranoid: i32,
    pub cpu: String,
    pub perf_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Sampling {
    pub rate_hz: u64,
    pub samples: u64,
    pub duration_s: f64,
    #[serde(default)]
    pub counters: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Hotspot {
    pub rank: u32,
    pub function: String,
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
    pub self_pct: f64,
    pub cumulative_pct: f64,
    pub samples: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WeightedStack {
    pub weight_pct: f64,
    pub frames: Vec<StackFrame>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StackFrame {
    pub function: String,
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Bottleneck {
    pub kind: BottleneckKind,
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
    pub function: String,
    pub metric: String,
    pub value: f64,
    pub note: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum BottleneckKind {
    HotFunction,
    MemoryPressure,
    HotFile,
    Dominator,
    TailFunction,
}

impl Report {
    pub fn write_to(&self, dir: &Path) -> anyhow::Result<PathBuf> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join(format!("report-{}.json", self.session_id));
        let bytes = serde_json::to_vec_pretty(self)?;
        std::fs::write(&path, bytes)?;
        Ok(path)
    }
}
