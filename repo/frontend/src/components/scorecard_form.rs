use std::collections::HashMap;
use uuid::Uuid;
use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};
use yew::prelude::*;

use crate::services::review_api::{
    AssignmentDetail, ConsistencyCheckOutput, ScorecardDimension, SubmitReviewResponse,
};

#[derive(Properties, PartialEq)]
pub struct ScorecardFormProps {
    pub detail: AssignmentDetail,
    pub is_submitting: bool,
    pub submit_result: Option<SubmitReviewResponse>,
    pub on_submit: Callback<(Vec<serde_json::Value>, Option<String>, String, bool)>,
    pub on_recuse: Callback<String>,
    pub on_back: Callback<()>,
}

#[function_component(ScorecardForm)]
pub fn scorecard_form(props: &ScorecardFormProps) -> Html {
    let ratings = use_state(HashMap::<Uuid, i32>::new);
    let comments = use_state(HashMap::<Uuid, String>::new);
    let overall_comment = use_state(String::new);
    let recommendation = use_state(|| "approve".to_string());
    let recuse_reason = use_state(String::new);
    let show_recuse = use_state(|| false);
    let ack_warnings = use_state(|| false);

    let d = &props.detail;
    let is_completed = d.assignment.status == "completed" || d.assignment.status == "recused";

    let on_back = {
        let cb = props.on_back.clone();
        Callback::from(move |_: MouseEvent| cb.emit(()))
    };

    // Build score inputs
    let on_submit = {
        let ratings = ratings.clone();
        let comments = comments.clone();
        let overall_comment = overall_comment.clone();
        let recommendation = recommendation.clone();
        let ack_warnings = ack_warnings.clone();
        let cb = props.on_submit.clone();

        Callback::from(move |_: MouseEvent| {
            let scores: Vec<serde_json::Value> = ratings.iter().map(|(dim_id, rating)| {
                serde_json::json!({
                    "dimension_id": dim_id,
                    "rating": rating,
                    "comment": comments.get(dim_id).cloned(),
                })
            }).collect();

            let oc = if overall_comment.is_empty() { None } else { Some((*overall_comment).clone()) };
            cb.emit((scores, oc, (*recommendation).clone(), *ack_warnings));
        })
    };

    let on_recuse_submit = {
        let cb = props.on_recuse.clone();
        let reason = recuse_reason.clone();
        Callback::from(move |_: MouseEvent| {
            if !reason.is_empty() {
                cb.emit((*reason).clone());
            }
        })
    };

    html! {
        <div>
            <button onclick={on_back} class="back-btn">{"< Back to Queue"}</button>

            <div class="task-detail-card">
                // Header
                <div class="task-detail-header">
                    <div>
                        <h2>{ &d.target_summary.title }</h2>
                        <span class="task-card-group">{ format!("Scorecard: {}", d.scorecard.name) }</span>
                    </div>
                    <span class={classes!("task-status-badge", match d.assignment.status.as_str() {
                        "pending" => "status-scheduled",
                        "in_progress" => "status-progress",
                        "completed" => "status-completed",
                        _ => "",
                    })}>{ &d.assignment.status }</span>
                </div>

                // Target info
                <div class="task-detail-meta">
                    { if let Some(ref name) = d.target_summary.submitter_name {
                        html! { <p>{ format!("Submitted by: {}", name) }</p> }
                    } else if d.assignment.is_blind {
                        html! { <p style="color:#7c3aed;font-weight:500;">{"Blind review - submitter identity hidden"}</p> }
                    } else { html! {} }}
                    { if let Some(ref at) = d.target_summary.submitted_at {
                        html! { <p>{ format!("Submitted: {}", at) }</p> }
                    } else { html! {} }}
                    { if let Some(ref due) = d.assignment.due_date {
                        html! { <p>{ format!("Review due: {}", due) }</p> }
                    } else { html! {} }}
                    { if let Some(ref ps) = d.scorecard.passing_score {
                        html! { <p>{ format!("Passing score: {:.1}", ps) }</p> }
                    } else { html! {} }}
                </div>

                // Scoring form
                { if !is_completed {
                    html! {
                        <div class="subtask-form">
                            <h3>{"Score Dimensions"}</h3>
                            { for d.dimensions.iter().map(|dim| {
                                render_dimension(dim, ratings.clone(), comments.clone())
                            })}

                            <div class="form-field" style="margin-top:20px;">
                                <label>{"Overall Comment"}</label>
                                <textarea
                                    value={(*overall_comment).clone()}
                                    oninput={{
                                        let oc = overall_comment.clone();
                                        Callback::from(move |e: InputEvent| {
                                            let t: HtmlTextAreaElement = e.target_unchecked_into();
                                            oc.set(t.value());
                                        })
                                    }}
                                    rows="3"
                                    placeholder="Overall assessment..."
                                />
                            </div>

                            <div class="form-field" style="margin-top:12px;">
                                <label>{"Recommendation"}</label>
                                <select onchange={{
                                    let rec = recommendation.clone();
                                    Callback::from(move |e: Event| {
                                        let s: HtmlSelectElement = e.target_unchecked_into();
                                        rec.set(s.value());
                                    })
                                }}>
                                    <option value="approve" selected=true>{"Approve"}</option>
                                    <option value="reject">{"Reject"}</option>
                                    <option value="revise">{"Needs Revision"}</option>
                                </select>
                            </div>

                            // Consistency warnings/errors display
                            { if let Some(ref result) = props.submit_result {
                                render_submit_result(result, ack_warnings.clone())
                            } else { html! {} }}

                            <div style="display:flex;gap:12px;margin-top:20px;">
                                <button onclick={on_submit} class="btn-primary" disabled={props.is_submitting}>
                                    { if props.is_submitting { "Submitting..." } else { "Submit Review" } }
                                </button>
                                <button onclick={{
                                    let show = show_recuse.clone();
                                    Callback::from(move |_: MouseEvent| show.set(!*show))
                                }} class="btn-small" style="color:#991b1b;">
                                    {"Recuse"}
                                </button>
                            </div>

                            { if *show_recuse {
                                html! {
                                    <div style="margin-top:12px;padding:12px;background:#fef2f2;border-radius:6px;">
                                        <label style="font-size:14px;font-weight:500;">{"Recusal Reason"}</label>
                                        <input type="text"
                                            value={(*recuse_reason).clone()}
                                            oninput={{
                                                let rr = recuse_reason.clone();
                                                Callback::from(move |e: InputEvent| {
                                                    let i: HtmlInputElement = e.target_unchecked_into();
                                                    rr.set(i.value());
                                                })
                                            }}
                                            style="width:100%;padding:8px;margin:8px 0;border:1px solid #fecaca;border-radius:4px;"
                                            placeholder="Reason for recusal..."
                                        />
                                        <button onclick={on_recuse_submit} class="btn-small" style="background:#fee2e2;">
                                            {"Confirm Recusal"}
                                        </button>
                                    </div>
                                }
                            } else { html! {} }}
                        </div>
                    }
                } else if let Some(ref rev) = d.existing_review {
                    // Show completed review
                    html! {
                        <div class="submission-display">
                            <h3>{"Review Submitted"}</h3>
                            <p>{ format!("Score: {:.1} | Recommendation: {}",
                                rev.overall_score.unwrap_or(0.0),
                                rev.recommendation.as_deref().unwrap_or("N/A")) }</p>
                            { if let Some(ref c) = rev.overall_comment {
                                html! { <p>{ format!("Comment: {}", c) }</p> }
                            } else { html! {} }}
                        </div>
                    }
                } else { html! {} }}
            </div>
        </div>
    }
}

