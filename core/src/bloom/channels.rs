//! Named channels: the opening stage's whole non-geometric state.
//!
//! A channel is a named quantity held per flower, per petal or per point
//! (sheet node), with 1 or 3 numbers each. Every channel follows one rule:
//!
//! * `Fixed`: never changes (petal type, twist direction, bud latents).
//! * `Drift`: mean-reverting noise, x += revert (mean - x) dt + sigma sqrt(dt) N(0,1).
//! * `Growth`: closes the gap to its goal g = gain * target + bias along a
//!   smoothstep window, x += (g - x) * rate * dS / (1 - S), where S runs 0 -> 1 between
//!   `onset` and `onset + duration`. With rate 1 this lands exactly on the
//!   target; slower petals stop short, faster ones arrive early. `monotone`
//!   forbids decreasing steps, so sizes never shrink.
//! * `Derived`: x = gain * from^power + bias (power defaults to 1; the sign
//!   of `from` is kept), recomputed every step (unit changes, mixing a
//!   species setting into a bud latent, a per-layer profile).
//! * `State`: owned by a consumer (geometry, sepals); the stepper leaves it.
//!
//! Every rule parameter is a constant or the *name* of another channel, so
//! channels are wired together by data and the stepper never knows what a
//! channel means. A step reads only the state at t_k and writes t_{k+1}, so
//! channel order cannot matter and the chain is Markov.
//!
//! Each (channel, petal) draws from its own PCG32 stream keyed by a hash of
//! the channel's name, so adding or removing a channel never shifts another
//! channel's random draws.

use crate::math;
use crate::rng::{derive, Pcg32};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Flower,
    Petal,
    Point,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Param {
    Const(f64),
    Channel(String),
}

impl From<f64> for Param {
    fn from(x: f64) -> Self {
        Param::Const(x)
    }
}

impl From<&str> for Param {
    fn from(s: &str) -> Self {
        Param::Channel(s.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "rule", rename_all = "lowercase")]
pub enum Rule {
    Fixed,
    Drift { mean: Param, revert: Param, sigma: Param },
    Growth { target: Param, gain: Param, bias: Param, rate: Param, onset: Param, duration: Param, monotone: bool },
    Derived {
        from: Param,
        gain: Param,
        bias: Param,
        #[serde(default = "unit_power")]
        power: Param,
    },
    State,
}

fn unit_power() -> Param {
    Param::Const(1.0)
}

/// `v^p`, keeping the sign of `v`; exactly `v` when p is 1.
fn shaped(v: f64, p: f64) -> f64 {
    if p == 1.0 {
        v
    } else {
        math::pow(v.abs(), p).copysign(v)
    }
}

impl Rule {
    pub fn kind(&self) -> &'static str {
        match self {
            Rule::Fixed => "fixed",
            Rule::Drift { .. } => "drift",
            Rule::Growth { .. } => "growth",
            Rule::Derived { .. } => "derived",
            Rule::State => "state",
        }
    }

