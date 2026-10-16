//! Stage 2 (flowering): opening a packed bud into a flower.
//!
//! Opening is a Markov chain over the whole flower. Its state is every
//! petal's geometry plus a set of named `channels`. Each step:
//!
//! 1. `channels`: every channel moves by its declared rule (fixed,
//!    mean-reverting drift, or windowed growth toward a target). The
//!    stepper never knows what a channel means.
//! 2. `consume`: the geometry channels become rest lengths, rest curvature
//!    and thickness for each petal.
//! 3. `relax`: the petals relax toward that rest shape against each other.
//!
//! Which channels exist and how they are wired is data (`program`), so new
//! latents such as colour gradients are added without changing 1-3. The
//! chain draws only from the flower's stream, so siblings on a plant open
//! differently from the same bud.

pub mod channels;
pub mod consume;
pub mod contact;
pub mod export;
pub mod program;
pub mod relax;

pub use channels::{ChannelSpec, Channels, Input, Param, Rule, Scope};
pub use relax::RelaxSettings;

use crate::sheet::Sheet;
use crate::Bud;

#[derive(Clone, Debug)]
pub struct BloomSettings {
    /// Opening frames after the bud (t = 1/frames .. 1).
    pub frames: usize,
    /// Chain steps per frame.
    pub substeps: usize,
    pub relax: RelaxSettings,
}

impl Default for BloomSettings {
    fn default() -> Self {
        BloomSettings { frames: 24, substeps: 1, relax: RelaxSettings::default() }
    }
}

/// One opening frame: geometry and channels at time t.
#[derive(Clone)]
pub struct Frame {
    pub t: f64,
    /// Per petal, v-major control grid positions and thickness.
    pub pos: Vec<Vec<[f64; 3]>>,
    pub thick: Vec<Vec<f64>>,
    pub channels: Channels,
    pub energy: f64,
    pub max_overlap: f64,
}

pub struct Opening {
    pub flower_index: u64,
    pub frames: Vec<Frame>,
}

/// Sub-stream of the flower seed for the opening chain (far from the bud's
/// small stream numbers).
pub const STREAM_OPENING: u64 = 0x0B10_0000;

/// The flower's chain seed, from the plant's per-flower stream.
pub fn flower_seed(bud: &Bud, flower_index: u64) -> u64 {
    crate::rng::derive(bud.seed.flower_seed(flower_index), STREAM_OPENING)
}

/// The bud's channels (`Bud::channels`) as opening inputs. Every channel
/// the bud declares comes through by name, whatever it is, plus each
/// petal's `layer_depth` (0 innermost layer, 1 outermost).
pub fn bud_inputs(bud: &Bud) -> Vec<Input> {
    use crate::channels::Scope as S;
    let petals = bud.sheets.len();
    let top = bud.sheets.iter().map(|s| s.layer).max().unwrap_or(0).max(1) as f64;
    let depth = Input {
        name: "layer_depth".into(),
        scope: Scope::Petal,
        dims: 1,
        values: bud.sheets.iter().map(|s| vec![s.layer as f64 / top]).collect(),
        labels: None,
    };
    let mut inputs: Vec<Input> = bud.channels
        .decls
        .iter()
        .map(|d| {
            let v = bud.channels.get(&d.name).unwrap_or(&[]);
            let (scope, values) = match d.scope {
                S::Flower => (Scope::Flower, vec![v.to_vec()]),
                S::Petal => (Scope::Petal, v.chunks(d.dims).map(|c| c.to_vec()).collect()),
                S::Point => {
                    let per = v.len() / petals.max(1);
                    (Scope::Point, v.chunks(per.max(1)).map(|c| c.to_vec()).collect())
                }
            };
            Input { name: d.name.clone(), scope, dims: d.dims, values, labels: d.labels.clone() }
        })
        .collect();
    if !inputs.iter().any(|i| i.name == depth.name) {
        inputs.push(depth);
    }
    inputs
}

/// The bud's petals with their laid-out (pre-packing) shapes.
pub fn bud_grids(bud: &Bud) -> Vec<relax::Grid> {
    bud.sheets
        .iter()
        .map(|sh| {
            let laid = Sheet::from_patch(&bud.layout.petals[sh.petal], &bud.shells, &bud.traits).expect("packed petals came from patches");
            {
                let (nu, nv) = sh.grid();
                relax::Grid { pos: sh.pos.clone(), laid: laid.pos, thick: sh.thick.clone(), nu, nv }
            }
        })
        .collect()
}

/// Open `bud` with the given channel program and inputs.
pub fn open_with(bud: &Bud, specs: &[ChannelSpec], inputs: &[Input], s: &BloomSettings) -> Result<Opening, channels::ChannelError> {
    let grids = bud_grids(bud);
    let nodes: Vec<usize> = grids.iter().map(|g| g.pos.len()).collect();
    let mut ch = Channels::new(specs, inputs, &nodes, flower_seed(bud, bud.flower.index))?;
    let mut flower = relax::Flower::new(&grids, &bud.shells, &s.relax);
    let steps = s.frames * s.substeps.max(1);
    let mut frames = Vec::with_capacity(s.frames);
    for k in 0..steps {
        let (t0, t1) = (k as f64 / steps as f64, (k + 1) as f64 / steps as f64);
        ch.step(t0, t1);
        let targets: Vec<_> = flower.petals.iter().enumerate().map(|(p, pt)| pt.targets(&ch, p)).collect();
        let energy = flower.relax(&targets, &s.relax);
        if (k + 1) % s.substeps.max(1) == 0 {
            let split = |p: usize| flower.offset[p]..flower.offset[p] + nodes[p];
            frames.push(Frame {
                t: t1,
                pos: (0..nodes.len()).map(|p| flower.x[split(p)].to_vec()).collect(),
                thick: (0..nodes.len()).map(|p| flower.thick[split(p)].to_vec()).collect(),
                channels: ch.clone(),
                energy,
                max_overlap: flower.max_overlap(),
            });
        }
    }
    Ok(Opening { flower_index: bud.flower.index, frames })
}

/// Open `bud` with the default program and the bud's latents.
pub fn open_flower(bud: &Bud, s: &BloomSettings) -> Opening {
    open_with(bud, &program::default_program(&bud.species), &bud_inputs(bud), s).expect("default program is well formed")
}
