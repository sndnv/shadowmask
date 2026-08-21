use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PeopleBatchRequest {
    pub people: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_a_people_list() {
        let req: PeopleBatchRequest = serde_json::from_str(r#"{"people":["p1","p2"]}"#).unwrap();
        assert_eq!(req.people, ["p1", "p2"]);
    }

    #[test]
    fn an_empty_list_is_accepted() {
        let req: PeopleBatchRequest = serde_json::from_str(r#"{"people":[]}"#).unwrap();
        assert!(req.people.is_empty());
    }
}