fn render_dimension(
    dim: &ScorecardDimension,
    ratings: UseStateHandle<HashMap<Uuid, i32>>,
    comments: UseStateHandle<HashMap<Uuid, String>>,
) -> Html {
    let id = dim.id;
    let levels = dim.rating_levels.as_array().cloned().unwrap_or_default();

    let on_rating = {
        let ratings = ratings.clone();
        Callback::from(move |e: Event| {
            let s: HtmlSelectElement = e.target_unchecked_into();
            if let Ok(v) = s.value().parse::<i32>() {
                let mut map = (*ratings).clone();
                map.insert(id, v);
                ratings.set(map);
            }
        })
    };

    let on_comment = {
        let comments = comments.clone();
        Callback::from(move |e: InputEvent| {
            let t: HtmlTextAreaElement = e.target_unchecked_into();
            let mut map = (*comments).clone();
            map.insert(id, t.value());
            comments.set(map);
        })
    };

    let comment_note = if dim.comment_required {
        "Comment required"
    } else if dim.comment_required_below.is_some() {
        "Comment required for low ratings"
    } else {
        ""
    };

    html! {
        <div class="subtask-item">
            <div style="display:flex;justify-content:space-between;align-items:center;">
                <label>
                    { &dim.name }
                    <span style="font-size:12px;color:#64748b;margin-left:8px;">
                        { format!("(weight: {:.1})", dim.weight) }
                    </span>
                </label>
            </div>
            { if let Some(ref desc) = dim.description {
                html! { <div class="subtask-desc">{ desc }</div> }
            } else { html! {} }}

            <select onchange={on_rating} style="margin-top:8px;">
                <option value="">{"-- Rate --"}</option>
                { for levels.iter().map(|level| {
                    let val = level.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                    let label = level.get("label").and_then(|v| v.as_str()).unwrap_or("");
                    html! { <option value={val.to_string()}>{ format!("{} - {}", val, label) }</option> }
                })}
            </select>

            <textarea
                oninput={on_comment}
                rows="2"
                placeholder={if comment_note.is_empty() { "Comment (optional)..." } else { comment_note }}
                style="margin-top:8px;width:100%;padding:6px;border:1px solid #e2e8f0;border-radius:4px;font-size:13px;"
            />
            { if !comment_note.is_empty() {
                html! { <span style="font-size:11px;color:#d97706;">{ comment_note }</span> }
            } else { html! {} }}
        </div>
    }
}

