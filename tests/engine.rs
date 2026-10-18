//! Tests for `rm2k::engine` - the engine enum and its per-engine selection helpers.

use rm2k::Engine;

#[test]
fn the_default_engine_is_r2k() {
    assert_eq!(Engine::default(), Engine::R2K);
}
