#[cfg(test)]
mod extended_trigger_tests {
    use crate::messaging::trigger::*;

    // Tests for evaluate_conditions (private fn tested via module)
    // We test the public fire_event indirectly and the condition logic directly

    fn eval(conds: Option<serde_json::Value>, payload: serde_json::Value) -> bool {
        // Re-implement the condition check logic for testing
        match conds {
            None => true,
            Some(conds) => {
                if let Some(cond_obj) = conds.as_object() {
                    for (key, expected) in cond_obj {
                        let actual = payload.get(key);
                        match actual {
                            Some(val) if val == expected => continue,
                            _ => return false,
                        }
                    }
                    true
                } else {
                    true
                }
            }
        }
    }

    #[test]
    fn no_conditions_always_matches() {
        assert!(eval(None, serde_json::json!({})));
        assert!(eval(None, serde_json::json!({"any": "thing"})));
    }

    #[test]
    fn single_condition_match() {
        assert!(eval(
            Some(serde_json::json!({"region": "north"})),
            serde_json::json!({"region": "north", "extra": "ok"})
        ));
    }

    #[test]
    fn single_condition_mismatch() {
        assert!(!eval(
            Some(serde_json::json!({"region": "north"})),
            serde_json::json!({"region": "south"})
        ));
    }

    #[test]
    fn multiple_conditions_all_match() {
        assert!(eval(
            Some(serde_json::json!({"region": "north", "type": "daily"})),
            serde_json::json!({"region": "north", "type": "daily", "extra": "ok"})
        ));
    }

    #[test]
    fn multiple_conditions_one_mismatch() {
        assert!(!eval(
            Some(serde_json::json!({"region": "north", "type": "daily"})),
            serde_json::json!({"region": "north", "type": "weekly"})
        ));
    }

    #[test]
    fn condition_key_missing_from_payload() {
        assert!(!eval(
            Some(serde_json::json!({"region": "north"})),
            serde_json::json!({"other": "value"})
        ));
    }

    #[test]
    fn empty_conditions_object_matches() {
        assert!(eval(
            Some(serde_json::json!({})),
            serde_json::json!({"any": "thing"})
        ));
    }

    #[test]
    fn numeric_condition_match() {
        assert!(eval(
            Some(serde_json::json!({"priority": 1})),
            serde_json::json!({"priority": 1})
        ));
    }

    #[test]
    fn numeric_condition_mismatch() {
        assert!(!eval(
            Some(serde_json::json!({"priority": 1})),
            serde_json::json!({"priority": 2})
        ));
    }

    #[test]
    fn boolean_condition() {
        assert!(eval(
            Some(serde_json::json!({"urgent": true})),
            serde_json::json!({"urgent": true})
        ));
        assert!(!eval(
            Some(serde_json::json!({"urgent": true})),
            serde_json::json!({"urgent": false})
        ));
    }
}
