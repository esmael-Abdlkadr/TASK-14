use web_sys::HtmlInputElement;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct LoginPageProps {
    pub on_login: Callback<(String, String)>,
}

#[function_component(LoginPage)]
pub fn login_page(props: &LoginPageProps) -> Html {
    let username = use_state(String::new);
    let password = use_state(String::new);
    let error = use_state(|| None::<String>);
    let loading = use_state(|| false);

    let on_submit = {
        let username = username.clone();
        let password = password.clone();
        let on_login = props.on_login.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            if !username.is_empty() && !password.is_empty() {
                on_login.emit(((*username).clone(), (*password).clone()));
            }
        })
    };

    html! {
        <div class="login-container">
            <div class="login-card">
                <h1 class="login-title">{"CivicSort"}</h1>
                <p class="login-subtitle">{"Operations Platform"}</p>

                <form onsubmit={on_submit}>
                    <div class="form-field">
                        <label>{"Username"}</label>
                        <input
                            type="text"
                            value={(*username).clone()}
                            oninput={{
                                let u = username.clone();
                                Callback::from(move |e: InputEvent| {
                                    let i: HtmlInputElement = e.target_unchecked_into();
                                    u.set(i.value());
                                })
                            }}
                            placeholder="Enter username"
                            autocomplete="username"
                        />
                    </div>
                    <div class="form-field">
                        <label>{"Password"}</label>
                        <input
                            type="password"
                            value={(*password).clone()}
                            oninput={{
                                let p = password.clone();
                                Callback::from(move |e: InputEvent| {
                                    let i: HtmlInputElement = e.target_unchecked_into();
                                    p.set(i.value());
                                })
                            }}
                            placeholder="Enter password"
                            autocomplete="current-password"
                        />
                    </div>
                    <button type="submit" class="btn-primary" style="width:100%;margin-top:16px;">
                        {"Sign In"}
                    </button>
                </form>
            </div>
        </div>
    }
}