fn render_submit_result(
    result: &SubmitReviewResponse,
    ack_warnings: UseStateHandle<bool>,
) -> Html {
    let cc = &result.consistency;

    html! {
        <div style="margin-top:16px;">
            { if result.valid {
                html! {
                    <div class="validation-success">
                        <p class="validation-ok">{"Review submitted successfully!"}</p>
                    </div>
                }
            } else {
                html! {
                    <div class={if cc.has_errors { "validation-errors" } else { "validation-success" }}>
                        { if let Some(ref msg) = result.message {
                            html! { <p class="validation-title">{ msg }</p> }
                        } else { html! {} }}

                        { for cc.results.iter().map(|item| {
                            let cls = if item.severity == "error" { "validation-error" } else { "validation-warning" };
                            html! {
                                <div class={cls} style="margin:8px 0;padding:8px;border-radius:4px;">
                                    <strong>{ format!("[{}] {}", item.severity.to_uppercase(), item.rule_name) }</strong>
                                    <p style="margin:4px 0 0;">{ &item.message }</p>
                                </div>
                            }
                        })}

                        { if cc.has_warnings && !cc.has_errors {
                            let ack = ack_warnings.clone();
                            html! {
                                <label class="checkbox-label" style="margin-top:12px;">
                                    <input type="checkbox"
                                        checked={*ack}
                                        onchange={Callback::from(move |e: Event| {
                                            let i: HtmlInputElement = e.target_unchecked_into();
                                            ack.set(i.checked());
                                        })}
                                    />
                                    <span>{"I acknowledge these consistency warnings and wish to proceed"}</span>
                                </label>
                            }
                        } else { html! {} }}
                    </div>
                }
            }}
        </div>
    }
}
