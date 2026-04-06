use uuid::Uuid;
use yew::prelude::*;

use crate::components::result_card::ResultCard;
use crate::services::api::KbSearchResponse;

#[derive(Properties, PartialEq)]
pub struct SearchResultsProps {
    pub response: Option<KbSearchResponse>,
    pub is_loading: bool,
    pub error: Option<String>,
    pub on_page_change: Callback<i64>,
    pub on_view_versions: Callback<Uuid>,
}

#[function_component(SearchResults)]
pub fn search_results(props: &SearchResultsProps) -> Html {
    if props.is_loading {
        return html! {
            <div class="loading-spinner">
                {"Searching knowledge base..."}
            </div>
        };
    }

    if let Some(ref err) = props.error {
        return html! {
            <div class="error-message">{ err }</div>
        };
    }

    let response = match &props.response {
        Some(r) => r,
        None => {
            return html! {
                <div class="empty-state">
                    <h3>{"Search the Knowledge Base"}</h3>
                    <p>{"Enter an item name, alias, or common misspelling to find waste sorting rules."}</p>
                </div>
            };
        }
    };

    if response.results.is_empty() {
        return html! {
            <div class="empty-state">
                <h3>{"No results found"}</h3>
                <p>{ format!("No entries match \"{}\". Try a different search term or check spelling.", response.query) }</p>
            </div>
        };
    }

    let total_pages = (response.total as f64 / response.page_size as f64).ceil() as i64;
    let current_page = response.page;

    let on_prev = {
        let cb = props.on_page_change.clone();
        let page = current_page;
        Callback::from(move |_: MouseEvent| {
            if page > 1 {
                cb.emit(page - 1);
            }
        })
    };

    let on_next = {
        let cb = props.on_page_change.clone();
        let page = current_page;
        let total = total_pages;
        Callback::from(move |_: MouseEvent| {
            if page < total {
                cb.emit(page + 1);
            }
        })
    };

    html! {
        <div>
            <div class="search-meta">
                { format!("Found {} results for \"{}\" (page {} of {})",
                    response.total, response.query, current_page, total_pages.max(1)) }
            </div>

            { for response.results.iter().map(|result| html! {
                <ResultCard
                    result={result.clone()}
                    on_view_versions={props.on_view_versions.clone()}
                />
            })}

            { if total_pages > 1 {
                html! {
                    <div class="pagination">
                        <button onclick={on_prev} disabled={current_page <= 1}>
                            {"Previous"}
                        </button>
                        <span>{ format!("Page {} of {}", current_page, total_pages) }</span>
                        <button onclick={on_next} disabled={current_page >= total_pages}>
                            {"Next"}
                        </button>
                    </div>
                }
            } else {
                html! {}
            }}
        </div>
    }
}
