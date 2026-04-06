#[cfg(test)]
mod payload_lifecycle_tests {
    use crate::messaging::template_engine::*;
    use crate::models::TemplateVariable;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn var(name: &str, required: bool) -> TemplateVariable {
        TemplateVariable {
            id: Uuid::new_v4(), template_id: Uuid::new_v4(),
            var_name: name.into(), var_type: "string".into(),
            description: None, default_value: None,
            is_required: required,
        }
    }

    // G.9 - Payload content rendering for different channels

    #[test]
    fn sms_template_renders_short_body() {
        let tmpl = "Overdue: {{task}}";
        let defs = vec![var("task", true)];
        let mut vars = HashMap::new();
        vars.insert("task".into(), "Dumpster Check".into());
        let result = render_template(tmpl, &vars, &defs).unwrap();
        assert_eq!(result, "Overdue: Dumpster Check");
        assert!(result.len() < 160); // SMS length limit awareness
    }

    #[test]
    fn email_template_renders_full_html() {
        let tmpl = "<h1>Hello {{name}}</h1><p>Your task {{task}} is overdue.</p>";
        let defs = vec![var("name", true), var("task", true)];
        let mut vars = HashMap::new();
        vars.insert("name".into(), "Alice".into());
        vars.insert("task".into(), "Area Inspection".into());
        let result = render_template(tmpl, &vars, &defs).unwrap();
        assert!(result.contains("<h1>Hello Alice</h1>"));
    }

    // G.9 - Template with missing optional variable renders with empty string
    #[test]
    fn optional_missing_renders_empty() {
        let tmpl = "Task: {{task}} Note: {{note}}";
        let defs = vec![
            var("task", true),
            {
                let mut v = var("note", false);
                v.default_value = None;
                v
            },
        ];
        let mut vars = HashMap::new();
        vars.insert("task".into(), "Check".into());
        let result = render_template(tmpl, &vars, &defs).unwrap();
        assert_eq!(result, "Task: Check Note: ");
    }

    // G.9 - Payload with special characters in variables
    #[test]
    fn special_chars_in_variables_preserved() {
        let tmpl = "Note: {{note}}";
        let defs = vec![var("note", true)];
        let mut vars = HashMap::new();
        vars.insert("note".into(), "Contains <html> & \"quotes\"".into());
        let result = render_template(tmpl, &vars, &defs).unwrap();
        assert!(result.contains("<html>"));
        assert!(result.contains("&"));
    }

    // G.9 - Multiple events with different payloads produce different outputs
    #[test]
    fn different_payloads_produce_different_output() {
        let tmpl = "User: {{user}}";
        let defs = vec![var("user", true)];

        let mut v1 = HashMap::new();
        v1.insert("user".into(), "Alice".into());
        let r1 = render_template(tmpl, &v1, &defs).unwrap();

        let mut v2 = HashMap::new();
        v2.insert("user".into(), "Bob".into());
        let r2 = render_template(tmpl, &v2, &defs).unwrap();

        assert_ne!(r1, r2);
    }
}
