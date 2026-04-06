use web_sys::HtmlSelectElement;
use yew::prelude::*;

use crate::services::admin_api;

#[function_component(AdminPage)]
pub fn admin_page() -> Html {
    let active_tab = use_state(|| "dashboard".to_string());
    let kpis = use_state(|| None::<admin_api::DashboardKpis>);
    let user_ov = use_state(|| None::<admin_api::UserOverview>);
    let item_ov = use_state(|| None::<admin_api::ItemOverview>);
    let work_ov = use_state(|| None::<admin_api::WorkOrderOverview>);
    let campaigns = use_state(|| Vec::<admin_api::Campaign>::new());
    let tags = use_state(|| Vec::<admin_api::Tag>::new());
    let is_loading = use_state(|| false);
    let error = use_state(|| None::<String>);
    let report_status = use_state(|| None::<String>);
    let report_type = use_state(|| "kpi_summary".to_string());
    let report_format = use_state(|| "csv".to_string());

    // Load data on tab change
    {
        let tab = active_tab.clone();
        let kpis = kpis.clone(); let user_ov = user_ov.clone();
        let item_ov = item_ov.clone(); let work_ov = work_ov.clone();
        let campaigns = campaigns.clone(); let tags = tags.clone();
        let is_loading = is_loading.clone(); let error = error.clone();

        use_effect_with((*tab).clone(), move |tab| {
            let tab = tab.clone();
            let kpis = kpis.clone(); let user_ov = user_ov.clone();
            let item_ov = item_ov.clone(); let work_ov = work_ov.clone();
            let campaigns = campaigns.clone(); let tags = tags.clone();
            let is_loading = is_loading.clone(); let error = error.clone();

            wasm_bindgen_futures::spawn_local(async move {
                is_loading.set(true); error.set(None);
                match tab.as_str() {
                    "dashboard" => {
                        if let Ok(k) = admin_api::get_dashboard(None, None).await { kpis.set(Some(k)); }
                    }
                    "users" => {
                        if let Ok(u) = admin_api::get_user_overview().await { user_ov.set(Some(u)); }
                    }
                    "items" => {
                        if let Ok(i) = admin_api::get_item_overview().await { item_ov.set(Some(i)); }
                    }
                    "workorders" => {
                        if let Ok(w) = admin_api::get_workorder_overview().await { work_ov.set(Some(w)); }
                    }
                    "campaigns" => {
                        if let Ok(c) = admin_api::get_campaigns(None, 1).await { campaigns.set(c); }
                        if let Ok(t) = admin_api::get_tags().await { tags.set(t); }
                    }
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
                    <div class="subtitle">{"Admin Console"}</div>
                </div>
            </header>

            <div class="search-container">
                // Tab nav
                <div class="admin-tabs">
                    { for ["dashboard", "users", "items", "workorders", "campaigns", "reports"].iter().map(|t| {
                        let is_active = *active_tab == *t;
                        html! {
                            <button
                                class={classes!("admin-tab", if is_active { "admin-tab-active" } else { "" })}
                                onclick={set_tab(t)}
                            >{ t.replace("workorders", "work orders") }</button>
                        }
                    })}
                </div>

                { if *is_loading {
                    html! { <div class="loading-spinner">{"Loading..."}</div> }
                } else if let Some(ref err) = *error {
                    html! { <div class="error-message">{ err }</div> }
                } else {
                    match (*active_tab).as_str() {
                        "dashboard" => render_dashboard(&kpis),
                        "users" => render_user_overview(&user_ov),
                        "items" => render_item_overview(&item_ov),
                        "workorders" => render_workorder_overview(&work_ov),
                        "campaigns" => render_campaigns(&campaigns, &tags),
                        "reports" => render_reports(report_status.clone(), report_type.clone(), report_format.clone()),
                        _ => html! {},
                    }
                }}
            </div>
        </div>
    }
}

fn render_dashboard(kpis: &Option<admin_api::DashboardKpis>) -> Html {
    let k = match kpis {
        Some(k) => k,
        None => return html! { <div class="empty-state"><p>{"Loading dashboard..."}</p></div> },
    };

    html! {
        <div>
            <h2 class="section-title">{"KPI Dashboard"}</h2>

            // KPI cards
            <div class="kpi-grid">
                { render_kpi_card(&k.sorting_conversion_rate, "%") }
                { render_kpi_card(&k.template_reuse_rate, "x") }
                { render_kpi_card(&k.retention_30d, "%") }
                { render_kpi_card(&k.retention_60d, "%") }
                { render_kpi_card(&k.retention_90d, "%") }
            </div>

            // Counter cards
            <h3 class="section-title" style="margin-top:24px;">{"Platform Metrics"}</h3>
            <div class="counter-grid">
                { render_counter("Active Users", k.active_users, "primary") }
                { render_counter("Tasks Completed", k.total_tasks_completed, "success") }
                { render_counter("Reviews Done", k.total_reviews_completed, "info") }
                { render_counter("KB Entries", k.total_kb_entries, "info") }
                { render_counter("Active Campaigns", k.active_campaigns, "warning") }
                { render_counter("Overdue Tasks", k.overdue_tasks, "danger") }
                { render_counter("Pending Reviews", k.pending_reviews, "warning") }
            </div>
        </div>
    }
}

fn render_kpi_card(m: &admin_api::KpiMetric, unit: &str) -> Html {
    let trend_class = if m.trend > 0.0 { "trend-up" } else if m.trend < 0.0 { "trend-down" } else { "trend-flat" };
    let trend_arrow = if m.trend > 0.0 { "^" } else if m.trend < 0.0 { "v" } else { "-" };

    html! {
        <div class="kpi-card">
            <div class="kpi-label">{ &m.label }</div>
            <div class="kpi-value">{ format!("{:.1}{}", m.current, unit) }</div>
            <div class={classes!("kpi-trend", trend_class)}>
                { format!("{} {:.1}%", trend_arrow, m.trend.abs()) }
            </div>
        </div>
    }
}

fn render_counter(label: &str, value: i64, color: &str) -> Html {
    html! {
        <div class={classes!("counter-card", format!("counter-{}", color))}>
            <div class="counter-value">{ value }</div>
            <div class="counter-label">{ label }</div>
        </div>
    }
}

fn render_user_overview(ov: &Option<admin_api::UserOverview>) -> Html {
    let u = match ov {
        Some(u) => u,
        None => return html! { <div class="empty-state"><p>{"Loading..."}</p></div> },
    };
    html! {
        <div>
            <h2 class="section-title">{"User Overview"}</h2>
            <div class="counter-grid">
                { render_counter("Total Users", u.total_users, "primary") }
                { render_counter("Recent Logins (7d)", u.recent_logins, "success") }
            </div>
            <div class="overview-tables">
                <div class="overview-table">
                    <h3>{"By Role"}</h3>
                    <table><thead><tr><th>{"Role"}</th><th>{"Count"}</th></tr></thead>
                    <tbody>
                    { for u.by_role.iter().map(|r| html! {
                        <tr><td>{ r.display_label() }</td><td>{ r.count }</td></tr>
                    })}
                    </tbody></table>
                </div>
                <div class="overview-table">
                    <h3>{"By Status"}</h3>
                    <table><thead><tr><th>{"Status"}</th><th>{"Count"}</th></tr></thead>
                    <tbody>
                    { for u.by_status.iter().map(|s| html! {
                        <tr><td>{ s.display_label() }</td><td>{ s.count }</td></tr>
                    })}
                    </tbody></table>
                </div>
            </div>
        </div>
    }
}

fn render_item_overview(ov: &Option<admin_api::ItemOverview>) -> Html {
    let i = match ov {
        Some(i) => i,
        None => return html! { <div class="empty-state"><p>{"Loading..."}</p></div> },
    };
    html! {
        <div>
            <h2 class="section-title">{"Item Overview (Knowledge Base)"}</h2>
            <div class="counter-grid">
                { render_counter("Total Entries", i.total_kb_entries, "primary") }
                { render_counter("Active Entries", i.active_entries, "success") }
                { render_counter("Categories", i.total_categories, "info") }
                { render_counter("Updated (7d)", i.recent_updates, "warning") }
            </div>
            <div class="overview-table" style="margin-top:16px;">
                <h3>{"Entries by Region"}</h3>
                <table><thead><tr><th>{"Region"}</th><th>{"Count"}</th></tr></thead>
                <tbody>
                { for i.entries_by_region.iter().map(|r| html! {
                    <tr><td>{ r.display_label() }</td><td>{ r.count }</td></tr>
                })}
                </tbody></table>
            </div>
        </div>
    }
}

fn render_workorder_overview(ov: &Option<admin_api::WorkOrderOverview>) -> Html {
    let w = match ov {
        Some(w) => w,
        None => return html! { <div class="empty-state"><p>{"Loading..."}</p></div> },
    };
    html! {
        <div>
            <h2 class="section-title">{"Work Order Overview"}</h2>
            <div class="counter-grid">
                { render_counter("Templates", w.total_templates, "primary") }
                { render_counter("Active Schedules", w.active_schedules, "info") }
                { render_counter("Total Instances", w.total_instances, "info") }
            </div>
            <div class="kpi-grid" style="margin-top:16px;">
                <div class="kpi-card">
                    <div class="kpi-label">{"Completion Rate"}</div>
                    <div class="kpi-value">{ format!("{:.1}%", w.completion_rate) }</div>
                </div>
                <div class="kpi-card">
                    <div class="kpi-label">{"Avg Completion Time"}</div>
                    <div class="kpi-value">{ format!("{:.1}h", w.avg_completion_time_hours) }</div>
                </div>
            </div>
            <div class="overview-table" style="margin-top:16px;">
                <h3>{"Instances by Status"}</h3>
                <table><thead><tr><th>{"Status"}</th><th>{"Count"}</th></tr></thead>
                <tbody>
                { for w.by_status.iter().map(|s| html! {
                    <tr><td>{ s.display_label() }</td><td>{ s.count }</td></tr>
                })}
                </tbody></table>
            </div>
        </div>
    }
}

fn render_campaigns(campaigns: &[admin_api::Campaign], tags: &[admin_api::Tag]) -> Html {
    html! {
        <div>
            <h2 class="section-title">{"Campaign Management"}</h2>
            { if campaigns.is_empty() {
                html! { <div class="empty-state"><p>{"No campaigns yet."}</p></div> }
            } else {
                html! {
                    <div>
                        { for campaigns.iter().map(|c| html! {
                            <div class="task-card">
                                <div class="task-card-header">
                                    <div class="task-card-title">{ &c.name }</div>
                                    <span class={classes!("task-status-badge", match c.status.as_str() {
                                        "draft" => "status-scheduled",
                                        "scheduled" => "status-progress",
                                        "active" => "status-completed",
                                        "completed" => "status-completed",
                                        "cancelled" => "status-missed",
                                        _ => "",
                                    })}>{ &c.status }</span>
                                </div>
                                <div class="task-card-meta">
                                    <span>{ format!("{} - {}", c.start_date, c.end_date) }</span>
                                    { if let Some(ref r) = c.target_region {
                                        html! { <span>{ format!("Region: {}", r) }</span> }
                                    } else { html! {} }}
                                </div>
                                { if let Some(ref desc) = c.description {
                                    html! { <p style="font-size:13px;color:#64748b;margin-top:8px;">{ desc }</p> }
                                } else { html! {} }}
                            </div>
                        })}
                    </div>
                }
            }}

            // Tags section
            <h3 class="section-title" style="margin-top:24px;">{"Tags"}</h3>
            <div class="tag-list">
                { for tags.iter().map(|t| {
                    let bg = t.color.as_deref().unwrap_or("#e2e8f0");
                    html! {
                        <span class="tag-pill" style={format!("background:{};", bg)}>
                            { &t.name }
                        </span>
                    }
                })}
                { if tags.is_empty() {
                    html! { <span style="color:#64748b;">{"No tags defined"}</span> }
                } else { html! {} }}
            </div>
        </div>
    }
}

fn render_reports(
    report_status: UseStateHandle<Option<String>>,
    report_type: UseStateHandle<String>,
    report_format: UseStateHandle<String>,
) -> Html {

    let on_generate = {
        let rt = report_type.clone();
        let rf = report_format.clone();
        let status = report_status.clone();
        Callback::from(move |_: MouseEvent| {
            let rt = (*rt).clone();
            let rf = (*rf).clone();
            let status = status.clone();
            status.set(Some("Generating...".into()));
            wasm_bindgen_futures::spawn_local(async move {
                match admin_api::generate_report(&rt, &rf, None, None).await {
                    Ok(_) => status.set(Some("Report downloaded successfully.".into())),
                    Err(e) => status.set(Some(format!("Error: {}", e))),
                }
            });
        })
    };

    html! {
        <div>
            <h2 class="section-title">{"Report Export"}</h2>
            <div class="search-box">
                <div class="search-filters" style="gap:12px;">
                    <div>
                        <label style="font-size:13px;display:block;margin-bottom:4px;">{"Report Type"}</label>
                        <select onchange={{
                            let rt = report_type.clone();
                            Callback::from(move |e: Event| {
                                let s: HtmlSelectElement = e.target_unchecked_into();
                                rt.set(s.value());
                            })
                        }}>
                            <option value="kpi_summary">{"KPI Summary"}</option>
                            <option value="user_overview">{"User Overview"}</option>
                            <option value="task_overview">{"Task Overview"}</option>
                            <option value="campaign_report">{"Campaign Report"}</option>
                            <option value="audit_report">{"Audit Report"}</option>
                        </select>
                    </div>
                    <div>
                        <label style="font-size:13px;display:block;margin-bottom:4px;">{"Format"}</label>
                        <select onchange={{
                            let rf = report_format.clone();
                            Callback::from(move |e: Event| {
                                let s: HtmlSelectElement = e.target_unchecked_into();
                                rf.set(s.value());
                            })
                        }}>
                            <option value="csv">{"CSV"}</option>
                            <option value="pdf">{"PDF"}</option>
                        </select>
                    </div>
                    <div style="display:flex;align-items:flex-end;">
                        <button onclick={on_generate} class="btn-primary">
                            {"Generate & Download"}
                        </button>
                    </div>
                </div>

                { if let Some(ref st) = *report_status {
                    html! { <p style="margin-top:12px;font-size:14px;color:#64748b;">{ st }</p> }
                } else { html! {} }}
            </div>

            <p style="margin-top:16px;font-size:13px;color:#94a3b8;">
                {"All reports are generated locally without network connectivity. Step-up verification may be required for exports."}
            </p>
        </div>
    }
}