    fn params(&self) -> Vec<&Param> {
        match self {
            Rule::Fixed | Rule::State => vec![],
            Rule::Derived { from, gain, bias, power } => vec![from, gain, bias, power],
            Rule::Drift { mean, revert, sigma } => vec![mean, revert, sigma],
            Rule::Growth { target, gain, bias, rate, onset, duration, .. } => vec![target, gain, bias, rate, onset, duration],
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ChannelSpec {
    pub name: String,
    pub scope: Scope,
    pub dims: usize,
    #[serde(flatten)]
    pub rule: Rule,
    /// Starting value (dims numbers) when no input supplies one.
    pub default: Vec<f64>,
    #[serde(default)]
    pub export: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
}

impl ChannelSpec {
    pub fn new(name: &str, scope: Scope, dims: usize, rule: Rule, default: Vec<f64>) -> Self {
        ChannelSpec { name: name.into(), scope, dims, rule, default, export: false, range: None, labels: None }
    }

    pub fn exported(mut self) -> Self {
        self.export = true;
        self
    }
}

/// A channel's starting values, as another stage hands them over: one
/// vector for flower scope, one per petal for petal and point scope
/// (point: nodes * dims, row-major).
#[derive(Clone, Debug, PartialEq)]
pub struct Input {
    pub name: String,
    pub scope: Scope,
    pub dims: usize,
    pub values: Vec<Vec<f64>>,
    pub labels: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChannelError {
    UnknownChannel { channel: String, param: String },
    Scope { channel: String, param: String },
    Dims { channel: String, detail: String },
    Duplicate(String),
}

impl fmt::Display for ChannelError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ChannelError::UnknownChannel { channel, param } => write!(f, "channel {channel} refers to unknown channel {param}"),
            ChannelError::Scope { channel, param } => write!(f, "channel {channel} reads {param}, which has a finer scope"),
            ChannelError::Dims { channel, detail } => write!(f, "channel {channel}: {detail}"),
            ChannelError::Duplicate(n) => write!(f, "channel {n} declared twice"),
        }
    }
}

impl std::error::Error for ChannelError {}

/// FNV-1a, so a channel's stream key is fixed by its name alone.
pub fn name_key(name: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in name.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Smooth 0 -> 1 window between onset and onset + duration.
pub fn window(t: f64, onset: f64, duration: f64) -> f64 {
    let x = ((t - onset) / duration.max(1e-9)).clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

#[derive(Clone, Debug)]
struct Channel {
    spec: ChannelSpec,
    /// values[0] for flower scope, values[p] per petal otherwise.
    values: Vec<Vec<f64>>,
    rngs: Vec<Pcg32>,
}

/// All channels of one flower at one time step.
#[derive(Clone, Debug)]
pub struct Channels {
    chans: BTreeMap<String, Channel>,
    nodes: Vec<usize>,
}

impl Channels {
    /// Build from the species' specs and the inputs handed over by the bud.
    /// An input with a spec gives that channel its starting value; an input
    /// without one becomes a fixed channel and is exported as it came; a
    /// spec without an input starts from its default.
    pub fn new(specs: &[ChannelSpec], inputs: &[Input], nodes: &[usize], seed: u64) -> Result<Self, ChannelError> {
        let petals = nodes.len();
        let mut all: BTreeMap<String, ChannelSpec> = BTreeMap::new();
        for s in specs {
            if all.insert(s.name.clone(), s.clone()).is_some() {
                return Err(ChannelError::Duplicate(s.name.clone()));
            }
        }
        let mut given: BTreeMap<String, &Input> = BTreeMap::new();
        for i in inputs {
            if given.insert(i.name.clone(), i).is_some() {
                return Err(ChannelError::Duplicate(i.name.clone()));
            }
            all.entry(i.name.clone()).or_insert_with(|| ChannelSpec {
                name: i.name.clone(),
                scope: i.scope,
                dims: i.dims,
                rule: Rule::Fixed,
                default: vec![0.0; i.dims],
                export: true,
                range: None,
                labels: i.labels.clone(),
            });
        }

        for s in all.values() {
            if s.dims == 0 || s.default.len() != s.dims {
                return Err(ChannelError::Dims { channel: s.name.clone(), detail: "default must have dims values".into() });
            }
            for p in s.rule.params() {
                if let Param::Channel(n) = p {
                    let Some(src) = all.get(n) else {
                        return Err(ChannelError::UnknownChannel { channel: s.name.clone(), param: n.clone() });
                    };
                    if src.scope > s.scope {
                        return Err(ChannelError::Scope { channel: s.name.clone(), param: n.clone() });
                    }
                    if src.dims != 1 && src.dims != s.dims {
                        return Err(ChannelError::Dims { channel: s.name.clone(), detail: format!("parameter {n} has {} dims", src.dims) });
                    }
                }
            }
        }

        let key = derive(seed, 0xB100_0000);
        let mut chans = BTreeMap::new();
        for (name, spec) in all {
            let count = |p: usize| match spec.scope {
                Scope::Flower | Scope::Petal => spec.dims,
                Scope::Point => nodes[p] * spec.dims,
            };
            let slots = if spec.scope == Scope::Flower { 1 } else { petals };
            let values: Vec<Vec<f64>> = match given.get(&name) {
                Some(inp) => {
                    if inp.scope != spec.scope || inp.dims != spec.dims || inp.values.len() != slots {
                        return Err(ChannelError::Dims { channel: name, detail: "input scope, dims or petal count differs from spec".into() });
                    }
                    for (p, v) in inp.values.iter().enumerate() {
                        if v.len() != count(p) {
                            return Err(ChannelError::Dims { channel: name, detail: format!("input for slot {p} has {} values", v.len()) });
                        }
                    }
                    inp.values.clone()
                }
                None => (0..slots).map(|p| spec.default.iter().cycle().take(count(p)).cloned().collect()).collect(),
            };
            let ck = derive(key, name_key(&name));
            let rngs = (0..slots).map(|p| Pcg32::new(ck, p as u64)).collect();
            chans.insert(name, Channel { spec, values, rngs });
        }
        let mut out = Channels { chans, nodes: nodes.to_vec() };
        out.derive();
        Ok(out)
    }

    /// Set every `Derived` channel from the current values (so they hold
    /// from the start, not only after the first step).
    fn derive(&mut self) {
        let mut next = Vec::new();
        for (name, c) in &self.chans {
            if let Rule::Derived { from, gain, bias, power } = &c.spec.rule {
                let d = c.spec.dims;
                let values: Vec<Vec<f64>> = c
                    .values
                    .iter()
                    .enumerate()
                    .map(|(slot, v)| {
                        (0..v.len())
                            .map(|i| {
                                let pr = |p: &Param| self.param(p, slot, i / d, i % d);
                                pr(gain) * shaped(pr(from), pr(power)) + pr(bias)
                            })
                            .collect()
                    })
                    .collect();
                next.push((name.clone(), values));
            }
        }
        for (name, values) in next {
            self.chans.get_mut(&name).expect("same keys").values = values;
        }
    }

    pub fn petals(&self) -> usize {
        self.nodes.len()
    }

    pub fn nodes(&self, petal: usize) -> usize {
        self.nodes[petal]
    }

    pub fn specs(&self) -> impl Iterator<Item = &ChannelSpec> {
        self.chans.values().map(|c| &c.spec)
    }

    pub fn spec(&self, name: &str) -> Option<&ChannelSpec> {
        self.chans.get(name).map(|c| &c.spec)
    }

    /// Raw values: flower scope ignores `petal`.
    pub fn get(&self, name: &str, petal: usize) -> Option<&[f64]> {
        let c = self.chans.get(name)?;
        Some(if c.spec.scope == Scope::Flower { &c.values[0] } else { &c.values[petal] })
    }

    /// First number of a flower or petal channel, or `fallback` if absent.
    pub fn scalar(&self, name: &str, petal: usize, fallback: f64) -> f64 {
        self.get(name, petal).and_then(|v| v.first().copied()).unwrap_or(fallback)
    }

    /// Value seen at (petal, node, component), broadcasting coarser scopes.
    pub fn at(&self, name: &str, petal: usize, node: usize, comp: usize) -> Option<f64> {
        let c = self.chans.get(name)?;
        let d = c.spec.dims;
        let comp = if d == 1 { 0 } else { comp };
        Some(match c.spec.scope {
            Scope::Flower => c.values[0][comp],
            Scope::Petal => c.values[petal][comp],
            Scope::Point => c.values[petal][node * d + comp],
        })
    }

    /// Overwrite a channel's values (for consumers of `State` channels).
    pub fn set(&mut self, name: &str, petal: usize, values: &[f64]) {
        if let Some(c) = self.chans.get_mut(name) {
            let slot = if c.spec.scope == Scope::Flower { 0 } else { petal };
            c.values[slot].copy_from_slice(values);
        }
    }

    fn param(&self, p: &Param, petal: usize, node: usize, comp: usize) -> f64 {
        match p {
            Param::Const(x) => *x,
            Param::Channel(n) => self.at(n, petal, node, comp).expect("checked in new"),
        }
    }

    /// Advance every channel from t0 to t1.
    pub fn step(&mut self, t0: f64, t1: f64) {
        let dt = t1 - t0;
        let mut next: BTreeMap<String, (Vec<Vec<f64>>, Vec<Pcg32>)> = BTreeMap::new();
        for (name, c) in &self.chans {
            let d = c.spec.dims;
            let mut values = c.values.clone();
            let mut rngs = c.rngs.clone();
            for (slot, (vals, rng)) in values.iter_mut().zip(rngs.iter_mut()).enumerate() {
                for (i, x) in vals.iter_mut().enumerate() {
                    let (node, comp) = (i / d, i % d);
                    let pr = |p: &Param| self.param(p, slot, node, comp);
                    match &c.spec.rule {
                        Rule::Fixed | Rule::State => {}
                        Rule::Derived { from, gain, bias, power } => *x = pr(gain) * shaped(pr(from), pr(power)) + pr(bias),
                        Rule::Drift { mean, revert, sigma } => {
                            let n = rng.normal();
                            *x += pr(revert) * (pr(mean) - *x) * dt + pr(sigma) * math::sqrt(dt.max(0.0)) * n;
                        }
                        Rule::Growth { target, gain, bias, rate, onset, duration, monotone } => {
                            let (on, du) = (pr(onset), pr(duration));
                            let (s0, s1) = (window(t0, on, du), window(t1, on, du));
                            if s1 > s0 {
                                let f = ((s1 - s0) / (1.0 - s0).max(1e-12) * pr(rate).max(0.0)).min(1.0);
                                let mut inc = (pr(gain) * pr(target) + pr(bias) - *x) * f;
                                if *monotone {
                                    inc = inc.max(0.0);
                                }
                                *x += inc;
                            }
                        }
                    }
                }
            }
            next.insert(name.clone(), (values, rngs));
        }
        for (name, (values, rngs)) in next {
            let c = self.chans.get_mut(&name).expect("same keys");
            c.values = values;
            c.rngs = rngs;
        }
    }
}
