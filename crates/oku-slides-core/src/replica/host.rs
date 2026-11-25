//! Option of a `host` param: a `LoroMap` of opaque maps and LWW values.
//!
//! Chart and table options name deck data (`data.rs`); they carry no rows. A `dataset` an older
//! writer stored inline is dropped on read and reported (`read.rs`).

use loro::LoroMap;
use serde_json::{Map, Value};
use snafu::ResultExt;

use super::schema::Ctx;
use super::write::{Res, Writer, json_to_loro, map_at, set_value};
use crate::error::LoroSnafu;

impl Writer<'_> {
    /// Brings the option `map` of a `host` param to the state of `obj`.
    pub(super) fn sync_host(&self, map: &LoroMap, obj: &Map<String, Value>) -> Res {
        let stale: Vec<String> = map.keys().map(|k| k.to_string()).filter(|k| !obj.contains_key(k)).collect();
        for key in stale {
            map.delete(&key).context(LoroSnafu)?;
        }
        for (key, value) in obj {
            match value {
                Value::Object(o) => self.sync_map(&map_at(map, key)?, o, Ctx::Opaque)?,
                v => set_value(map, key, json_to_loro(v))?,
            }
        }
        Ok(())
    }
}
