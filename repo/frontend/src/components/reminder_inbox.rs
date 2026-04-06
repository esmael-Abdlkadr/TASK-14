use uuid::Uuid;
use yew::prelude::*;

use crate::services::inspection_api::{ReminderInbox as InboxData, TaskReminder};

#[derive(Properties, PartialEq)]
pub struct ReminderInboxProps {
    pub inbox: Option<InboxData>,
    pub is_loading: bool,
    pub on_read: Callback<Uuid>,
    pub on_dismiss: Callback<Uuid>,
    pub on_read_all: Callback<()>,
    pub on_refresh: Callback<()>,
}

fn reminder_icon(reminder_type: &str) -> &'static str {
    match reminder_type {
        "upcoming" => "[i]",
        "due_soon" => "[!]",
        "overdue" => "[!!]",
        "makeup_deadline" => "[M]",
        "missed_warning" => "[X]",
        _ => "[ ]",
    }
}

fn reminder_class(reminder_type: &str) -> &'static str {
    match reminder_type {
        "upcoming" => "reminder-upcoming",
        "due_soon" => "reminder-due-soon",
        "overdue" => "reminder-overdue",
        "makeup_deadline" => "reminder-makeup",
        "missed_warning" => "reminder-missed",
        _ => "",
    }
}

#[function_component(ReminderInboxComponent)]
pub fn reminder_inbox(props: &ReminderInboxProps) -> Html {
    let expanded = use_state(|| false);

    let toggle = {
        let expanded = expanded.clone();
        let on_refresh = props.on_refresh.clone();
        Callback::from(move |_: MouseEvent| {
            let new_state = !*expanded;
            expanded.set(new_state);
            if new_state {
                on_refresh.emit(());
            }
        })
    };

    let inbox = props.inbox.as_ref();
    let unread = inbox.map(|i| i.unread_count).unwrap_or(0);

    html! {
        <div class="reminder-inbox">
            <div class="reminder-inbox-header" onclick={toggle}>
                <span class="reminder-inbox-title">
                    {"Reminders"}
                    { if unread > 0 {
                        html! { <span class="unread-badge">{ unread }</span> }
                    } else { html! {} }}
                </span>
                <span>{ if *expanded { "[-]" } else { "[+]" } }</span>
            </div>

            { if *expanded {
                html! {
                    <div class="reminder-inbox-body">
                        { if props.is_loading {
                            html! { <div class="loading-spinner">{"Loading..."}</div> }
                        } else if let Some(inbox) = inbox {
                            html! {
                                <div>
                                    { if unread > 0 {
                                        let on_read_all = props.on_read_all.clone();
                                        html! {
                                            <button
                                                onclick={Callback::from(move |_: MouseEvent| on_read_all.emit(()))}
                                                class="btn-small"
                                            >
                                                {"Mark all read"}
                                            </button>
                                        }
                                    } else { html! {} }}

                                    { if inbox.reminders.is_empty() {
                                        html! { <p class="empty-reminders">{"No reminders"}</p> }
                                    } else {
                                        html! {
                                            <div class="reminder-list">
                                                { for inbox.reminders.iter().map(|r| {
                                                    render_reminder(r, props.on_read.clone(), props.on_dismiss.clone())
                                                })}
                                            </div>
                                        }
                                    }}
                                </div>
                            }
                        } else {
                            html! { <p>{"Click to load reminders"}</p> }
                        }}
                    </div>
                }
            } else { html! {} }}
        </div>
    }
}

fn render_reminder(
    reminder: &TaskReminder,
    on_read: Callback<Uuid>,
    on_dismiss: Callback<Uuid>,
) -> Html {
    let is_unread = reminder.status == "unread";
    let id = reminder.id;

    let on_read_click = {
        let on_read = on_read.clone();
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            on_read.emit(id);
        })
    };

    let on_dismiss_click = {
        let on_dismiss = on_dismiss.clone();
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            on_dismiss.emit(id);
        })
    };

    html! {
        <div class={classes!(
            "reminder-item",
            reminder_class(&reminder.reminder_type),
            if is_unread { "reminder-unread" } else { "" }
        )}>
            <div class="reminder-content">
                <span class="reminder-icon">{ reminder_icon(&reminder.reminder_type) }</span>
                <div>
                    <div class="reminder-title">{ &reminder.title }</div>
                    <div class="reminder-message">{ &reminder.message }</div>
                    <div class="reminder-meta">
                        { if let Some(ref date) = reminder.due_date {
                            format!("Due: {} | ", date)
                        } else {
                            String::new()
                        }}
                        { &reminder.created_at }
                    </div>
                </div>
            </div>
            <div class="reminder-actions">
                { if is_unread {
                    html! { <button onclick={on_read_click} class="btn-tiny">{"Read"}</button> }
                } else { html! {} }}
                <button onclick={on_dismiss_click} class="btn-tiny btn-dismiss">{"Dismiss"}</button>
            </div>
        </div>
    }
}
