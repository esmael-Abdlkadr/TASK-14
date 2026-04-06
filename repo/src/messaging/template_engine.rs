use std::collections::HashMap;

use crate::errors::AppError;
use crate::models::TemplateVariable;

/// Render a template string by replacing `{{variable}}` placeholders with values.
/// Variables are provided as a HashMap<String, String>.
/// Missing required variables cause an error; optional ones use defaults.
pub fn render_template(
    template: &str,
    variables: &HashMap<String, String>,
    variable_defs: &[TemplateVariable],
) -> Result<String, AppError> {
    let mut result = template.to_string();

    // Validate required variables are present
    for def in variable_defs {
        if def.is_required && !variables.contains_key(&def.var_name) && def.default_value.is_none() {
            return Err(AppError::BadRequest(format!(
                "Required template variable '{}' is missing", def.var_name
            )));
        }
    }

    // Replace all {{variable}} placeholders
    for def in variable_defs {
        let placeholder = format!("{{{{{}}}}}", def.var_name);
        let value = variables
            .get(&def.var_name)
            .cloned()
            .or_else(|| def.default_value.clone())
            .unwrap_or_default();

        // Format based on type
        let formatted = format_variable(&value, &def.var_type);
        result = result.replace(&placeholder, &formatted);
    }

    // Also replace any ad-hoc {{key}} from the variables map not in defs
    for (key, value) in variables {
        let placeholder = format!("{{{{{}}}}}", key);
        if result.contains(&placeholder) {
            result = result.replace(&placeholder, value);
        }
    }

    Ok(result)
}

/// Format a variable value based on its declared type
fn format_variable(value: &str, var_type: &str) -> String {
    match var_type {
        "date" => {
            // Try to parse and reformat dates nicely
            if let Ok(d) = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d") {
                d.format("%B %d, %Y").to_string()
            } else {
                value.to_string()
            }
        }
        "number" => {
            if let Ok(n) = value.parse::<f64>() {
                if n == n.floor() {
                    format!("{}", n as i64)
                } else {
                    format!("{:.2}", n)
                }
            } else {
                value.to_string()
            }
        }
        _ => value.to_string(),
    }
}

/// Extract all variable placeholders from a template string
pub fn extract_placeholders(template: &str) -> Vec<String> {
    let mut vars = Vec::new();
    let mut remaining = template;

    while let Some(start) = remaining.find("{{") {
        if let Some(end) = remaining[start..].find("}}") {
            let var_name = &remaining[start + 2..start + end];
            let var_name = var_name.trim().to_string();
            if !var_name.is_empty() && !vars.contains(&var_name) {
                vars.push(var_name);
            }
            remaining = &remaining[start + end + 2..];
        } else {
            break;
        }
    }

    vars
}

/// Build a variables map from a JSON event payload
pub fn payload_to_variables(payload: &serde_json::Value) -> HashMap<String, String> {
    let mut map = HashMap::new();

    if let Some(obj) = payload.as_object() {
        for (key, value) in obj {
            let str_val = match value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Null => String::new(),
                _ => value.to_string(),
            };
            map.insert(key.clone(), str_val);
        }
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use chrono::Utc;

    fn make_var(name: &str, var_type: &str, required: bool, default: Option<&str>) -> TemplateVariable {
        TemplateVariable {
            id: Uuid::new_v4(),
            template_id: Uuid::new_v4(),
            var_name: name.to_string(),
            var_type: var_type.to_string(),
            description: None,
            default_value: default.map(String::from),
            is_required: required,
        }
    }

    #[test]
    fn test_basic_render() {
        let template = "Hello {{user_name}}, your task {{task_name}} is due on {{due_date}}.";
        let defs = vec![
            make_var("user_name", "string", true, None),
            make_var("task_name", "string", true, None),
            make_var("due_date", "date", true, None),
        ];
        let mut vars = HashMap::new();
        vars.insert("user_name".into(), "Alice".into());
        vars.insert("task_name".into(), "Dumpster Inspection".into());
        vars.insert("due_date".into(), "2026-04-15".into());

        let result = render_template(template, &vars, &defs).unwrap();
        assert_eq!(result, "Hello Alice, your task Dumpster Inspection is due on April 15, 2026.");
    }

    #[test]
    fn test_default_value() {
        let template = "Status: {{status}}";
        let defs = vec![make_var("status", "string", false, Some("pending"))];
        let vars = HashMap::new();
        let result = render_template(template, &vars, &defs).unwrap();
        assert_eq!(result, "Status: pending");
    }

    #[test]
    fn test_missing_required() {
        let template = "Hello {{user_name}}";
        let defs = vec![make_var("user_name", "string", true, None)];
        let vars = HashMap::new();
        let result = render_template(template, &vars, &defs);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_placeholders() {
        let template = "{{greeting}} {{user_name}}, task {{task_id}} is {{status}}.";
        let vars = extract_placeholders(template);
        assert_eq!(vars, vec!["greeting", "user_name", "task_id", "status"]);
    }

    #[test]
    fn test_payload_to_variables() {
        let payload = serde_json::json!({
            "user_name": "Bob",
            "count": 42,
            "active": true,
        });
        let vars = payload_to_variables(&payload);
        assert_eq!(vars.get("user_name").unwrap(), "Bob");
        assert_eq!(vars.get("count").unwrap(), "42");
        assert_eq!(vars.get("active").unwrap(), "true");
    }

    #[test]
    fn test_number_formatting() {
        let template = "Score: {{score}}";
        let defs = vec![make_var("score", "number", true, None)];
        let mut vars = HashMap::new();
        vars.insert("score".into(), "4.50".into());
        let result = render_template(template, &vars, &defs).unwrap();
        assert_eq!(result, "Score: 4.50");
    }
}
