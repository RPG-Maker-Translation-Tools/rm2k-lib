//! The RPG Maker 2000/2003 data model.
//!
//! One module per file format, plus the shared enumerations and flag bundles. The
//! modules re-export into this one because the formats reference each other's types
//! freely - an `LSD` save embeds `LMU` move routes, for instance.
//!

pub mod enums;
pub mod flags;
pub mod ldb;
pub mod lmt;
pub mod lmu;
pub mod lsd;

mod treemap;
pub use flags::*;
pub use ldb::*;
pub use lmt::*;
pub use lmu::*;
pub use lsd::*;
pub use treemap::TreeMap;
