use uuid::Uuid;
use yew::prelude::*;

use crate::components::search_bar::SearchBar;
use crate::components::search_config_panel::SearchConfigPanel;
use crate::components::search_results::SearchResults;
use crate::components::version_history::VersionHistory;
use crate::services::api;

#[function_component(KbSearchPage)]
pub fn kb_search_page() -> Html {
    // State
    let search_response = use_state(|| None::<api::KbSearchResponse>);
    let is_loading = use_state(|| false);
    let error = use_state(|| None::<String>);
    let categories = use_state(Vec::<api::KbCategory>::new);

    // Version history modal
    let version_data = use_state(|| None::<api::KbVersionHistoryResponse>);
    let version_loading = use_state(|| false);
    let version_error = use_state(|| None::<String>);

    // Search config
    let search_config = use_state(|| None::<api::KbSearchConfig>);
    let config_saving = use_state(|| false);

    // Current search params for pagination
    let current_query = use_state(String::new);
    let current_region = use_state(|| None::<String>);
    let current_category = use_state(|| None::<Uuid>);

    // Load categories on mount
    {
        let categories = categories.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(cats) = api::get_categories().await {
                    categories.set(cats);
                }
            });
            || ()
        });
    }

    // Search handler
    let on_search = {
        let search_response = search_response.clone();
        let is_loading = is_loading.clone();
        let error = error.clone();
        let current_query = current_query.clone();
        let current_region = current_region.clone();
        let current_category = current_category.clone();

        Callback::from(move |(query, region, category_id): (String, Option<String>, Option<Uuid>)| {
            let search_response = search_response.clone();
            let is_loading = is_loading.clone();
            let error = error.clone();
            let current_query = current_query.clone();
            let current_region = current_region.clone();
            let current_category = current_category.clone();
            let q = query.clone();
            let r = region.clone();
            let c = category_id;

            current_query.set(q.clone());
            current_region.set(r.clone());
            current_category.set(c);
            is_loading.set(true);
            error.set(None);

            wasm_bindgen_futures::spawn_local(async move {
                match api::search_kb(&q, r.as_deref(), c, 1, 20).await {
                    Ok(resp) => {
                        search_response.set(Some(resp));
                        error.set(None);
                    }
                    Err(e) => {
                        error.set(Some(e));
                        search_response.set(None);
                    }
                }
                is_loading.set(false);
            });
        })
    };

    // Page change handler
    let on_page_change = {
        let search_response = search_response.clone();
        let is_loading = is_loading.clone();
        let error = error.clone();
        let current_query = current_query.clone();
        let current_region = current_region.clone();
        let current_category = current_category.clone();

        Callback::from(move |page: i64| {
            let search_response = search_response.clone();
            let is_loading = is_loading.clone();
            let error = error.clone();
            let q = (*current_query).clone();
            let r = (*current_region).clone();
            let c = *current_category;

            is_loading.set(true);

            wasm_bindgen_futures::spawn_local(async move {
                match api::search_kb(&q, r.as_deref(), c, page, 20).await {
                    Ok(resp) => {
                        search_response.set(Some(resp));
                        error.set(None);
                    }
                    Err(e) => error.set(Some(e)),
                }
                is_loading.set(false);
            });
        })
    };

    // View version history
    let on_view_versions = {
        let version_data = version_data.clone();
        let version_loading = version_loading.clone();
        let version_error = version_error.clone();

        Callback::from(move |entry_id: Uuid| {
            let version_data = version_data.clone();
            let version_loading = version_loading.clone();
            let version_error = version_error.clone();

            version_loading.set(true);
            version_error.set(None);

            wasm_bindgen_futures::spawn_local(async move {
                match api::get_version_history(entry_id).await {
                    Ok(data) => version_data.set(Some(data)),
                    Err(e) => version_error.set(Some(e)),
                }
                version_loading.set(false);
            });
        })
    };

    // Close version modal
    let on_close_versions = {
        let version_data = version_data.clone();
        let version_error = version_error.clone();
        Callback::from(move |_: ()| {
            version_data.set(None);
            version_error.set(None);
        })
    };

    // Save search config
    let on_save_config = {
        let search_config = search_config.clone();
        let config_saving = config_saving.clone();

        Callback::from(move |config: api::KbSearchConfig| {
            let search_config = search_config.clone();
            let config_saving = config_saving.clone();
            config_saving.set(true);

            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(updated) = api::update_search_config(&config).await {
                    search_config.set(Some(updated));
                }
                config_saving.set(false);
            });
        })
    };

    html! {
        <div>
            <header class="app-header">
                <div>
                    <h1>{"CivicSort"}</h1>
                    <div class="subtitle">{"Waste-Sorting Knowledge Base"}</div>
                </div>
            </header>

            <div class="search-container">
                <SearchBar
                    on_search={on_search}
                    categories={(*categories).clone()}
                    is_loading={*is_loading}
                />

                <SearchResults
                    response={(*search_response).clone()}
                    is_loading={*is_loading}
                    error={(*error).clone()}
                    on_page_change={on_page_change}
                    on_view_versions={on_view_versions}
                />

                <SearchConfigPanel
                    config={(*search_config).clone()}
                    on_save={on_save_config}
                    is_saving={*config_saving}
                    is_admin={true}
                />
            </div>

            // Version history modal
            { if version_data.is_some() || *version_loading {
                html! {
                    <VersionHistory
                        data={(*version_data).clone()}
                        is_loading={*version_loading}
                        error={(*version_error).clone()}
                        on_close={on_close_versions}
                    />
                }
            } else {
                html! {}
            }}
        </div>
    }
}
