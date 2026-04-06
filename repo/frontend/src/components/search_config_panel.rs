use web_sys::HtmlInputElement;
use yew::prelude::*;

use crate::services::api::KbSearchConfig;

#[derive(Properties, PartialEq)]
pub struct SearchConfigPanelProps {
    pub config: Option<KbSearchConfig>,
    pub on_save: Callback<KbSearchConfig>,
    pub is_saving: bool,
    pub is_admin: bool,
}

#[function_component(SearchConfigPanel)]
pub fn search_config_panel(props: &SearchConfigPanelProps) -> Html {
    let expanded = use_state(|| false);

    if !props.is_admin {
        return html! {};
    }

    let config = match &props.config {
        Some(c) => c.clone(),
        None => return html! {},
    };

    let local_config = use_state(|| config.clone());

    // Reset local config when prop changes
    {
        let local_config = local_config.clone();
        let config = config.clone();
        use_effect_with(config.clone(), move |c| {
            local_config.set(c.clone());
            || ()
        });
    }

    let toggle = {
        let expanded = expanded.clone();
        Callback::from(move |_: MouseEvent| expanded.set(!*expanded))
    };

    if !*expanded {
        return html! {
            <div class="config-panel">
                <button onclick={toggle}
                        style="background:none;border:none;cursor:pointer;font-size:14px;color:#2563eb;">
                    {"+ Search Weight Configuration"}
                </button>
            </div>
        };
    }

    let make_handler = |field: &'static str, local_config: UseStateHandle<KbSearchConfig>| {
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Ok(val) = input.value().parse::<f32>() {
                let mut c = (*local_config).clone();
                match field {
                    "name_exact" => c.name_exact_weight = val,
                    "name_prefix" => c.name_prefix_weight = val,
                    "name_fuzzy" => c.name_fuzzy_weight = val,
                    "alias_exact" => c.alias_exact_weight = val,
                    "alias_fuzzy" => c.alias_fuzzy_weight = val,
                    "category" => c.category_boost = val,
                    "region" => c.region_boost = val,
                    "recency" => c.recency_boost = val,
                    "threshold" => c.fuzzy_threshold = val,
                    "max_results" => c.max_results = val as i32,
                    _ => {}
                }
                local_config.set(c);
            }
        })
    };

    let on_save = {
        let local_config = local_config.clone();
        let cb = props.on_save.clone();
        Callback::from(move |_: MouseEvent| {
            cb.emit((*local_config).clone());
        })
    };

    let lc = &*local_config;

    html! {
        <div class="config-panel">
            <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:16px;">
                <h3>{"Search Weight Configuration"}</h3>
                <button onclick={toggle}
                        style="background:none;border:none;cursor:pointer;font-size:14px;color:#64748b;">
                    {"- Collapse"}
                </button>
            </div>

            <div class="config-grid">
                <div class="config-field">
                    <label>{"Exact Name Weight"}</label>
                    <input type="number" step="0.1" value={lc.name_exact_weight.to_string()}
                           oninput={make_handler("name_exact", local_config.clone())} />
                </div>
                <div class="config-field">
                    <label>{"Prefix Name Weight"}</label>
                    <input type="number" step="0.1" value={lc.name_prefix_weight.to_string()}
                           oninput={make_handler("name_prefix", local_config.clone())} />
                </div>
                <div class="config-field">
                    <label>{"Fuzzy Name Weight"}</label>
                    <input type="number" step="0.1" value={lc.name_fuzzy_weight.to_string()}
                           oninput={make_handler("name_fuzzy", local_config.clone())} />
                </div>
                <div class="config-field">
                    <label>{"Exact Alias Weight"}</label>
                    <input type="number" step="0.1" value={lc.alias_exact_weight.to_string()}
                           oninput={make_handler("alias_exact", local_config.clone())} />
                </div>
                <div class="config-field">
                    <label>{"Fuzzy Alias Weight"}</label>
                    <input type="number" step="0.1" value={lc.alias_fuzzy_weight.to_string()}
                           oninput={make_handler("alias_fuzzy", local_config.clone())} />
                </div>
                <div class="config-field">
                    <label>{"Category Boost"}</label>
                    <input type="number" step="0.1" value={lc.category_boost.to_string()}
                           oninput={make_handler("category", local_config.clone())} />
                </div>
                <div class="config-field">
                    <label>{"Region Boost"}</label>
                    <input type="number" step="0.1" value={lc.region_boost.to_string()}
                           oninput={make_handler("region", local_config.clone())} />
                </div>
                <div class="config-field">
                    <label>{"Recency Boost"}</label>
                    <input type="number" step="0.1" value={lc.recency_boost.to_string()}
                           oninput={make_handler("recency", local_config.clone())} />
                </div>
                <div class="config-field">
                    <label>{"Fuzzy Threshold"}</label>
                    <input type="number" step="0.01" min="0" max="1"
                           value={lc.fuzzy_threshold.to_string()}
                           oninput={make_handler("threshold", local_config.clone())} />
                </div>
                <div class="config-field">
                    <label>{"Max Results"}</label>
                    <input type="number" step="1" min="1" max="100"
                           value={lc.max_results.to_string()}
                           oninput={make_handler("max_results", local_config.clone())} />
                </div>
            </div>

            <div style="margin-top:16px;">
                <button onclick={on_save}
                        disabled={props.is_saving}
                        style="padding:8px 20px;background:#2563eb;color:white;border:none;border-radius:6px;cursor:pointer;">
                    { if props.is_saving { "Saving..." } else { "Save Configuration" } }
                </button>
            </div>
        </div>
    }
}
