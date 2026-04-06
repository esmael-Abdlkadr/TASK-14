use uuid::Uuid;
use web_sys::HtmlInputElement;
use web_sys::HtmlSelectElement;
use web_sys::HtmlTextAreaElement;
use yew::prelude::*;

use crate::services::inspection_api::{
    SubmissionResponse, TaskInstanceDetail, TemplateSubtask, ValidationResult,
};

#[derive(Properties, PartialEq)]
pub struct TaskDetailProps {
    pub task: Option<TaskInstanceDetail>,
    pub is_loading: bool,
    pub is_submitting: bool,
    pub submission_result: Option<SubmissionResponse>,
    pub on_start: Callback<Uuid>,
    pub on_submit: Callback<(Uuid, Option<String>, Vec<serde_json::Value>)>,
    pub on_back: Callback<()>,
}

#[function_component(TaskDetail)]
pub fn task_detail(props: &TaskDetailProps) -> Html {
    let notes = use_state(String::new);
    let responses = use_state(|| std::collections::HashMap::<Uuid, serde_json::Value>::new());

    if props.is_loading {
        return html! { <div class="loading-spinner">{"Loading task..."}</div> };
    }

    let task = match &props.task {
        Some(t) => t,
        None => return html! {},
    };

    let on_back = {
        let cb = props.on_back.clone();
        Callback::from(move |_: MouseEvent| cb.emit(()))
    };

    let on_start = {
        let cb = props.on_start.clone();
        let id = task.instance.id;
        Callback::from(move |_: MouseEvent| cb.emit(id))
    };

    let on_notes_change = {
        let notes = notes.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlTextAreaElement = e.target_unchecked_into();
            notes.set(input.value());
        })
    };

    let can_submit = task.instance.status == "in_progress"
        || task.instance.status == "scheduled"
        || task.instance.status == "makeup";
    let can_start = task.instance.status == "scheduled" || task.instance.status == "makeup";

    let on_submit = {
        let cb = props.on_submit.clone();
        let id = task.instance.id;
        let notes = notes.clone();
        let responses = responses.clone();
        Callback::from(move |_: MouseEvent| {
            let resp_list: Vec<serde_json::Value> = responses
                .iter()
                .map(|(subtask_id, value)| {
                    serde_json::json!({
                        "subtask_id": subtask_id,
                        "response_value": value,
                    })
                })
                .collect();
            let n = if notes.is_empty() {
                None
            } else {
                Some((*notes).clone())
            };
            cb.emit((id, n, resp_list));
        })
    };

    html! {
        <div>
            <button onclick={on_back} class="back-btn">{"< Back to Tasks"}</button>

            <div class="task-detail-card">
                <div class="task-detail-header">
                    <div>
                        <h2>{ &task.template_name }</h2>
                        { if let Some(ref group) = task.template_group {
                            html! { <span class="task-card-group">{ group }</span> }
                        } else { html! {} }}
                    </div>
                    <span class={classes!("task-status-badge",
                        match task.instance.status.as_str() {
                            "scheduled" => "status-scheduled",
                            "in_progress" => "status-progress",
                            "submitted" => "status-submitted",
                            "completed" => "status-completed",
                            "overdue" => "status-overdue",
                            "missed" => "status-missed",
                            "makeup" => "status-makeup",
                            _ => "",
                        }
                    )}>
                        { &task.instance.status }
                    </span>
                </div>

                <div class="task-detail-meta">
                    <p>{ format!("Due: {} | Window: {} - {}",
                        task.instance.due_date, task.instance.window_start, task.instance.window_end) }</p>
                    { if task.instance.is_makeup {
                        html! {
                            <p class="makeup-note">
                                { format!("This is a makeup task. Deadline: {}",
                                    task.instance.makeup_deadline.as_deref().unwrap_or("N/A")) }
                            </p>
                        }
                    } else { html! {} }}
                </div>

                { if can_start && task.instance.status == "scheduled" {
                    html! {
                        <button onclick={on_start} class="btn-primary" style="margin:16px 0;">
                            {"Start Inspection"}
                        </button>
                    }
                } else { html! {} }}

                // Subtask form
                { if can_submit {
                    html! {
                        <div class="subtask-form">
                            <h3>{"Inspection Checklist"}</h3>
                            { for task.subtasks.iter().map(|subtask| {
                                render_subtask_input(subtask, responses.clone())
                            })}

                            <div class="form-field" style="margin-top:16px;">
                                <label>{"Notes (optional)"}</label>
                                <textarea
                                    value={(*notes).clone()}
                                    oninput={on_notes_change.clone()}
                                    placeholder="Additional notes..."
                                    rows="3"
                                />
                            </div>

                            // Validation feedback
                            { if let Some(ref result) = props.submission_result {
                                render_validation_feedback(result)
                            } else { html! {} }}

                            <button
                                onclick={on_submit}
                                class="btn-primary"
                                disabled={props.is_submitting}
                                style="margin-top:16px;"
                            >
                                { if props.is_submitting { "Submitting..." } else { "Submit Inspection" } }
                            </button>
                        </div>
                    }
                } else { html! {} }}

                // Existing submission display
                { if let Some(ref sub) = task.submission {
                    html! {
                        <div class="submission-display">
                            <h3>{"Submission"}</h3>
                            <p>{ format!("Status: {} | Submitted: {}", sub.status, sub.submitted_at) }</p>
                            { if let Some(ref notes) = sub.notes {
                                html! { <p>{ format!("Notes: {}", notes) }</p> }
                            } else { html! {} }}
                            { if let Some(ref review) = sub.review_notes {
                                html! { <p>{ format!("Review: {}", review) }</p> }
                            } else { html! {} }}
                        </div>
                    }
                } else { html! {} }}
            </div>
        </div>
    }
}

