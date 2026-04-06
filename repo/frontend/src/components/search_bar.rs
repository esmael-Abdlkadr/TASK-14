use uuid::Uuid;
use web_sys::HtmlInputElement;
use web_sys::HtmlSelectElement;
use yew::prelude::*;

use crate::services::api::KbCategory;

#[derive(Properties, PartialEq)]
pub struct SearchBarProps {
    pub on_search: Callback<(String, Option<String>, Option<Uuid>)>,
    pub categories: Vec<KbCategory>,
    pub is_loading: bool,
}

#[function_component(SearchBar)]
pub fn search_bar(props: &SearchBarProps) -> Html {
    let query = use_state(String::new);
    let region = use_state(String::new);
    let category_id = use_state(|| None::<Uuid>);

    let on_input = {
        let query = query.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            query.set(input.value());
        })
    };

    let on_region_change = {
        let region = region.clone();
        Callback::from(move |e: Event| {
            let select: HtmlSelectElement = e.target_unchecked_into();
            region.set(select.value());
        })
    };

    let on_category_change = {
        let category_id = category_id.clone();
        Callback::from(move |e: Event| {
            let select: HtmlSelectElement = e.target_unchecked_into();
            let val = select.value();
            if val.is_empty() {
                category_id.set(None);
            } else {
                if let Ok(id) = val.parse::<Uuid>() {
                    category_id.set(Some(id));
                }
            }
        })
    };

    let on_submit = {
        let query = query.clone();
        let region = region.clone();
        let category_id = category_id.clone();
        let on_search = props.on_search.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let q = (*query).clone();
            if !q.trim().is_empty() {
                let r = if region.is_empty() {
                    None
                } else {
                    Some((*region).clone())
                };
                on_search.emit((q, r, *category_id));
            }
        })
    };

    let on_keydown = {
        let query = query.clone();
        let region = region.clone();
        let category_id = category_id.clone();
        let on_search = props.on_search.clone();
        Callback::from(move |e: KeyboardEvent| {
            if e.key() == "Enter" {
                let q = (*query).clone();
                if !q.trim().is_empty() {
                    let r = if region.is_empty() {
                        None
                    } else {
                        Some((*region).clone())
                    };
                    on_search.emit((q, r, *category_id));
                }
            }
        })
    };

    html! {
        <div class="search-box">
            <form onsubmit={on_submit}>
                <div class="search-input-row">
                    <input
                        type="text"
                        placeholder="Search waste sorting rules (e.g., 'plastic bottle', 'styrofoam')..."
                        value={(*query).clone()}
                        oninput={on_input}
                        onkeydown={on_keydown}
                    />
                    <button type="submit" disabled={props.is_loading}>
                        { if props.is_loading { "Searching..." } else { "Search" } }
                    </button>
                </div>
                <div class="search-filters">
                    <select onchange={on_region_change}>
                        <option value="">{"All Regions"}</option>
                        <option value="default">{"Default"}</option>
                        <option value="north">{"North District"}</option>
                        <option value="south">{"South District"}</option>
                        <option value="east">{"East District"}</option>
                        <option value="west">{"West District"}</option>
                    </select>
                    <select onchange={on_category_change}>
                        <option value="">{"All Categories"}</option>
                        { for props.categories.iter().map(|cat| html! {
                            <option value={cat.id.to_string()}>{ &cat.name }</option>
                        })}
                    </select>
                </div>
            </form>
        </div>
    }
}
