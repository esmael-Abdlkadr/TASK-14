use uuid::Uuid;
use yew::prelude::*;

use crate::services::inspection_api::{TaskInstanceDetail, TaskListResponse};

#[derive(Properties, PartialEq)]
pub struct TaskListProps {
    pub response: Option<TaskListResponse>,
    pub is_loading: bool,
    pub error: Option<String>,
    pub on_select_task: Callback<Uuid>,
    pub on_page_change: Callback<i64>,
}

fn status_class(status: &str) -> &'static str {
    match status {
        "scheduled" => "status-scheduled",
        "in_progress" => "status-progress",
        "submitted" => "status-submitted",
        "completed" => "status-completed",
        "overdue" => "status-overdue",
        "missed" => "status-missed",
        "makeup" => "status-makeup",
        _ => "",
    }
}

fn status_label(status: &str) -> String {
    match status {
        "scheduled" => "Scheduled".to_string(),
        "in_progress" => "In Progress".to_string(),
        "submitted" => "Submitted".to_string(),
        "completed" => "Completed".to_string(),
        "overdue" => "Overdue".to_string(),
        "missed" => "Missed".to_string(),
        "makeup" => "Makeup".to_string(),
        _ => status.to_string(),
    }
}

#[function_component(TaskList)]
pub fn task_list(props: &TaskListProps) -> Html {
    if props.is_loading {
        return html! { <div class="loading-spinner">{"Loading tasks..."}</div> };
    }

    if let Some(ref err) = props.error {
        return html! { <div class="error-message">{ err }</div> };
    }

    let resp = match &props.response {
        Some(r) => r,
        None => {
            return html! {
                <div class="empty-state">
                    <h3>{"No tasks to display"}</h3>
                    <p>{"Tasks will appear here when schedules are created."}</p>
                </div>
            }
        }
    };

    if resp.tasks.is_empty() {
        return html! {
            <div class="empty-state">
                <h3>{"No tasks found"}</h3>
                <p>{"Try adjusting your filters."}</p>
            </div>
        };
    }

    let total_pages = (resp.total as f64 / resp.page_size as f64).ceil() as i64;

    html! {
        <div>
            <div class="task-list-header">
                { format!("{} tasks found", resp.total) }
            </div>

            { for resp.tasks.iter().map(|task| {
                let on_click = {
                    let cb = props.on_select_task.clone();
                    let id = task.instance.id;
                    Callback::from(move |_: MouseEvent| cb.emit(id))
                };

                html! {
                    <div class="task-card" onclick={on_click}>
                        <div class="task-card-header">
                            <div>
                                <div class="task-card-title">{ &task.template_name }</div>
                                { if let Some(ref group) = task.template_group {
                                    html! { <span class="task-card-group">{ group }</span> }
                                } else {
                                    html! {}
                                }}
                            </div>
                            <span class={classes!("task-status-badge", status_class(&task.instance.status))}>
                                { status_label(&task.instance.status) }
                            </span>
                        </div>
                        <div class="task-card-meta">
                            <span>{ format!("Due: {}", task.instance.due_date) }</span>
                            <span>{ format!("{} - {}", task.instance.window_start, task.instance.window_end) }</span>
                            { if task.instance.is_makeup {
                                html! { <span class="tag tag-makeup">{"Makeup"}</span> }
                            } else {
                                html! {}
                            }}
                            <span>{ format!("{} subtasks", task.subtasks.len()) }</span>
                        </div>
                        { if let Some(ref sub) = task.submission {
                            html! {
                                <div class="task-card-submission">
                                    { format!("Submitted: {} | Review: {}", sub.submitted_at, sub.status) }
                                </div>
                            }
                        } else {
                            html! {}
                        }}
                    </div>
                }
            })}

            { if total_pages > 1 {
                let on_prev = {
                    let cb = props.on_page_change.clone();
                    let page = resp.page;
                    Callback::from(move |_: MouseEvent| { if page > 1 { cb.emit(page - 1); } })
                };
                let on_next = {
                    let cb = props.on_page_change.clone();
                    let page = resp.page;
                    let total = total_pages;
                    Callback::from(move |_: MouseEvent| { if page < total { cb.emit(page + 1); } })
                };
                html! {
                    <div class="pagination">
                        <button onclick={on_prev} disabled={resp.page <= 1}>{"Previous"}</button>
                        <span>{ format!("Page {} of {}", resp.page, total_pages) }</span>
                        <button onclick={on_next} disabled={resp.page >= total_pages}>{"Next"}</button>
                    </div>
                }
            } else {
                html! {}
            }}
        </div>
    }
}
