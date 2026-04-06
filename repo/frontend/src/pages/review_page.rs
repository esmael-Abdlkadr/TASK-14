use uuid::Uuid;
use web_sys::HtmlSelectElement;
use yew::prelude::*;

use crate::components::review_queue::ReviewQueue;
use crate::components::scorecard_form::ScorecardForm;
use crate::services::review_api;

#[function_component(ReviewPage)]
pub fn review_page() -> Html {
    let queue_response = use_state(|| None::<review_api::ReviewQueueResponse>);
    let is_loading = use_state(|| false);
    let error = use_state(|| None::<String>);
    let status_filter = use_state(|| Some("pending".to_string()));
    let current_page = use_state(|| 1i64);

    // Detail state
    let selected_detail = use_state(|| None::<review_api::AssignmentDetail>);
    let detail_loading = use_state(|| false);
    let is_submitting = use_state(|| false);
    let submit_result = use_state(|| None::<review_api::SubmitReviewResponse>);

    // Load queue
    {
        let queue_response = queue_response.clone();
        let is_loading = is_loading.clone();
        let error = error.clone();
        let status_filter = status_filter.clone();
        let current_page = current_page.clone();

        use_effect_with(
            ((*status_filter).clone(), *current_page),
            move |(sf, page)| {
                let queue_response = queue_response.clone();
                let is_loading = is_loading.clone();
                let error = error.clone();
                let sf = sf.clone();
                let page = *page;
                wasm_bindgen_futures::spawn_local(async move {
                    is_loading.set(true);
                    match review_api::get_review_queue(sf.as_deref(), page).await {
                        Ok(resp) => { queue_response.set(Some(resp)); error.set(None); }
                        Err(e) => error.set(Some(e)),
                    }
                    is_loading.set(false);
                });
                || ()
            },
        );
    }

    let on_filter = {
        let sf = status_filter.clone();
        let cp = current_page.clone();
        Callback::from(move |e: Event| {
            let s: HtmlSelectElement = e.target_unchecked_into();
            let v = s.value();
            sf.set(if v.is_empty() { None } else { Some(v) });
            cp.set(1);
        })
    };

    let on_page = {
        let cp = current_page.clone();
        Callback::from(move |p: i64| cp.set(p))
    };

    let on_select = {
        let selected_detail = selected_detail.clone();
        let detail_loading = detail_loading.clone();
        let submit_result = submit_result.clone();
        Callback::from(move |id: Uuid| {
            let sd = selected_detail.clone();
            let dl = detail_loading.clone();
            let sr = submit_result.clone();
            dl.set(true); sr.set(None);
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(detail) = review_api::get_assignment_detail(id).await {
                    sd.set(Some(detail));
                }
                dl.set(false);
            });
        })
    };

    let on_back = {
        let sd = selected_detail.clone();
        let sr = submit_result.clone();
        Callback::from(move |_: ()| { sd.set(None); sr.set(None); })
    };

    let on_submit = {
        let is_submitting = is_submitting.clone();
        let submit_result = submit_result.clone();
        let selected_detail = selected_detail.clone();
        Callback::from(move |(scores, comment, rec, ack): (Vec<serde_json::Value>, Option<String>, String, bool)| {
            let is_sub = is_submitting.clone();
            let sr = submit_result.clone();
            let sd = selected_detail.clone();
            let assignment_id = sd.as_ref().map(|d| d.assignment.id).unwrap_or_default();
            is_sub.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match review_api::submit_review(assignment_id, scores, comment, rec, ack).await {
                    Ok(result) => {
                        sr.set(Some(result.clone()));
                        if result.valid {
                            if let Ok(detail) = review_api::get_assignment_detail(assignment_id).await {
                                sd.set(Some(detail));
                            }
                        }
                    }
                    Err(e) => {
                        if let Ok(result) = serde_json::from_str::<review_api::SubmitReviewResponse>(&e) {
                            sr.set(Some(result));
                        }
                    }
                }
                is_sub.set(false);
            });
        })
    };

    let on_recuse = {
        let selected_detail = selected_detail.clone();
        Callback::from(move |reason: String| {
            let sd = selected_detail.clone();
            let assignment_id = sd.as_ref().map(|d| d.assignment.id).unwrap_or_default();
            wasm_bindgen_futures::spawn_local(async move {
                let _ = review_api::recuse_assignment(assignment_id, &reason).await;
                sd.set(None);
            });
        })
    };

    let in_detail = selected_detail.is_some() || *detail_loading;

    html! {
        <div>
            <header class="app-header">
                <div>
                    <h1>{"CivicSort"}</h1>
                    <div class="subtitle">{"Review Workspace"}</div>
                </div>
            </header>
            <div class="search-container">
                { if in_detail {
                    if *detail_loading {
                        html! { <div class="loading-spinner">{"Loading review..."}</div> }
                    } else if let Some(ref detail) = *selected_detail {
                        html! {
                            <ScorecardForm
                                detail={detail.clone()}
                                is_submitting={*is_submitting}
                                submit_result={(*submit_result).clone()}
                                on_submit={on_submit}
                                on_recuse={on_recuse}
                                on_back={on_back}
                            />
                        }
                    } else { html! {} }
                } else {
                    html! {
                        <div>
                            <div class="search-box">
                                <div class="search-filters">
                                    <select onchange={on_filter}>
                                        <option value="pending" selected=true>{"Pending"}</option>
                                        <option value="in_progress">{"In Progress"}</option>
                                        <option value="completed">{"Completed"}</option>
                                        <option value="">{"All"}</option>
                                    </select>
                                </div>
                            </div>
                            <ReviewQueue
                                response={(*queue_response).clone()}
                                is_loading={*is_loading}
                                error={(*error).clone()}
                                on_select={on_select}
                                on_page_change={on_page}
                            />
                        </div>
                    }
                }}
            </div>
        </div>
    }
}
