#[derive(Debug, Clone, PartialEq)]
pub enum LaneKind {
    Robust,  // state label shown at each segment
    Concise, // condensed — label once per state span
    Clock,   // pulse train
    Binary,  // two-level waveform
}

#[derive(Debug, Clone)]
pub struct Lane {
    pub name: String, // canonical identifier used by `is` statements
    pub label: Option<String>,
    pub kind: LaneKind,
}

/// A transition event: at time `t` lane `lane` enters state `state`.
#[derive(Debug, Clone)]
pub struct Event {
    pub time: u64,
    pub lane: String,
    pub state: String,
}

#[derive(Debug, Clone, Default)]
pub struct TimingDiagram {
    pub title: Option<String>,
    pub lanes: Vec<Lane>,
    pub events: Vec<Event>,
    pub skinparams: Vec<(String, String)>,
}
