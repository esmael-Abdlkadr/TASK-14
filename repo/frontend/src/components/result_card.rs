use uuid::Uuid;
use yew::prelude::*;

use crate::services::api::KbSearchResult;

#[derive(Properties, PartialEq)]
pub struct ResultCardProps {
    pub result: KbSearchResult,
    pub on_view_versions: Callback<Uuid>,
}

#[function_component(ResultCard)]
pub fn result_card(props: &ResultCardProps) -> Html {
    let r = &props.result;

    let match_label = match r.match_type.as_str() {
        "exact" => "Exact match",
        "prefix" => "Prefix match",
        "fuzzy" => "Fuzzy match",
        "alias_exact" => "Alias match",
        "alias_fuzzy" => "Fuzzy alias match",
        _ => &r.match_type,
    };

    let on_versions = {
        let entry_id = r.entry_id;
        let cb = props.on_view_versions.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            cb.emit(entry_id);
        })
    };

    html! {
        <div class="result-card">
            <div class="result-header">
                <div class="result-title">{ &r.item_name }</div>
                <span class="result-score">
                    { format!("Score: {:.1}", r.score) }
                </span>
            </div>

            <div class="result-match-info">
                { match_label }
                { if let Some(ref alias) = r.matched_alias {
                    html! { <span>{ format!(" (matched: \"{}\")", alias) }</span> }
                } else {
                    html! {}
                }}
            </div>

            <div class="result-tags">
                <span class="tag tag-region">{ format!("Region: {}", r.region) }</span>
                <span class="tag tag-version">
                    <a href="#" onclick={on_versions} class="version-badge">
                        { format!("v{}", r.current_version) }
                    </a>
                </span>
                { if let Some(ref cat) = r.category_name {
                    html! { <span class="tag tag-category">{ cat }</span> }
                } else {
                    html! {}
                }}
                <span class="tag tag-disposal">{ &r.disposal_category }</span>
            </div>

            <div class="result-body">
                <h4>{"Disposal Instructions"}</h4>
                <p>{ &r.disposal_instructions }</p>

                { if let Some(ref handling) = r.special_handling {
                    html! {
                        <>
                            <h4>{"Special Handling"}</h4>
                            <p>{ handling }</p>
                        </>
                    }
                } else {
                    html! {}
                }}

                { if let Some(ref notes) = r.contamination_notes {
                    html! {
                        <>
                            <h4>{"Contamination Notes"}</h4>
                            <p>{ notes }</p>
                        </>
                    }
                } else {
                    html! {}
                }}

                { if let Some(ref source) = r.rule_source {
                    html! {
                        <p style="margin-top: 8px; font-size: 12px; color: #94a3b8;">
                            { format!("Source: {} | Effective: {}", source, r.effective_date) }
                        </p>
                    }
                } else {
                    html! {
                        <p style="margin-top: 8px; font-size: 12px; color: #94a3b8;">
                            { format!("Effective: {}", r.effective_date) }
                        </p>
                    }
                }}
            </div>

            { if !r.images.is_empty() {
                html! {
                    <div class="result-images">
                        { for r.images.iter().map(|img| html! {
                            <div>
                                <img
                                    src={img.url.clone()}
                                    alt={img.file_name.clone()}
                                    title={img.caption.clone().unwrap_or_default()}
                                    loading="lazy"
                                />
                                { if let Some(ref cap) = img.caption {
                                    html! { <div class="img-caption">{ cap }</div> }
                                } else {
                                    html! {}
                                }}
                            </div>
                        })}
                    </div>
                }
            } else {
                html! {}
            }}
        </div>
    }
}
