//! Named latent channels: per-flower and per-petal quantities that travel
//! with the geometry into the opening stage and the export.
//!
//! The machinery never knows what a channel means. A channel is declared
//! once (name, scope, dims, optional range, kind) and gets its values from
//! its own random sub-stream keyed by a hash of its name, so adding a
//! channel never shifts the draws of the others. Consumers (the opening
//! stage, the viewer) look channels up by name.

use crate::rng::{derive, Pcg32};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Point,
    Petal,
    Flower,
}

/// How a channel is meant to evolve during opening.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// Never changes.
    Fixed,
    /// A goal that a growth quantity moves toward.
    Target,
    /// Current state, updated by the stage that owns it.
    State,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ChannelDecl {
    pub name: String,
    pub scope: Scope,
    pub dims: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<[f64; 2]>,
    pub kind: Kind,
    /// Names of the values of a categorical fixed channel (0, 1, 2, ...).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
}

/// Declarations plus current values. Values are flattened: a petal channel
/// with dims d holds petals * d numbers, a flower channel holds d numbers.
#[derive(Clone, Debug, Default)]
pub struct Channels {
    pub decls: Vec<ChannelDecl>,
    pub values: BTreeMap<String, Vec<f64>>,
}

/// FNV-1a, so a channel's stream depends only on its name.
fn name_key(name: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in name.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

/// The random sub-stream for channel `name` under `seed`.
pub fn channel_rng(seed: u64, name: &str) -> Pcg32 {
    let k = name_key(name);
    Pcg32::new(derive(seed, k), k)
}

impl Channels {
    pub fn declare(&mut self, name: &str, scope: Scope, dims: usize, range: Option<[f64; 2]>, kind: Kind, values: Vec<f64>) {
        assert!(!self.values.contains_key(name), "channel {name} declared twice");
        self.decls.push(ChannelDecl { name: name.to_string(), scope, dims, range, kind, labels: None });
        self.values.insert(name.to_string(), values);
    }

    /// Attach value labels to a categorical channel.
    pub fn label(&mut self, name: &str, labels: &[&str]) {
        if let Some(d) = self.decls.iter_mut().find(|d| d.name == name) {
            d.labels = Some(labels.iter().map(|l| l.to_string()).collect());
        }
    }

    pub fn get(&self, name: &str) -> Option<&[f64]> {
        self.values.get(name).map(|v| v.as_slice())
    }

    pub fn decl(&self, name: &str) -> Option<&ChannelDecl> {
        self.decls.iter().find(|d| d.name == name)
    }
}
