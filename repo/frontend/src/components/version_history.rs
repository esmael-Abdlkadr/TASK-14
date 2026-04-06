use yew::prelude::*;

use crate::services::api::KbVersionHistoryResponse;

#[derive(Properties, PartialEq)]
pub struct VersionHistoryProps {
    pub data: Option<KbVersionHistoryResponse>,
    pub is_loading: bool,
    pub error: Option<String>,
    pub on_close: Callback<()>,
}

#[function_component(VersionHistory)]
pub fn version_history(props: &VersionHistoryProps) -> Html {
    let on_close = {
        let cb = props.on_close.clone();
        Callback::from(move |_: MouseEvent| cb.emit(()))
    };

    // Overlay backdrop click
    let on_backdrop = {
        let cb = props.on_close.clone();
        Callback::from(move |_: MouseEvent| cb.emit(()))
    };

    let data = match &props.data {
        Some(d) => d,
        None if props.is_loading => {
            return html! {
                <div style="position:fixed;inset:0;background:rgba(0,0,0,0.5);display:flex;align-items:center;justify-content:center;z-index:100;">
                    <div style="background:white;border-radius:8px;padding:24px;max-width:700px;width:90%;">
                        {"Loading version history..."}
                    </div>
                </div>
            };
        }
        None => return html! {},
    };

    html! {
        <div onclick={on_backdrop}
             style="position:fixed;inset:0;background:rgba(0,0,0,0.5);display:flex;align-items:center;justify-content:center;z-index:100;">
            <div onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}
                 style="background:white;border-radius:8px;padding:24px;max-width:700px;width:90%;max-height:80vh;overflow-y:auto;">

                <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:16px;">
                    <h2 style="font-size:18px;">
                        { format!("Version History: {}", data.item_name) }
                    </h2>
                    <button onclick={on_close}
                            style="border:none;background:none;font-size:20px;cursor:pointer;">
                        {"x"}
                    </button>
                </div>

                { if let Some(ref err) = props.error {
                    html! { <div class="error-message">{ err }</div> }
                } else {
                    html! {}
                }}

                { for data.versions.iter().map(|detail| {
                    let v = &detail.version;
                    html! {
                        <div style="border:1px solid #e2e8f0;border-radius:8px;padding:16px;margin-bottom:12px;">
                            <div style="display:flex;justify-content:space-between;align-items:center;">
                                <strong>{ format!("Version {}", v.version_number) }</strong>
                                <span style="font-size:12px;color:#64748b;">
                                    { &v.created_at }
                                </span>
                            </div>

                            { if let Some(ref summary) = v.change_summary {
                                html! {
                                    <p style="font-size:13px;color:#64748b;margin:8px 0;">
                                        { format!("Change: {}", summary) }
                                    </p>
                                }
                            } else {
                                html! {}
                            }}

                            <div style="margin-top:8px;">
                                <div class="result-tags">
                                    <span class="tag tag-region">{ format!("Region: {}", v.region) }</span>
                                    <span class="tag tag-disposal">{ &v.disposal_category }</span>
                                </div>

                                <p style="font-size:14px;margin-top:8px;">{ &v.disposal_instructions }</p>

                                { if let Some(ref handling) = v.special_handling {
                                    html! { <p style="font-size:13px;color:#64748b;margin-top:4px;">{ format!("Special: {}", handling) }</p> }
                                } else {
                                    html! {}
                                }}

                                <p style="font-size:12px;color:#94a3b8;margin-top:8px;">
                                    { format!("Effective: {}", v.effective_date) }
                                    { if let Some(ref src) = v.rule_source {
                                        format!(" | Source: {}", src)
                                    } else {
                                        String::new()
                                    }}
                                </p>
                            </div>

                            { if !detail.images.is_empty() {
                                html! {
                                    <div class="result-images" style="margin-top:8px;">
                                        { for detail.images.iter().map(|img| html! {
                                            <img
                                                src={img.url.clone()}
                                                alt={img.file_name.clone()}
                                                style="width:80px;height:60px;object-fit:cover;border-radius:4px;"
                                                loading="lazy"
                                            />
                                        })}
                                    </div>
                                }
                            } else {
                                html! {}
                            }}
                        </div>
                    }
                })}
            </div>
        </div>
    }
}
