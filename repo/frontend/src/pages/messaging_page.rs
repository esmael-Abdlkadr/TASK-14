use uuid::Uuid;
use web_sys::HtmlSelectElement;
use yew::prelude::*;

use crate::components::notification_center::NotificationCenter;
use crate::components::payload_queue::PayloadQueue;
use crate::services::messaging_api;

#[function_component(MessagingPage)]
pub fn messaging_page() -> Html {
    let active_tab = use_state(|| "inbox".to_string());

    // Inbox state
    let inbox = use_state(|| None::<messaging_api::NotificationInbox>);
    let inbox_loading = use_state(|| false);
    let inbox_page = use_state(|| 1i64);

    // Payload queue state
    let payload_resp = use_state(|| None::<messaging_api::PayloadQueueResponse>);
    let payload_loading = use_state(|| false);

    // Templates/triggers state
    let templates = use_state(Vec::<messaging_api::NotificationTemplate>::new);
    let triggers = use_state(Vec::<messaging_api::TriggerRule>::new);

    // Delivery log state
    let delivery_log = use_state(Vec::<messaging_api::DeliveryLogEntry>::new);
    let log_payload_id = use_state(|| None::<Uuid>);

    let export_result = use_state(|| None::<String>);

    // Load data on tab change
    {
        let tab = active_tab.clone();
        let inbox = inbox.clone(); let inbox_loading = inbox_loading.clone();
        let payload_resp = payload_resp.clone(); let payload_loading = payload_loading.clone();
        let templates = templates.clone(); let triggers = triggers.clone();
        let inbox_page = inbox_page.clone();

        use_effect_with((*tab).clone(), move |tab| {
            let tab = tab.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match tab.as_str() {
                    "inbox" => {
                        inbox_loading.set(true);
                        if let Ok(i) = messaging_api::get_notifications(None, *inbox_page).await {
                            inbox.set(Some(i));
                        }
                        inbox_loading.set(false);
                    }
                    "queue" => {
                        payload_loading.set(true);
                        if let Ok(r) = messaging_api::get_payload_queue(None, 1).await {
                            payload_resp.set(Some(r));
                        }
                        payload_loading.set(false);
                    }
                    "config" => {
                        if let Ok(t) = messaging_api::get_templates().await { templates.set(t); }
                        if let Ok(r) = messaging_api::get_triggers().await { triggers.set(r); }
                    }
                    _ => {}
                }
            });
            || ()
        });
    }

    let set_tab = |name: &'static str| {
        let tab = active_tab.clone();
        Callback::from(move |_: MouseEvent| tab.set(name.to_string()))
    };

    // Inbox callbacks
    let on_read = {
        let inbox = inbox.clone(); let inbox_loading = inbox_loading.clone(); let page = inbox_page.clone();
        Callback::from(move |id: Uuid| {
            let inbox = inbox.clone(); let il = inbox_loading.clone(); let p = *page;
            wasm_bindgen_futures::spawn_local(async move {
                let _ = messaging_api::mark_notification_read(id).await;
                if let Ok(i) = messaging_api::get_notifications(None, p).await { inbox.set(Some(i)); }
            });
        })
    };
    let on_dismiss = {
        let inbox = inbox.clone(); let page = inbox_page.clone();
        Callback::from(move |id: Uuid| {
            let inbox = inbox.clone(); let p = *page;
            wasm_bindgen_futures::spawn_local(async move {
                let _ = messaging_api::dismiss_notification(id).await;
                if let Ok(i) = messaging_api::get_notifications(None, p).await { inbox.set(Some(i)); }
            });
        })
    };
    let on_read_all = {
        let inbox = inbox.clone();
        Callback::from(move |_: ()| {
            let inbox = inbox.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let _ = messaging_api::mark_all_read().await;
                if let Ok(i) = messaging_api::get_notifications(None, 1).await { inbox.set(Some(i)); }
            });
        })
    };
    let on_inbox_refresh = {
        let inbox = inbox.clone(); let il = inbox_loading.clone(); let page = inbox_page.clone();
        Callback::from(move |_: ()| {
            let inbox = inbox.clone(); let il = il.clone(); let p = *page;
            il.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(i) = messaging_api::get_notifications(None, p).await { inbox.set(Some(i)); }
                il.set(false);
            });
        })
    };
    let on_inbox_page = {
        let inbox = inbox.clone(); let page = inbox_page.clone();
        Callback::from(move |p: i64| {
            page.set(p);
            let inbox = inbox.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(i) = messaging_api::get_notifications(None, p).await { inbox.set(Some(i)); }
            });
        })
    };

    // Payload callbacks
    let on_export = {
        let export_result = export_result.clone(); let payload_resp = payload_resp.clone();
        Callback::from(move |channel: String| {
            let er = export_result.clone(); let pr = payload_resp.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match messaging_api::export_payloads(&channel).await {
                    Ok(r) => er.set(Some(format!("Exported {} {} payloads to {}", r.count, channel, r.export_dir))),
                    Err(e) => er.set(Some(format!("Export error: {}", e))),
                }
                if let Ok(r) = messaging_api::get_payload_queue(None, 1).await { pr.set(Some(r)); }
            });
        })
    };
    let on_mark_delivered = {
        let payload_resp = payload_resp.clone();
        Callback::from(move |ids: Vec<Uuid>| {
            let pr = payload_resp.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let _ = messaging_api::mark_delivered(ids).await;
                if let Ok(r) = messaging_api::get_payload_queue(None, 1).await { pr.set(Some(r)); }
            });
        })
    };
    let on_mark_failed = {
        let payload_resp = payload_resp.clone();
        Callback::from(move |(id, err): (Uuid, String)| {
            let pr = payload_resp.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let _ = messaging_api::mark_failed(id, &err).await;
                if let Ok(r) = messaging_api::get_payload_queue(None, 1).await { pr.set(Some(r)); }
            });
        })
    };
    let on_view_log = {
        let delivery_log = delivery_log.clone(); let log_id = log_payload_id.clone();
        Callback::from(move |id: Uuid| {
            let dl = delivery_log.clone(); let li = log_id.clone();
            li.set(Some(id));
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(l) = messaging_api::get_delivery_log(id).await { dl.set(l); }
            });
        })
    };
    let on_queue_refresh = {
        let pr = payload_resp.clone(); let pl = payload_loading.clone();
        Callback::from(move |_: ()| {
            let pr = pr.clone(); let pl = pl.clone();
            pl.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(r) = messaging_api::get_payload_queue(None, 1).await { pr.set(Some(r)); }
                pl.set(false);
            });
        })
    };

    html! {
        <div>
            <header class="app-header">
                <div>
                    <h1>{"CivicSort"}</h1>
                    <div class="subtitle">{"Messaging & Notifications"}</div>
                </div>
            </header>

            <div class="search-container">
                <div class="admin-tabs">
                    <button class={classes!("admin-tab", if *active_tab == "inbox" { "admin-tab-active" } else { "" })} onclick={set_tab("inbox")}>{"Inbox"}</button>
                    <button class={classes!("admin-tab", if *active_tab == "queue" { "admin-tab-active" } else { "" })} onclick={set_tab("queue")}>{"Payload Queue"}</button>
                    <button class={classes!("admin-tab", if *active_tab == "config" { "admin-tab-active" } else { "" })} onclick={set_tab("config")}>{"Templates & Triggers"}</button>
                </div>

                { match (*active_tab).as_str() {
                    "inbox" => html! {
                        <NotificationCenter
                            inbox={(*inbox).clone()}
                            is_loading={*inbox_loading}
                            on_read={on_read}
                            on_dismiss={on_dismiss}
                            on_read_all={on_read_all}
                            on_refresh={on_inbox_refresh}
                            on_page_change={on_inbox_page}
                        />
                    },
                    "queue" => html! {
                        <div>
                            { if let Some(ref msg) = *export_result {
                                html! { <div class="validation-success" style="margin-bottom:16px;"><p>{ msg }</p></div> }
                            } else { html! {} }}
                            <PayloadQueue
                                response={(*payload_resp).clone()}
                                is_loading={*payload_loading}
                                on_export={on_export}
                                on_mark_delivered={on_mark_delivered}
                                on_mark_failed={on_mark_failed}
                                on_view_log={on_view_log.clone()}
                                on_refresh={on_queue_refresh}
                            />
                            // Delivery log modal
                            { if log_payload_id.is_some() {
                                let close = {
                                    let li = log_payload_id.clone();
                                    Callback::from(move |_: MouseEvent| li.set(None))
                                };
                                html! {
                                    <div class="config-panel" style="margin-top:16px;">
                                        <div style="display:flex;justify-content:space-between;">
                                            <h3>{"Delivery Log"}</h3>
                                            <button onclick={close} class="btn-tiny">{"Close"}</button>
                                        </div>
                                        <div class="overview-table">
                                            <table>
                                                <thead><tr><th>{"Action"}</th><th>{"Status"}</th><th>{"Details"}</th><th>{"Time"}</th></tr></thead>
                                                <tbody>
                                                { for delivery_log.iter().map(|e| html! {
                                                    <tr>
                                                        <td>{ &e.action }</td>
                                                        <td>{ &e.status_after }</td>
                                                        <td>{ e.details.as_deref().unwrap_or("-") }</td>
                                                        <td>{ &e.performed_at }</td>
                                                    </tr>
                                                })}
                                                </tbody>
                                            </table>
                                        </div>
                                    </div>
                                }
                            } else { html! {} }}
                        </div>
                    },
                    "config" => html! {
                        <div>
                            <h3 class="section-title">{"Notification Templates"}</h3>
                            { if templates.is_empty() {
                                html! { <p style="color:#64748b;">{"No templates configured"}</p> }
                            } else {
                                html! {
                                    <div class="overview-table">
                                        <table>
                                            <thead><tr><th>{"Name"}</th><th>{"Channel"}</th><th>{"Active"}</th></tr></thead>
                                            <tbody>
                                            { for templates.iter().map(|t| html! {
                                                <tr>
                                                    <td>{ &t.name }</td>
                                                    <td>{ &t.channel }</td>
                                                    <td>{ if t.is_active { "Yes" } else { "No" } }</td>
                                                </tr>
                                            })}
                                            </tbody>
                                        </table>
                                    </div>
                                }
                            }}

                            <h3 class="section-title" style="margin-top:24px;">{"Trigger Rules"}</h3>
                            { if triggers.is_empty() {
                                html! { <p style="color:#64748b;">{"No trigger rules configured"}</p> }
                            } else {
                                html! {
                                    <div class="overview-table">
                                        <table>
                                            <thead><tr><th>{"Name"}</th><th>{"Event"}</th><th>{"Channel"}</th><th>{"Priority"}</th></tr></thead>
                                            <tbody>
                                            { for triggers.iter().map(|r| html! {
                                                <tr>
                                                    <td>{ &r.name }</td>
                                                    <td>{ &r.event }</td>
                                                    <td>{ &r.channel }</td>
                                                    <td>{ r.priority }</td>
                                                </tr>
                                            })}
                                            </tbody>
                                        </table>
                                    </div>
                                }
                            }}
                        </div>
                    },
                    _ => html! {},
                }}
            </div>
        </div>
    }
}
