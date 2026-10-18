//! A loaded value serializes straight to a `JsValue` within the same call, so the borrow never
//! outlives it. `serde_wasm_bindgen::from_value` never hands out borrowed data (a `JsValue` has no
//! linear-memory slice to borrow), so every `DbStr` becomes owned bytes. Unlike the C API, this
//! crate needs no `IntoOwned`/handle machinery.
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use rm2k::Diagnostic;
use wasm_bindgen::prelude::*;

fn to_js_error(error: impl core::fmt::Display) -> JsError {
    JsError::new(&error.to_string())
}

/// JSON-shaped mirror of [`rm2k::Diagnostic`], local to this crate because the core type
/// deliberately doesn't derive `Serialize` (it lives below the `alloc`/`serde` split).
#[derive(serde::Serialize)]
struct WasmDiagnostic {
    offset: u32,
    #[serde(rename = "chunkId")]
    chunk_id: u32,
    #[serde(rename = "structName")]
    struct_name: &'static str,
    kind: &'static str,
}

impl From<Diagnostic> for WasmDiagnostic {
    fn from(diag: Diagnostic) -> Self {
        Self {
            offset: diag.offset,
            chunk_id: diag.chunk_id,
            struct_name: diag.struct_name,
            kind: diag.kind.as_str(),
        }
    }
}

fn diagnostics_to_js(diagnostics: Vec<Diagnostic>) -> Result<JsValue, JsError> {
    let mirrored: Vec<WasmDiagnostic> = diagnostics.into_iter().map(Into::into).collect();
    serde_wasm_bindgen::to_value(&mirrored).map_err(to_js_error)
}

/// What a `loadX` call returns: the parsed value, the file's original header bytes, and every
/// recoverable problem found while parsing.
#[wasm_bindgen(getter_with_clone)]
pub struct LoadResult {
    pub value: JsValue,
    pub header: Vec<u8>,
    pub diagnostics: JsValue,
}

/// Which RPG Maker a file targets, mirroring [`rm2k::Engine`].
#[wasm_bindgen]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Engine {
    R2K,
    R2K3,
}

impl From<Engine> for rm2k::Engine {
    fn from(engine: Engine) -> Self {
        match engine {
            Engine::R2K => Self::R2K,
            Engine::R2K3 => Self::R2K3,
        }
    }
}

impl From<rm2k::Engine> for Engine {
    fn from(engine: rm2k::Engine) -> Self {
        match engine {
            rm2k::Engine::R2K => Self::R2K,
            rm2k::Engine::R2K3 => Self::R2K3,
        }
    }
}

/// Classifies a raw `system.ldb_id` value: `2003` is 2003, anything else is 2000.
#[wasm_bindgen(js_name = engineFromLdbId)]
#[must_use]
pub fn engine_from_ldb_id(ldb_id: i32) -> Engine {
    rm2k::Engine::from_ldb_id(ldb_id).into()
}

/// Loads an `RPG_RT.ldb` database from its bytes. Recoverable problems are returned in `diagnostics`.
///
/// # Errors
///
/// Fails when `bytes` is too short to hold the file's header.
#[wasm_bindgen(js_name = loadDatabase)]
pub fn load_database(bytes: &[u8]) -> Result<LoadResult, JsError> {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let loaded = rm2k::file::load_database_with(bytes, &mut diagnostics).map_err(to_js_error)?;
    let value = serde_wasm_bindgen::to_value(&loaded.value).map_err(to_js_error)?;
    Ok(LoadResult {
        value,
        header: loaded.header.to_vec(),
        diagnostics: diagnostics_to_js(diagnostics)?,
    })
}

/// Writes a database value, as `loadDatabase` returns it, back to bytes.
///
/// # Errors
///
/// Fails when `value` does not deserialize into a database.
#[wasm_bindgen(js_name = saveDatabase)]
pub fn save_database(value: JsValue, engine: Engine, preserve_header: bool, header: &[u8]) -> Result<Vec<u8>, JsError> {
    let value: rm2k::rpg::Database<'static> = serde_wasm_bindgen::from_value(value).map_err(to_js_error)?;
    let opt = rm2k::SaveOpt { preserve_header };
    Ok(rm2k::file::to_vec(|out| {
        rm2k::file::save_database(&value, out, engine.into(), opt, header)
    }))
}

