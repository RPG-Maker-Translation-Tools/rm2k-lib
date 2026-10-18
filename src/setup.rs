//! `RPG_RT`'s post-load fixups, as an explicit opt-in.
//!
//! Three `Actor` fields (see the [schema docs](crate::schema) for why the schema stores `-1`
//! for them rather than a real value) get resolved to the engine's actual default by
//! [`Setup::setup`], which then also pads the actor's six stat curves out to the engine's
//! maximum level.
//!
//! Not automatic: padding the curves **changes the file**. `Parameters` is `persist`, so it is
//! always written back at whatever length padding left it. Running this on load would make
//! load/save non-idempotent.
//!
//! Byte-exactness is this crate's bar, so the fixup is separate. Call it for values a game engine
//! can run on; skip it when editing files and writing back exactly what you read.
//!
//! ```
//! # use rm2k::{rpg::Database, setup::Setup, Engine, LcfStruct};
//! # let mut db = Database::defaults(Engine::R2K);
//! // Reading a game: resolve the sentinels first.
//! db.setup(Engine::R2K);
//! ```

use crate::{
    engine::Engine,
    rpg::{Actor, Database},
};

/// Apply `RPG_RT`'s post-load fixups.
pub trait Setup {
    /// Resolve sentinel defaults and pad derived data for `engine`.
    fn setup(&mut self, engine: Engine);
}

/// The highest level an actor can reach, per engine.
///
/// It is both the substitute for a `-1` final level and the length the stat curves
/// are padded to - note the curves use *this*, not the actor's own final level,
/// even when the actor's is lower.
#[must_use]
#[inline]
pub const fn max_final_level(engine: Engine) -> i32 {
    engine.pick(50, 99)
}

/// The default experience curve constants, per engine.
#[must_use]
#[inline]
const fn default_exp(engine: Engine) -> i32 {
    engine.pick(30, 300)
}

impl Setup for Actor<'_> {
    fn setup(&mut self, engine: Engine) {
        let max_level = max_final_level(engine);

        if self.final_level == -1 {
            self.final_level = max_level;
        }
        if self.exp_base == -1 {
            self.exp_base = default_exp(engine);
        }
        if self.exp_inflation == -1 {
            self.exp_inflation = default_exp(engine);
        }

        pad_parameters(&mut self.parameters, max_level);
    }
}

/// Pad an actor's six stat curves out to `levels` entries.
///
/// Only ever grows, and each curve has its own fill value. A curve that
/// is already long enough is untouched, so this is idempotent.
fn pad_parameters(parameters: &mut crate::raw::Parameters, levels: i32) {
    let levels = if levels > 0 { levels as usize } else { 0 };
    for (curve, fill) in [
        (&mut parameters.maxhp, 1i16),
        (&mut parameters.maxsp, 0),
        (&mut parameters.attack, 1),
        (&mut parameters.defense, 1),
        (&mut parameters.spirit, 1),
        (&mut parameters.agility, 1),
    ] {
        if curve.0.len() < levels {
            curve.0.resize(levels, fill);
        }
    }
}

impl Setup for Database<'_> {
    /// Run [`Actor::setup`](Setup::setup) over every actor.
    fn setup(&mut self, engine: Engine) {
        for actor in self.actors.iter_mut() {
            actor.setup(engine);
        }
    }
}