fn render_subtask_input(
    subtask: &TemplateSubtask,
    responses: UseStateHandle<std::collections::HashMap<Uuid, serde_json::Value>>,
) -> Html {
    let id = subtask.id;
    let required = subtask.is_required;

    match subtask.expected_type.as_str() {
        "checkbox" => {
            let on_change = {
                let responses = responses.clone();
                Callback::from(move |e: Event| {
                    let input: HtmlInputElement = e.target_unchecked_into();
                    let mut map = (*responses).clone();
                    map.insert(id, serde_json::json!({"checked": input.checked()}));
                    responses.set(map);
                })
            };
            html! {
                <div class="subtask-item">
                    <label class="checkbox-label">
                        <input type="checkbox" onchange={on_change} />
                        <span>{ &subtask.title }</span>
                        { if required { html! { <span class="required">{"*"}</span> } } else { html! {} } }
                    </label>
                    { if let Some(ref desc) = subtask.description {
                        html! { <div class="subtask-desc">{ desc }</div> }
                    } else { html! {} }}
                </div>
            }
        }
        "text" => {
            let on_input = {
                let responses = responses.clone();
                Callback::from(move |e: InputEvent| {
                    let input: HtmlInputElement = e.target_unchecked_into();
                    let mut map = (*responses).clone();
                    map.insert(id, serde_json::json!({"text": input.value()}));
                    responses.set(map);
                })
            };
            html! {
                <div class="subtask-item">
                    <label>
                        { &subtask.title }
                        { if required { html! { <span class="required">{"*"}</span> } } else { html! {} } }
                    </label>
                    <input type="text" oninput={on_input} placeholder="Enter response..." />
                    { if let Some(ref desc) = subtask.description {
                        html! { <div class="subtask-desc">{ desc }</div> }
                    } else { html! {} }}
                </div>
            }
        }
        "number" => {
            let on_input = {
                let responses = responses.clone();
                Callback::from(move |e: InputEvent| {
                    let input: HtmlInputElement = e.target_unchecked_into();
                    if let Ok(num) = input.value().parse::<f64>() {
                        let mut map = (*responses).clone();
                        map.insert(id, serde_json::json!({"number": num}));
                        responses.set(map);
                    }
                })
            };
            html! {
                <div class="subtask-item">
                    <label>
                        { &subtask.title }
                        { if required { html! { <span class="required">{"*"}</span> } } else { html! {} } }
                    </label>
                    <input type="number" oninput={on_input} />
                    { if let Some(ref desc) = subtask.description {
                        html! { <div class="subtask-desc">{ desc }</div> }
                    } else { html! {} }}
                </div>
            }
        }
        "select" => {
            let choices = subtask
                .options
                .as_ref()
                .and_then(|o| o.get("choices"))
                .and_then(|c| c.as_array())
                .cloned()
                .unwrap_or_default();

            let on_change = {
                let responses = responses.clone();
                Callback::from(move |e: Event| {
                    let select: HtmlSelectElement = e.target_unchecked_into();
                    let mut map = (*responses).clone();
                    map.insert(id, serde_json::json!({"selected": select.value()}));
                    responses.set(map);
                })
            };
            html! {
                <div class="subtask-item">
                    <label>
                        { &subtask.title }
                        { if required { html! { <span class="required">{"*"}</span> } } else { html! {} } }
                    </label>
                    <select onchange={on_change}>
                        <option value="">{"-- Select --"}</option>
                        { for choices.iter().filter_map(|c| c.as_str()).map(|c| {
                            html! { <option value={c.to_string()}>{ c }</option> }
                        })}
                    </select>
                    { if let Some(ref desc) = subtask.description {
                        html! { <div class="subtask-desc">{ desc }</div> }
                    } else { html! {} }}
                </div>
            }
        }
        _ => {
            html! {
                <div class="subtask-item">
                    <label>{ &subtask.title }</label>
                    <p class="subtask-desc">{ format!("Unsupported type: {}", subtask.expected_type) }</p>
                </div>
            }
        }
    }
}

fn render_validation_feedback(result: &SubmissionResponse) -> Html {
    let v = &result.validation;

    html! {
        <div class={if result.valid { "validation-success" } else { "validation-errors" }}>
            { if result.valid {
                html! { <p class="validation-ok">{"Submission accepted successfully."}</p> }
            } else {
                html! {
                    <div>
                        <p class="validation-title">{"Please fix the following errors:"}</p>
                        <ul>
                            { for v.errors.iter().map(|e| html! {
                                <li class="validation-error">{ &e.message }</li>
                            })}
                        </ul>
                    </div>
                }
            }}
            { if !v.warnings.is_empty() {
                html! {
                    <div>
                        <ul>
                            { for v.warnings.iter().map(|w| html! {
                                <li class="validation-warning">{ &w.message }</li>
                            })}
                        </ul>
                    </div>
                }
            } else { html! {} }}
        </div>
    }
}