/// Loads a `MapNNNN.lmu` map from its bytes. Recoverable problems are returned in `diagnostics`.
///
/// # Errors
///
/// Fails when `bytes` is too short to hold the file's header.
#[wasm_bindgen(js_name = loadMap)]
pub fn load_map(bytes: &[u8]) -> Result<LoadResult, JsError> {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let loaded = rm2k::file::load_map_with(bytes, &mut diagnostics).map_err(to_js_error)?;
    let value = serde_wasm_bindgen::to_value(&loaded.value).map_err(to_js_error)?;
    Ok(LoadResult {
        value,
        header: loaded.header.to_vec(),
        diagnostics: diagnostics_to_js(diagnostics)?,
    })
}

/// Writes a map value, as `loadMap` returns it, back to bytes.
///
/// # Errors
///
/// Fails when `value` does not deserialize into a map.
#[wasm_bindgen(js_name = saveMap)]
pub fn save_map(value: JsValue, engine: Engine, preserve_header: bool, header: &[u8]) -> Result<Vec<u8>, JsError> {
    let value: rm2k::rpg::Map<'static> = serde_wasm_bindgen::from_value(value).map_err(to_js_error)?;
    let opt = rm2k::SaveOpt { preserve_header };
    Ok(rm2k::file::to_vec(|out| {
        rm2k::file::save_map(&value, out, engine.into(), opt, header)
    }))
}

/// Loads an `LSD` save from its bytes. Recoverable problems are returned in `diagnostics`.
///
/// # Errors
///
/// Fails when `bytes` is too short to hold the file's header.
#[wasm_bindgen(js_name = loadSave)]
pub fn load_save(bytes: &[u8]) -> Result<LoadResult, JsError> {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let loaded = rm2k::file::load_save_with(bytes, &mut diagnostics).map_err(to_js_error)?;
    let value = serde_wasm_bindgen::to_value(&loaded.value).map_err(to_js_error)?;
    Ok(LoadResult {
        value,
        header: loaded.header.to_vec(),
        diagnostics: diagnostics_to_js(diagnostics)?,
    })
}

/// Writes a save value, as `loadSave` returns it, back to bytes.
///
/// # Errors
///
/// Fails when `value` does not deserialize into a save.
#[wasm_bindgen(js_name = saveSave)]
pub fn save_save(value: JsValue, engine: Engine, preserve_header: bool, header: &[u8]) -> Result<Vec<u8>, JsError> {
    let value: rm2k::rpg::Save<'static> = serde_wasm_bindgen::from_value(value).map_err(to_js_error)?;
    let opt = rm2k::SaveOpt { preserve_header };
    Ok(rm2k::file::to_vec(|out| {
        rm2k::file::save_save(&value, out, engine.into(), opt, header)
    }))
}

/// Loads an `LMT` map tree from its bytes. Recoverable problems are returned in `diagnostics`.
///
/// # Errors
///
/// Fails when `bytes` is too short to hold the file's header.
#[wasm_bindgen(js_name = loadTreeMap)]
pub fn load_tree_map(bytes: &[u8]) -> Result<LoadResult, JsError> {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let loaded = rm2k::file::load_tree_map_with(bytes, &mut diagnostics).map_err(to_js_error)?;
    let value = serde_wasm_bindgen::to_value(&loaded.value).map_err(to_js_error)?;
    Ok(LoadResult {
        value,
        header: loaded.header.to_vec(),
        diagnostics: diagnostics_to_js(diagnostics)?,
    })
}

/// Writes a map tree value, as `loadTreeMap` returns it, back to bytes.
///
/// # Errors
///
/// Fails when `value` does not deserialize into a map tree.
#[wasm_bindgen(js_name = saveTreeMap)]
pub fn save_tree_map(value: JsValue, engine: Engine, preserve_header: bool, header: &[u8]) -> Result<Vec<u8>, JsError> {
    let value: rm2k::rpg::TreeMap<'static> = serde_wasm_bindgen::from_value(value).map_err(to_js_error)?;
    let opt = rm2k::SaveOpt { preserve_header };
    Ok(rm2k::file::to_vec(|out| {
        rm2k::file::save_tree_map(&value, out, engine.into(), opt, header)
    }))
}
