use uuid::Uuid;
use web_sys::HtmlSelectElement;
use yew::prelude::*;

use crate::services::bulk_data_api;

#[function_component(BulkDataPage)]
pub fn bulk_data_page() -> Html {
    let active_tab = use_state(|| "imports".to_string());
    let imports = use_state(Vec::<bulk_data_api::ImportJob>::new);
    let changes = use_state(|| None::<bulk_data_api::ChangeHistoryResponse>);
    let duplicates = use_state(Vec::<bulk_data_api::DuplicateFlag>::new);
    let merges = use_state(Vec::<bulk_data_api::MergeRequest>::new);
    let is_loading = use_state(|| false);
    let status_msg = use_state(|| None::<String>);

    {
        let tab = active_tab.clone();
        let imports = imports.clone(); let changes = changes.clone();
        let duplicates = duplicates.clone(); let merges = merges.clone();
        let is_loading = is_loading.clone();

        use_effect_with((*tab).clone(), move |tab| {
            let tab = tab.clone();
            wasm_bindgen_futures::spawn_local(async move {
                is_loading.set(true);
                match tab.as_str() {
                    "imports" => { if let Ok(j) = bulk_data_api::list_imports().await { imports.set(j); } }
                    "history" => { if let Ok(c) = bulk_data_api::get_changes(None, 1).await { changes.set(Some(c)); } }
                    "duplicates" => { if let Ok(d) = bulk_data_api::get_duplicates(Some("detected")).await { duplicates.set(d); } }
                    "merges" => { if let Ok(m) = bulk_data_api::get_merge_requests().await { merges.set(m); } }
                    _ => {}
                }
                is_loading.set(false);
            });
            || ()
        });
    }

    let set_tab = |name: &'static str| {
        let tab = active_tab.clone();
        Callback::from(move |_: MouseEvent| tab.set(name.to_string()))
    };

    html! {
        <div>
            <header class="app-header">
                <div>
                    <h1>{"CivicSort"}</h1>
                    <div class="subtitle">{"Bulk Data Management"}</div>
                </div>
            </header>
            <div class="search-container">
                <div class="admin-tabs">
                    { for ["imports", "history", "duplicates", "merges"].iter().map(|t| html! {
                        <button class={classes!("admin-tab", if *active_tab == *t { "admin-tab-active" } else { "" })} onclick={set_tab(t)}>
                            { t.to_string() }
                        </button>
                    })}
                </div>

                { if let Some(ref msg) = *status_msg {
                    html! { <div class="validation-success" style="margin-bottom:16px;"><p>{ msg }</p></div> }
                } else { html! {} }}

                { if *is_loading {
                    html! { <div class="loading-spinner">{"Loading..."}</div> }
                } else {
                    match (*active_tab).as_str() {
                        "imports" => render_imports(&imports),
                        "history" => render_history(&changes),
                        "duplicates" => render_duplicates(&duplicates),
                        "merges" => render_merges(&merges),
                        _ => html! {},
                    }
                }}
            </div>
        </div>
    }
}

fn render_imports(jobs: &[bulk_data_api::ImportJob]) -> Html {
    html! {
        <div>
            <h2 class="section-title">{"Import Jobs"}</h2>
            { if jobs.is_empty() {
                html! { <div class="empty-state"><p>{"No import jobs"}</p></div> }
            } else {
                html! {
                    <div class="overview-table">
                        <table>
                            <thead><tr><th>{"Name"}</th><th>{"Type"}</th><th>{"Status"}</th><th>{"Total"}</th><th>{"Imported"}</th><th>{"Dupes"}</th><th>{"Errors"}</th></tr></thead>
                            <tbody>
                            { for jobs.iter().map(|j| html! {
                                <tr>
                                    <td>{ &j.name }</td><td>{ &j.entity_type }</td>
                                    <td><span class={classes!("task-status-badge", match j.status.as_str() {
                                        "completed" => "status-completed", "failed" => "status-overdue",
                                        "validated" => "status-progress", _ => "status-scheduled",
                                    })}>{ &j.status }</span></td>
                                    <td>{ j.total_rows }</td><td>{ j.imported_rows }</td>
                                    <td>{ j.duplicate_rows }</td><td>{ j.error_rows }</td>
                                </tr>
                            })}
                            </tbody>
                        </table>
                    </div>
                }
            }}
        </div>
    }
}

