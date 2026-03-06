use serde_json::{Map, Value};

pub(crate) fn map_is_empty(map: &Map<String, Value>) -> bool {
    map.is_empty()
}
