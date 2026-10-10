//! JSON Schema of [`Config`], committed as `schema/config.schema.json` so editors can check a
//! config file while it is written.

use crate::{Config, Error};

/// Pretty JSON Schema of [`Config`], with a trailing newline.
///
/// # Errors
/// [`Error::Schema`] when the schema cannot be serialized.
pub fn schema_json() -> Result<String, Error> {
    let schema = schemars::schema_for!(Config);
    let mut json = serde_json::to_string_pretty(&schema)?;
    json.push('\n');
    Ok(json)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::schema_json;

    const COMMITTED: &str = include_str!("../schema/config.schema.json");

    /// Set to `1` to rewrite the committed schema instead of failing.
    const BLESS: &str = "BLESS";

    #[test]
    fn the_committed_schema_matches_the_types() {
        let generated = schema_json().unwrap();
        if std::env::var(BLESS).is_ok_and(|value| value == "1") {
            let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("schema/config.schema.json");
            std::fs::write(path, &generated).unwrap();
            return;
        }
        assert!(
            generated == COMMITTED,
            "schema/config.schema.json is stale; run `BLESS=1 cargo nextest run -p mailune-config`"
        );
    }

    #[test]
    fn the_schema_rejects_unknown_keys() {
        let schema: serde_json::Value = serde_json::from_str(COMMITTED).unwrap();
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(schema["$defs"]["Account"]["additionalProperties"], false);
    }
}
