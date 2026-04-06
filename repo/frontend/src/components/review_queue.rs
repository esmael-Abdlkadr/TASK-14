use uuid::Uuid;
use yew::prelude::*;

use crate::services::review_api::{AssignmentDetail, ReviewQueueResponse};

#[derive(Properties, PartialEq)]
pub struct ReviewQueueProps {
    pub response: Option<ReviewQueueResponse>,
    pub is_loading: bool,
    pub error: Option<String>,
    pub on_select: Callback<Uuid>,
    pub on_page_change: Callback<i64>,
}

#[function_component(ReviewQueue)]
pub fn review_queue(props: &ReviewQueueProps) -> Html {
    if props.is_loading {
        return html! { <div class="loading-spinner">{"Loading review queue..."}</div> };
    }
    if let Some(ref err) = props.error {
        return html! { <div class="error-message">{ err }</div> };
    }
    let resp = match &props.response {
        Some(r) => r,
        None => return html! {
            <div class="empty-state">
                <h3>{"Review Queue"}</h3>
                <p>{"No review assignments to display."}</p>
            </div>
        },
    };

    if resp.assignments.is_empty() {
        return html! {
            <div class="empty-state">
                <h3>{"All caught up!"}</h3>
                <p>{"No pending reviews in your queue."}</p>
            </div>
        };
    }

    let total_pages = (resp.total as f64 / resp.page_size as f64).ceil() as i64;

    html! {
        <div>
            <div class="task-list-header">{ format!("{} reviews assigned", resp.total) }</div>
            { for resp.assignments.iter().map(|a| {
                let on_click = {
                    let cb = props.on_select.clone();
                    let id = a.assignment.id;
                    Callback::from(move |_: MouseEvent| cb.emit(id))
                };
                html! {
                    <div class="task-card" onclick={on_click}>
                        <div class="task-card-header">
                            <div>
                                <div class="task-card-title">{ &a.target_summary.title }</div>
                                <span class="task-card-group">{ format!("Scorecard: {}", a.scorecard.name) }</span>
                            </div>
                            <span class={classes!("task-status-badge", match a.assignment.status.as_str() {
                                "pending" => "status-scheduled",
                                "in_progress" => "status-progress",
                                "completed" => "status-completed",
                                "recused" => "status-missed",
                                _ => "",
                            })}>
                                { &a.assignment.status }
                            </span>
                        </div>
                        <div class="task-card-meta">
                            <span>{ format!("Assigned: {}", &a.assignment.assigned_at) }</span>
                            { if let Some(ref due) = a.assignment.due_date {
                                html! { <span>{ format!("Due: {}", due) }</span> }
                            } else { html! {} }}
                            { if a.assignment.is_blind {
                                html! { <span class="tag tag-makeup">{"Blind"}</span> }
                            } else { html! {} }}
                            <span>{ format!("{} dimensions", a.dimensions.len()) }</span>
                            { if let Some(ref name) = a.target_summary.submitter_name {
                                html! { <span>{ format!("By: {}", name) }</span> }
                            } else { html! {} }}
                        </div>
                        { if let Some(ref rev) = a.existing_review {
                            html! {
                                <div class="task-card-submission">
                                    { format!("Review: {} | Score: {:.1}",
                                        rev.status, rev.overall_score.unwrap_or(0.0)) }
                                </div>
                            }
                        } else { html! {} }}
                    </div>
                }
            })}
            { if total_pages > 1 {
                let on_prev = {
                    let cb = props.on_page_change.clone();
                    let p = resp.page;
                    Callback::from(move |_: MouseEvent| { if p > 1 { cb.emit(p - 1); } })
                };
                let on_next = {
                    let cb = props.on_page_change.clone();
                    let p = resp.page; let t = total_pages;
                    Callback::from(move |_: MouseEvent| { if p < t { cb.emit(p + 1); } })
                };
                html! {
                    <div class="pagination">
                        <button onclick={on_prev} disabled={resp.page <= 1}>{"Previous"}</button>
                        <span>{ format!("Page {} of {}", resp.page, total_pages) }</span>
                        <button onclick={on_next} disabled={resp.page >= total_pages}>{"Next"}</button>
                    </div>
                }
            } else { html! {} }}
        </div>
    }
}
