use uuid::Uuid;
use web_sys::HtmlSelectElement;
use yew::prelude::*;

use crate::services::messaging_api::{ExternalPayload, PayloadQueueResponse};

#[derive(Properties, PartialEq)]
pub struct PayloadQueueProps {
    pub response: Option<PayloadQueueResponse>,
    pub is_loading: bool,
    pub on_export: Callback<String>,
    pub on_mark_delivered: Callback<Vec<Uuid>>,
    pub on_mark_failed: Callback<(Uuid, String)>,
    pub on_view_log: Callback<Uuid>,
    pub on_refresh: Callback<()>,
}

fn status_badge(status: &str) -> (&'static str, String) {
    match status {
        "queued" => ("status-scheduled", "Queued".to_string()),
        "exported" => ("status-progress", "Exported".to_string()),
        "delivered" => ("status-completed", "Delivered".to_string()),
        "failed" => ("status-overdue", "Failed".to_string()),
        "retrying" => ("status-makeup", "Retrying".to_string()),
        _ => ("", status.to_string()),
    }
}

#[function_component(PayloadQueue)]
pub fn payload_queue(props: &PayloadQueueProps) -> Html {
    let export_channel = use_state(|| "sms".to_string());

    let on_export = {
        let ch = export_channel.clone();
        let cb = props.on_export.clone();
        Callback::from(move |_: MouseEvent| cb.emit((*ch).clone()))
    };

    html! {
        <div>
            <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:16px;">
                <h3 class="section-title" style="margin:0;">{"External Payload Queue"}</h3>
                <div style="display:flex;gap:8px;align-items:center;">
                    <select onchange={{
                        let ch = export_channel.clone();
                        Callback::from(move |e: Event| {
                            let s: HtmlSelectElement = e.target_unchecked_into();
                            ch.set(s.value());
                        })
                    }}>
                        <option value="sms">{"SMS"}</option>
                        <option value="email">{"Email"}</option>
                        <option value="push">{"Push"}</option>
                    </select>
                    <button onclick={on_export} class="btn-primary" style="padding:6px 16px;font-size:13px;">
                        {"Export Queued"}
                    </button>
                    <button onclick={{
                        let cb = props.on_refresh.clone();
                        Callback::from(move |_: MouseEvent| cb.emit(()))
                    }} class="btn-small">{"Refresh"}</button>
                </div>
            </div>

            { if props.is_loading {
                html! { <div class="loading-spinner">{"Loading..."}</div> }
            } else if let Some(ref resp) = props.response {
                html! {
                    <div>
                        <div class="counter-grid" style="margin-bottom:16px;">
                            <div class="counter-card counter-warning">
                                <div class="counter-value">{ resp.queued_count }</div>
                                <div class="counter-label">{"Queued"}</div>
                            </div>
                            <div class="counter-card counter-danger">
                                <div class="counter-value">{ resp.failed_count }</div>
                                <div class="counter-label">{"Failed"}</div>
                            </div>
                        </div>

                        { if resp.payloads.is_empty() {
                            html! { <div class="empty-state"><p>{"No payloads in queue"}</p></div> }
                        } else {
                            html! {
                                <div>
                                    { for resp.payloads.iter().map(|p| {
                                        render_payload(p, props.on_mark_delivered.clone(), props.on_mark_failed.clone(), props.on_view_log.clone())
                                    })}
                                </div>
                            }
                        }}
                    </div>
                }
            } else {
                html! { <div class="empty-state"><p>{"Click refresh to load queue"}</p></div> }
            }}
        </div>
    }
}

fn render_payload(
    p: &ExternalPayload,
    on_delivered: Callback<Vec<Uuid>>,
    on_failed: Callback<(Uuid, String)>,
    on_log: Callback<Uuid>,
) -> Html {
    let (badge_class, badge_label) = status_badge(&p.status);
    let id = p.id;
    let on_log_click = {
        let cb = on_log.clone();
        Callback::from(move |_: MouseEvent| cb.emit(id))
    };

    html! {
        <div class="task-card" style="cursor:default;">
            <div class="task-card-header">
                <div>
                    <span style="font-weight:600;">{ &p.channel }</span>
                    <span style="margin-left:8px;color:#64748b;font-size:13px;">{ format!("-> {}", &p.recipient) }</span>
                </div>
                <span class={classes!("task-status-badge", badge_class)}>{ badge_label }</span>
            </div>
            <div style="font-size:13px;color:#64748b;margin:4px 0;">
                { if let Some(ref subj) = p.subject { format!("Subject: {} | ", subj) } else { String::new() } }
                { format!("Retries: {}/{}", p.retry_count, p.max_retries) }
                { if let Some(ref err) = p.last_error { format!(" | Error: {}", err) } else { String::new() } }
            </div>
            <div style="font-size:13px;margin:4px 0;">{ &p.body }</div>
            <div style="display:flex;gap:8px;margin-top:8px;">
                { if p.status == "exported" {
                    let cb = on_delivered.clone();
                    html! { <button onclick={Callback::from(move |_: MouseEvent| cb.emit(vec![id]))} class="btn-small" style="color:#065f46;">{"Mark Delivered"}</button> }
                } else { html! {} }}
                { if p.status == "exported" || p.status == "retrying" {
                    let cb = on_failed.clone();
                    html! { <button onclick={Callback::from(move |_: MouseEvent| cb.emit((id, "Manual failure".into())))} class="btn-small" style="color:#991b1b;">{"Mark Failed"}</button> }
                } else { html! {} }}
                <button onclick={on_log_click} class="btn-tiny">{"Log"}</button>
            </div>
        </div>
    }
}
