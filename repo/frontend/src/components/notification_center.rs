use uuid::Uuid;
use yew::prelude::*;

use crate::services::messaging_api::{Notification, NotificationInbox};

#[derive(Properties, PartialEq)]
pub struct NotificationCenterProps {
    pub inbox: Option<NotificationInbox>,
    pub is_loading: bool,
    pub on_read: Callback<Uuid>,
    pub on_dismiss: Callback<Uuid>,
    pub on_read_all: Callback<()>,
    pub on_refresh: Callback<()>,
    pub on_page_change: Callback<i64>,
}

fn channel_icon(channel: &str) -> &'static str {
    match channel {
        "in_app" => "[A]",
        "sms" => "[S]",
        "email" => "[E]",
        "push" => "[P]",
        _ => "[ ]",
    }
}

fn status_class(status: &str) -> &'static str {
    match status {
        "pending" | "delivered" => "notif-unread",
        "read" => "notif-read",
        "failed" => "notif-failed",
        "dismissed" => "notif-dismissed",
        _ => "",
    }
}

#[function_component(NotificationCenter)]
pub fn notification_center(props: &NotificationCenterProps) -> Html {
    let expanded = use_state(|| false);

    let toggle = {
        let expanded = expanded.clone();
        let on_refresh = props.on_refresh.clone();
        Callback::from(move |_: MouseEvent| {
            let new = !*expanded;
            expanded.set(new);
            if new {
                on_refresh.emit(());
            }
        })
    };

    let inbox = props.inbox.as_ref();
    let unread = inbox.map(|i| i.unread_count).unwrap_or(0);

    html! {
        <div class="notification-center">
            <div class="notif-header" onclick={toggle}>
                <span class="notif-title">
                    {"Notifications"}
                    { if unread > 0 {
                        html! { <span class="unread-badge">{ unread }</span> }
                    } else { html! {} }}
                </span>
                <span>{ if *expanded { "[-]" } else { "[+]" } }</span>
            </div>

            { if *expanded {
                html! {
                    <div class="notif-body">
                        { if props.is_loading {
                            html! { <div class="loading-spinner">{"Loading..."}</div> }
                        } else if let Some(inbox) = inbox {
                            html! {
                                <div>
                                    { if unread > 0 {
                                        let cb = props.on_read_all.clone();
                                        html! { <button onclick={Callback::from(move |_: MouseEvent| cb.emit(()))} class="btn-small">{"Mark all read"}</button> }
                                    } else { html! {} }}

                                    { if inbox.notifications.is_empty() {
                                        html! { <p class="empty-reminders">{"No notifications"}</p> }
                                    } else {
                                        html! {
                                            <div class="notif-list">
                                                { for inbox.notifications.iter().map(|n| {
                                                    render_notification(n, props.on_read.clone(), props.on_dismiss.clone())
                                                })}
                                            </div>
                                        }
                                    }}

                                    { if inbox.total > inbox.page_size {
                                        let total_pages = (inbox.total as f64 / inbox.page_size as f64).ceil() as i64;
                                        let on_prev = {
                                            let cb = props.on_page_change.clone();
                                            let p = inbox.page;
                                            Callback::from(move |_: MouseEvent| { if p > 1 { cb.emit(p - 1); } })
                                        };
                                        let on_next = {
                                            let cb = props.on_page_change.clone();
                                            let p = inbox.page; let t = total_pages;
                                            Callback::from(move |_: MouseEvent| { if p < t { cb.emit(p + 1); } })
                                        };
                                        html! {
                                            <div class="pagination" style="margin-top:8px;">
                                                <button onclick={on_prev} disabled={inbox.page <= 1} class="btn-tiny">{"<"}</button>
                                                <span style="font-size:12px;">{ format!("{}/{}", inbox.page, total_pages) }</span>
                                                <button onclick={on_next} disabled={inbox.page >= total_pages} class="btn-tiny">{">"}</button>
                                            </div>
                                        }
                                    } else { html! {} }}
                                </div>
                            }
                        } else { html! {} }}
                    </div>
                }
            } else { html! {} }}
        </div>
    }
}

fn render_notification(
    n: &Notification,
    on_read: Callback<Uuid>,
    on_dismiss: Callback<Uuid>,
) -> Html {
    let is_unread = n.status == "pending" || n.status == "delivered";
    let id = n.id;
    let on_dismiss_click = {
        let cb = on_dismiss.clone();
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            cb.emit(id);
        })
    };

    html! {
        <div class={classes!("notif-item", status_class(&n.status))}>
            <div class="notif-content">
                <span class="notif-icon">{ channel_icon(&n.channel) }</span>
                <div style="flex:1;">
                    { if let Some(ref subj) = n.subject {
                        html! { <div class="notif-subject">{ subj }</div> }
                    } else { html! {} }}
                    <div class="notif-message">{ &n.body }</div>
                    <div class="notif-meta">
                        { if let Some(ref evt) = n.event_type {
                            format!("{} | ", evt)
                        } else { String::new() }}
                        { &n.created_at }
                    </div>
                </div>
            </div>
            <div class="reminder-actions">
                { if is_unread {
                    let cb = on_read.clone();
                    html! { <button onclick={Callback::from(move |e: MouseEvent| { e.stop_propagation(); cb.emit(id); })} class="btn-tiny">{"Read"}</button> }
                } else { html! {} }}
                <button onclick={on_dismiss_click} class="btn-tiny btn-dismiss">{"X"}</button>
            </div>
        </div>
    }
}
