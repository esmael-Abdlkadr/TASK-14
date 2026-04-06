#[cfg(test)]
mod extended_template_tests {
    use crate::messaging::template_engine::*;
    use crate::models::TemplateVariable;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn var(name: &str, vtype: &str, required: bool, default: Option<&str>) -> TemplateVariable {
        TemplateVariable {
            id: Uuid::new_v4(), template_id: Uuid::new_v4(),
            var_name: name.into(), var_type: vtype.into(),
            description: None, default_value: default.map(String::from),
            is_required: required,
        }
    }

    // ── Basic rendering ─────────────────────────────────────

    #[test]
    fn render_multiple_variables() {
        let t = "Dear {{name}}, your task {{task}} is due.";
        let defs = vec![var("name", "string", true, None), var("task", "string", true, None)];
        let mut vars = HashMap::new();
        vars.insert("name".into(), "Alice".into());
        vars.insert("task".into(), "Inspection #5".into());
        let r = render_template(t, &vars, &defs).unwrap();
        assert_eq!(r, "Dear Alice, your task Inspection #5 is due.");
    }

    #[test]
    fn render_repeated_variable() {
        let t = "{{x}} and {{x}} again";
        let defs = vec![var("x", "string", true, None)];
        let mut vars = HashMap::new();
        vars.insert("x".into(), "hello".into());
        let r = render_template(t, &vars, &defs).unwrap();
        assert_eq!(r, "hello and hello again");
    }

    #[test]
    fn render_no_placeholders() {
        let t = "Static text with no variables.";
        let r = render_template(t, &HashMap::new(), &[]).unwrap();
        assert_eq!(r, "Static text with no variables.");
    }

    // ── Date formatting ─────────────────────────────────────

    #[test]
    fn date_formatted_correctly() {
        let defs = vec![var("date", "date", true, None)];
        let mut vars = HashMap::new();
        vars.insert("date".into(), "2026-04-15".into());
        let r = render_template("Due: {{date}}", &vars, &defs).unwrap();
        assert_eq!(r, "Due: April 15, 2026");
    }

    #[test]
    fn date_invalid_passthrough() {
        let defs = vec![var("date", "date", true, None)];
        let mut vars = HashMap::new();
        vars.insert("date".into(), "not-a-date".into());
        let r = render_template("Due: {{date}}", &vars, &defs).unwrap();
        assert_eq!(r, "Due: not-a-date");
    }

    // ── Number formatting ───────────────────────────────────

    #[test]
    fn number_integer_no_decimals() {
        let defs = vec![var("count", "number", true, None)];
        let mut vars = HashMap::new();
        vars.insert("count".into(), "42.0".into());
        let r = render_template("Count: {{count}}", &vars, &defs).unwrap();
        assert_eq!(r, "Count: 42");
    }

    #[test]
    fn number_decimal_two_places() {
        let defs = vec![var("score", "number", true, None)];
        let mut vars = HashMap::new();
        vars.insert("score".into(), "3.14159".into());
        let r = render_template("Score: {{score}}", &vars, &defs).unwrap();
        assert_eq!(r, "Score: 3.14");
    }

    // ── Default values ──────────────────────────────────────

    #[test]
    fn optional_uses_default() {
        let defs = vec![var("greeting", "string", false, Some("Hello"))];
        let r = render_template("{{greeting}} world", &HashMap::new(), &defs).unwrap();
        assert_eq!(r, "Hello world");
    }

    #[test]
    fn provided_value_overrides_default() {
        let defs = vec![var("greeting", "string", false, Some("Hello"))];
        let mut vars = HashMap::new();
        vars.insert("greeting".into(), "Hi".into());
        let r = render_template("{{greeting}} world", &vars, &defs).unwrap();
        assert_eq!(r, "Hi world");
    }

    // ── Required validation ─────────────────────────────────

    #[test]
    fn required_missing_errors() {
        let defs = vec![var("name", "string", true, None)];
        let result = render_template("Hi {{name}}", &HashMap::new(), &defs);
        assert!(result.is_err());
    }

    #[test]
    fn required_with_default_ok() {
        let defs = vec![var("name", "string", true, Some("World"))];
        let result = render_template("Hi {{name}}", &HashMap::new(), &defs);
        assert!(result.is_ok());
    }

    // ── Extract placeholders ────────────────────────────────

    #[test]
    fn extract_multiple() {
        let v = extract_placeholders("{{a}} text {{b}} more {{c}}");
        assert_eq!(v, vec!["a", "b", "c"]);
    }

    #[test]
    fn extract_duplicates_deduped() {
        let v = extract_placeholders("{{x}} and {{x}}");
        assert_eq!(v, vec!["x"]);
    }

    #[test]
    fn extract_none() {
        let v = extract_placeholders("no variables here");
        assert!(v.is_empty());
    }

    #[test]
    fn extract_unclosed_brace() {
        let v = extract_placeholders("{{open but no close");
        assert!(v.is_empty());
    }

    #[test]
    fn extract_empty_placeholder_skipped() {
        let v = extract_placeholders("{{}} should be skipped");
        assert!(v.is_empty());
    }

    // ── Payload to variables ────────────────────────────────

    #[test]
    fn payload_string_values() {
        let p = serde_json::json!({"name": "Alice"});
        let v = payload_to_variables(&p);
        assert_eq!(v.get("name").unwrap(), "Alice");
    }

    #[test]
    fn payload_number_values() {
        let p = serde_json::json!({"count": 42});
        let v = payload_to_variables(&p);
        assert_eq!(v.get("count").unwrap(), "42");
    }

    #[test]
    fn payload_bool_values() {
        let p = serde_json::json!({"active": true});
        let v = payload_to_variables(&p);
        assert_eq!(v.get("active").unwrap(), "true");
    }

    #[test]
    fn payload_null_empty_string() {
        let p = serde_json::json!({"field": null});
        let v = payload_to_variables(&p);
        assert_eq!(v.get("field").unwrap(), "");
    }

    #[test]
    fn payload_non_object_empty() {
        let p = serde_json::json!("just a string");
        let v = payload_to_variables(&p);
        assert!(v.is_empty());
    }
}