fn render_history(changes: &Option<bulk_data_api::ChangeHistoryResponse>) -> Html {
    let resp = match changes {
        Some(c) => c,
        None => return html! { <div class="empty-state"><p>{"No changes recorded"}</p></div> },
    };
    html! {
        <div>
            <h2 class="section-title">{ format!("Change History ({} total)", resp.total) }</h2>
            <div class="overview-table">
                <table>
                    <thead><tr><th>{"Entity"}</th><th>{"Operation"}</th><th>{"Field"}</th><th>{"Changed By"}</th><th>{"When"}</th><th>{"Reverted"}</th></tr></thead>
                    <tbody>
                    { for resp.changes.iter().map(|c| html! {
                        <tr>
                            <td>{ format!("{}:{}", c.entity_type, &c.entity_id.to_string()[..8]) }</td>
                            <td>{ &c.operation }</td>
                            <td>{ c.field_name.as_deref().unwrap_or("-") }</td>
                            <td>{ &c.changed_by.to_string()[..8] }</td>
                            <td>{ &c.changed_at }</td>
                            <td>{ if c.reverted_at.is_some() { "Yes" } else { "-" } }</td>
                        </tr>
                    })}
                    </tbody>
                </table>
            </div>
        </div>
    }
}

fn render_duplicates(flags: &[bulk_data_api::DuplicateFlag]) -> Html {
    html! {
        <div>
            <h2 class="section-title">{"Detected Duplicates"}</h2>
            { if flags.is_empty() {
                html! { <div class="empty-state"><p>{"No duplicates detected"}</p></div> }
            } else {
                html! {
                    <div>
                        { for flags.iter().map(|f| html! {
                            <div class="task-card" style="cursor:default;">
                                <div class="task-card-header">
                                    <div>
                                        <span style="font-weight:600;">{ format!("{} duplicate", f.entity_type) }</span>
                                        <span class="task-card-group">{ format!(" | Match: {} ({:.0}%)", f.match_type, f.confidence * 100.0) }</span>
                                    </div>
                                    <span class={classes!("task-status-badge", "status-overdue")}>{ &f.status }</span>
                                </div>
                                <div class="task-card-meta">
                                    <span>{ format!("Source: {}", &f.source_id.to_string()[..8]) }</span>
                                    <span>{ format!("Target: {}", &f.target_id.to_string()[..8]) }</span>
                                </div>
                            </div>
                        })}
                    </div>
                }
            }}
        </div>
    }
}

fn render_merges(merges: &[bulk_data_api::MergeRequest]) -> Html {
    html! {
        <div>
            <h2 class="section-title">{"Merge Requests"}</h2>
            { if merges.is_empty() {
                html! { <div class="empty-state"><p>{"No pending merge requests"}</p></div> }
            } else {
                html! {
                    <div class="overview-table">
                        <table>
                            <thead><tr><th>{"Type"}</th><th>{"Source"}</th><th>{"Target"}</th><th>{"Status"}</th></tr></thead>
                            <tbody>
                            { for merges.iter().map(|m| html! {
                                <tr>
                                    <td>{ &m.entity_type }</td>
                                    <td>{ &m.source_id.to_string()[..8] }</td>
                                    <td>{ &m.target_id.to_string()[..8] }</td>
                                    <td><span class={classes!("task-status-badge", match m.status.as_str() {
                                        "pending" => "status-scheduled", "approved" | "applied" => "status-completed",
                                        "rejected" => "status-overdue", _ => "",
                                    })}>{ &m.status }</span></td>
                                </tr>
                            })}
                            </tbody>
                        </table>
                    </div>
                }
            }}
        </div>
    }
}
