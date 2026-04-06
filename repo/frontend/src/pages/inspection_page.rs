use uuid::Uuid;
use web_sys::HtmlSelectElement;
use yew::prelude::*;

use crate::components::reminder_inbox::ReminderInboxComponent;
use crate::components::task_detail::TaskDetail;
use crate::components::task_list::TaskList;
use crate::services::inspection_api;

#[function_component(InspectionPage)]
pub fn inspection_page() -> Html {
    // Task list state
    let task_response = use_state(|| None::<inspection_api::TaskListResponse>);
    let is_loading = use_state(|| false);
    let error = use_state(|| None::<String>);
    let status_filter = use_state(|| None::<String>);
    let current_page = use_state(|| 1i64);

    // Task detail state
    let selected_task = use_state(|| None::<inspection_api::TaskInstanceDetail>);
    let detail_loading = use_state(|| false);
    let is_submitting = use_state(|| false);
    let submission_result = use_state(|| None::<inspection_api::SubmissionResponse>);

    // Reminder state
    let reminder_inbox = use_state(|| None::<inspection_api::ReminderInbox>);
    let reminders_loading = use_state(|| false);

    // Load tasks on mount and when filters change
    {
        let task_response = task_response.clone();
        let is_loading = is_loading.clone();
        let error = error.clone();
        let status_filter = status_filter.clone();
        let current_page = current_page.clone();

        use_effect_with(
            ((*status_filter).clone(), *current_page),
            move |(sf, page)| {
                let task_response = task_response.clone();
                let is_loading = is_loading.clone();
                let error = error.clone();
                let sf = sf.clone();
                let page = *page;

                wasm_bindgen_futures::spawn_local(async move {
                    is_loading.set(true);
                    match inspection_api::get_tasks(sf.as_deref(), None, None, page).await {
                        Ok(resp) => {
                            task_response.set(Some(resp));
                            error.set(None);
                        }
                        Err(e) => error.set(Some(e)),
                    }
                    is_loading.set(false);
                });
                || ()
            },
        );
    }

    // Filter change
    let on_filter_change = {
        let status_filter = status_filter.clone();
        let current_page = current_page.clone();
        Callback::from(move |e: Event| {
            let select: HtmlSelectElement = e.target_unchecked_into();
            let val = select.value();
            status_filter.set(if val.is_empty() { None } else { Some(val) });
            current_page.set(1);
        })
    };

    // Page change
    let on_page_change = {
        let current_page = current_page.clone();
        Callback::from(move |page: i64| current_page.set(page))
    };

    // Select task
    let on_select_task = {
        let selected_task = selected_task.clone();
        let detail_loading = detail_loading.clone();
        let submission_result = submission_result.clone();

        Callback::from(move |id: Uuid| {
            let selected_task = selected_task.clone();
            let detail_loading = detail_loading.clone();
            let submission_result = submission_result.clone();

            detail_loading.set(true);
            submission_result.set(None);

            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(task) = inspection_api::get_task(id).await {
                    selected_task.set(Some(task));
                }
                detail_loading.set(false);
            });
        })
    };

    // Back to list
    let on_back = {
        let selected_task = selected_task.clone();
        let submission_result = submission_result.clone();
        Callback::from(move |_: ()| {
            selected_task.set(None);
            submission_result.set(None);
        })
    };

    // Start task
    let on_start = {
        let selected_task = selected_task.clone();
        Callback::from(move |id: Uuid| {
            let selected_task = selected_task.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(_) = inspection_api::start_task(id).await {
                    // Refresh task detail
                    if let Ok(task) = inspection_api::get_task(id).await {
                        selected_task.set(Some(task));
                    }
                }
            });
        })
    };

    // Submit task
    let on_submit = {
        let is_submitting = is_submitting.clone();
        let submission_result = submission_result.clone();
        let selected_task = selected_task.clone();

        Callback::from(
            move |(id, notes, responses): (Uuid, Option<String>, Vec<serde_json::Value>)| {
                let is_submitting = is_submitting.clone();
                let submission_result = submission_result.clone();
                let selected_task = selected_task.clone();

                is_submitting.set(true);
                wasm_bindgen_futures::spawn_local(async move {
                    match inspection_api::submit_task(id, notes, responses).await {
                        Ok(result) => {
                            submission_result.set(Some(result.clone()));
                            if result.valid {
                                // Refresh task
                                if let Ok(task) = inspection_api::get_task(id).await {
                                    selected_task.set(Some(task));
                                }
                            }
                        }
                        Err(e) => {
                            // Try to parse as validation response
                            if let Ok(result) = serde_json::from_str::<inspection_api::SubmissionResponse>(&e) {
                                submission_result.set(Some(result));
                            }
                        }
                    }
                    is_submitting.set(false);
                });
            },
        )
    };

    // Reminder callbacks
    let on_refresh_reminders = {
        let reminder_inbox = reminder_inbox.clone();
        let reminders_loading = reminders_loading.clone();
        Callback::from(move |_: ()| {
            let reminder_inbox = reminder_inbox.clone();
            let reminders_loading = reminders_loading.clone();
            reminders_loading.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(inbox) = inspection_api::get_reminders(None, 1).await {
                    reminder_inbox.set(Some(inbox));
                }
                reminders_loading.set(false);
            });
        })
    };

    let on_read = {
        let on_refresh = on_refresh_reminders.clone();
        Callback::from(move |id: Uuid| {
            let on_refresh = on_refresh.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let _ = inspection_api::mark_reminder_read(id).await;
                on_refresh.emit(());
            });
        })
    };

    let on_dismiss = {
        let on_refresh = on_refresh_reminders.clone();
        Callback::from(move |id: Uuid| {
            let on_refresh = on_refresh.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let _ = inspection_api::dismiss_reminder(id).await;
                on_refresh.emit(());
            });
        })
    };

    let on_read_all = {
        let on_refresh = on_refresh_reminders.clone();
        Callback::from(move |_: ()| {
            let on_refresh = on_refresh.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let _ = inspection_api::mark_all_read().await;
                on_refresh.emit(());
            });
        })
    };

    // Render
    let in_detail = selected_task.is_some() || *detail_loading;

    html! {
        <div>
            <header class="app-header">
                <div>
                    <h1>{"CivicSort"}</h1>
                    <div class="subtitle">{"Inspection Tasks"}</div>
                </div>
            </header>

            <div class="search-container">
                // Reminder inbox
                <ReminderInboxComponent
                    inbox={(*reminder_inbox).clone()}
                    is_loading={*reminders_loading}
                    on_read={on_read}
                    on_dismiss={on_dismiss}
                    on_read_all={on_read_all}
                    on_refresh={on_refresh_reminders}
                />

                { if in_detail {
                    html! {
                        <TaskDetail
                            task={(*selected_task).clone()}
                            is_loading={*detail_loading}
                            is_submitting={*is_submitting}
                            submission_result={(*submission_result).clone()}
                            on_start={on_start}
                            on_submit={on_submit}
                            on_back={on_back}
                        />
                    }
                } else {
                    html! {
                        <div>
                            // Filters
                            <div class="search-box">
                                <div class="search-filters">
                                    <select onchange={on_filter_change}>
                                        <option value="">{"All Statuses"}</option>
                                        <option value="scheduled">{"Scheduled"}</option>
                                        <option value="in_progress">{"In Progress"}</option>
                                        <option value="submitted">{"Submitted"}</option>
                                        <option value="completed">{"Completed"}</option>
                                        <option value="overdue">{"Overdue"}</option>
                                        <option value="missed">{"Missed"}</option>
                                        <option value="makeup">{"Makeup"}</option>
                                    </select>
                                </div>
                            </div>

                            <TaskList
                                response={(*task_response).clone()}
                                is_loading={*is_loading}
                                error={(*error).clone()}
                                on_select_task={on_select_task}
                                on_page_change={on_page_change}
                            />
                        </div>
                    }
                }}
            </div>
        </div>
    }
}
