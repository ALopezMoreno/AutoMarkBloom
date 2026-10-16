"""Seeds and plant-level traits.

A plant is identified by one integer seed. Everything Phase 1 needs is drawn
from it into an explicit PlantTraits vector, so later inheritance can work on
named numbers (mutate, cross over) instead of on an opaque RNG state.
Each flower's seed is derived from (plant seed, flower index), so siblings are
reproducible but independent.
"""
from dataclasses import dataclass, asdict
import numpy as np


@dataclass(frozen=True)
class PlantTraits:
    center_rings: int        # rings reserved for stamens/carpels
    inhibit_sigma: float     # reach of a primordium's inhibition field (ring units)
    inhibit_threshold: float # a cell may initiate only below this inhibition
    init_prob: float         # chance an uninhibited empty cell initiates
    continue_prob: float     # chance a petal cell continues into the next ring
    extent_min: int          # petal radial extent range, in rings
    extent_max: int
    lateral_prob: float      # chance an empty cell is claimed by a neighbor petal
    lateral_sweeps: int
    ruffle: float            # edge ruffle amplitude (render only)

    def as_dict(self):
        return asdict(self)


@dataclass(frozen=True)
class PlantSeed:
    value: int

    def plant_rng(self) -> np.random.Generator:
        return np.random.default_rng(np.random.SeedSequence(self.value))

    def flower_rng(self, index: int) -> np.random.Generator:
        return np.random.default_rng(np.random.SeedSequence(self.value, spawn_key=(index + 1,)))

    def traits(self) -> PlantTraits:
        # Separate stream from the CA's stream so changing trait draws does not
        # shift the CA's random sequence.
        rng = np.random.default_rng(np.random.SeedSequence(self.value, spawn_key=(0,)))
        emin = int(rng.integers(4, 6))
        return PlantTraits(
            center_rings=int(rng.integers(3, 6)),
            inhibit_sigma=float(rng.uniform(2.2, 3.0)),
            inhibit_threshold=float(rng.uniform(0.35, 0.6)),
            init_prob=float(rng.uniform(0.15, 0.4)),
            continue_prob=float(rng.uniform(0.82, 0.95)),
            extent_min=emin,
            extent_max=emin + int(rng.integers(3, 6)),
            lateral_prob=float(rng.uniform(0.5, 0.85)),
            lateral_sweeps=int(rng.integers(3, 7)),
            ruffle=float(rng.uniform(0.025, 0.05)),
        )
