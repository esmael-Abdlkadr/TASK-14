use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlSelectElement;
use yew::prelude::*;

use crate::pages::admin_page::AdminPage;
use crate::pages::bulk_data_page::BulkDataPage;
use crate::pages::inspection_page::InspectionPage;
use crate::pages::kb_search_page::KbSearchPage;
use crate::pages::login_page::LoginPage;
use crate::pages::messaging_page::MessagingPage;
use crate::pages::review_page::ReviewPage;
use crate::services::api;

#[function_component(App)]
pub fn app() -> Html {
    let authenticated = use_state(|| is_session_stored());
    let current_page = use_state(|| "kb".to_string());
    let user_role = use_state(|| get_stored_role());
    let username = use_state(|| get_stored_username());
    let login_error = use_state(|| None::<String>);

    let on_login = {
        let authenticated = authenticated.clone();
        let user_role = user_role.clone();
        let username = username.clone();
        let login_error = login_error.clone();
        Callback::from(move |(user, pass): (String, String)| {
            let authenticated = authenticated.clone();
            let user_role = user_role.clone();
            let username = username.clone();
            let login_error = login_error.clone();
            spawn_local(async move {
                match api::login(&user, &pass).await {
                    Ok(resp) => {
                        store_session(&resp.token, &resp.role, &resp.username);
                        authenticated.set(true);
                        user_role.set(Some(resp.role));
                        username.set(Some(resp.username));
                        login_error.set(None);
                    }
                    Err(e) => login_error.set(Some(e)),
                }
            });
        })
    };

    let on_logout = {
        let authenticated = authenticated.clone();
        let user_role = user_role.clone();
        let username = username.clone();
        let current_page = current_page.clone();
        Callback::from(move |_: MouseEvent| {
            clear_session();
            authenticated.set(false);
            user_role.set(None);
            username.set(None);
            current_page.set("kb".to_string());
        })
    };

    let set_page = |page: &'static str| {
        let current_page = current_page.clone();
        Callback::from(move |_: MouseEvent| current_page.set(page.to_string()))
    };

    if !*authenticated {
        return html! {
            <div>
                { if let Some(ref err) = *login_error {
                    html! { <div class="error-message" style="max-width:400px;margin:20px auto;">{ err }</div> }
                } else { html! {} }}
                <LoginPage on_login={on_login} />
            </div>
        };
    }

    let role = user_role.as_deref().unwrap_or("");
    let is_admin = role == "operations_admin" || role == "OperationsAdmin";
    let is_manager = role == "department_manager" || role == "DepartmentManager";
    let is_reviewer = role == "reviewer" || role == "Reviewer";
    let is_admin_or_manager = is_admin || is_manager;

    html! {
        <div>
            // Navigation bar
            <nav class="app-nav">
                <div class="nav-brand" onclick={set_page("kb")}>{"CivicSort"}</div>
                <div class="nav-links">
                    <button class={nav_class(&current_page, "kb")} onclick={set_page("kb")}>{"Knowledge Base"}</button>
                    <button class={nav_class(&current_page, "inspection")} onclick={set_page("inspection")}>{"Inspections"}</button>
                    { if is_reviewer || is_admin {
                        html! { <button class={nav_class(&current_page, "review")} onclick={set_page("review")}>{"Reviews"}</button> }
                    } else { html! {} }}
                    { if is_admin_or_manager {
                        html! { <button class={nav_class(&current_page, "admin")} onclick={set_page("admin")}>{"Admin"}</button> }
                    } else { html! {} }}
                    { if is_admin {
                        html! {
                            <>
                                <button class={nav_class(&current_page, "messaging")} onclick={set_page("messaging")}>{"Messaging"}</button>
                                <button class={nav_class(&current_page, "bulk")} onclick={set_page("bulk")}>{"Bulk Data"}</button>
                            </>
                        }
                    } else { html! {} }}
                </div>
                <div class="nav-user">
                    <span class="nav-username">{ username.as_deref().unwrap_or("") }</span>
                    <span class="nav-role">{ role }</span>
                    <button class="btn-small nav-logout" onclick={on_logout}>{"Logout"}</button>
                </div>
            </nav>

            // Page content
            { match (*current_page).as_str() {
                "kb" => html! { <KbSearchPage /> },
                "inspection" => html! { <InspectionPage /> },
                "review" => html! { <ReviewPage /> },
                "admin" => html! { <AdminPage /> },
                "messaging" => html! { <MessagingPage /> },
                "bulk" => html! { <BulkDataPage /> },
                _ => html! { <KbSearchPage /> },
            }}
        </div>
    }
}

fn nav_class(current: &str, page: &str) -> String {
    if current == page {
        "nav-link nav-link-active".to_string()
    } else {
        "nav-link".to_string()
    }
}

fn is_session_stored() -> bool {
    get_storage().and_then(|s| s.get_item("session_token").ok().flatten()).is_some()
}

fn get_stored_role() -> Option<String> {
    get_storage().and_then(|s| s.get_item("user_role").ok().flatten())
}

fn get_stored_username() -> Option<String> {
    get_storage().and_then(|s| s.get_item("username").ok().flatten())
}

fn store_session(token: &str, role: &str, username: &str) {
    if let Some(s) = get_storage() {
        let _ = s.set_item("session_token", token);
        let _ = s.set_item("user_role", role);
        let _ = s.set_item("username", username);
    }
}

fn clear_session() {
    if let Some(s) = get_storage() {
        let _ = s.remove_item("session_token");
        let _ = s.remove_item("user_role");
        let _ = s.remove_item("username");
    }
}

fn get_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.session_storage().ok()?
}
