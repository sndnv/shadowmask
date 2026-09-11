use serde::Serialize;

pub fn encode_payload<W: Serialize>(wire: &W) -> String {
    serde_json::to_string(wire).expect("job payload serializes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct Wire {
        version_id: String,
        target_height: u32,
    }

    #[test]
    fn a_payload_encodes_to_its_wire_shape() {
        let encoded = encode_payload(&Wire { version_id: "v1".into(), target_height: 1080 });

        assert_eq!(encoded, r#"{"version_id":"v1","target_height":1080}"#);
    }
}
