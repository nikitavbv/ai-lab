use {
    std::{sync::{Arc, Mutex}, rc::Rc},
    tracing::info,
    yew::prelude::*,
    yew_router::{prelude::*, navigator},
    tonic_web_wasm_client::Client,
    tonic::{Request, Status},
    wasm_bindgen_futures::spawn_local,
    tracing_wasm::WASMLayerConfigBuilder,
    web_sys::{EventTarget, HtmlInputElement, window},
    wasm_bindgen::JsCast,
    urlencoding::encode,
    rpc::{
        ml_sandbox_service_client::MlSandboxServiceClient,
        GenerateImageRequest,
    },
    crate::{
        components::header::Header,
        pages::{
            task::TaskPage,
            login::LoginPage,
        },
        utils::{client, Route},
    },
};

pub mod components;
pub mod pages;
pub mod utils;

#[derive(Clone)]
struct ModelState {
    inference_started: bool,
    prompt: String,
    result: Option<InferenceResult>,
}

#[derive(Clone, PartialEq)]
enum InferenceResultData {
    Text(String),
    Image(Vec<u8>),
}

#[derive(Clone, PartialEq)]
struct InferenceResult {
    data: InferenceResultData,
    worker: String,
}

enum ModelAction {
    UpdatePrompt(String),
    StartInference,
    SetInferenceResult(InferenceResult),
}

#[derive(Properties, PartialEq)]
pub struct InferenceResultDisplayProps {
    result: InferenceResult,
}

impl Default for ModelState {
    fn default() -> Self {
        Self {
            inference_started: false,
            prompt: "".to_owned(),
            result: None,
        }
    }
}

impl Reducible for ModelState {
    type Action = ModelAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            Self::Action::UpdatePrompt(prompt) => Self {
                prompt,
                ..(*self).clone()
            },
            Self::Action::StartInference => Self {
                inference_started: true,
                result: None,
                ..(*self).clone()
            },
            Self::Action::SetInferenceResult(result) => Self {
                inference_started: false,
                result: Some(result),
                ..(*self).clone()
            },
        }.into()
    }
}

#[function_component(App)]
fn app() -> Html {
    info!("application started");

    html!(
        <div>
            <Header />
            <BrowserRouter>
                <Switch<Route> render={router_switch} />
            </BrowserRouter>
        </div>
    )
}

fn router_switch(route: Route) -> Html {
    match route {
        Route::Home => html!(<Home />),
        Route::Login => html!(<LoginPage />),
        Route::Task { id }=> html!(<TaskPage task_id={id} />),
    }
}

#[function_component(Home)]
fn home() -> Html {
    let navigator = use_navigator().unwrap();
    let client = Arc::new(Mutex::new(client()));
    let state = use_reducer(ModelState::default);

    let on_prompt_change = {
        let state = state.clone();

        Callback::from(move |e: Event| {
            let target: Option<EventTarget> = e.target();
            let input = target.and_then(|t| t.dyn_into::<HtmlInputElement>().ok());
            if let Some(input) = input {
                state.dispatch(ModelAction::UpdatePrompt(input.value()));
            }
        })
    };

    let run_inference = {
        let state = state.clone();
        let client = client.clone();
        let navigator = navigator.clone();

        let prompt = state.prompt.clone();

        Callback::from(move |_| {
            let client = client.clone();
            let state = state.clone();
            let navigator = navigator.clone();

            let prompt = prompt.clone();

            spawn_local(async move {
                let mut client = client.lock().unwrap();
                let res = client.generate_image(GenerateImageRequest {
                    prompt,
                }).await.unwrap().into_inner();
                navigator.push(&Route::Task {
                    id: res.id,
                });
            });
        })
    };

    let login = Callback::from(move |_| {
        let current_location = window().unwrap().location();
        let redirect_to = format!("{}//{}/login", current_location.protocol().unwrap(), current_location.host().unwrap());
        window().unwrap().location().set_href(&format!("https://access.nikitavbv.com?redirect_to={}", encode(&redirect_to)));
    });

    html!(
        <div>
            <button onclick={login}>{"login"}</button>
            <h1>{"image generation"}</h1>
            <input onchange={on_prompt_change} value={state.prompt.clone()} placeholder={"prompt"}/>
            <button onclick={run_inference}>{"run model"}</button>
        </div>
    )
}

#[function_component(InferenceResultDisplay)]
fn inference_result_display(props: &InferenceResultDisplayProps) -> Html {
    match &props.result.data {
        InferenceResultData::Text(text) => html!(
            <div>
                <div><b>{"Result: "}</b>{ text }</div>
                <div><b>{"Generated by "}</b>{ &props.result.worker }</div>
            </div>
        ),
        InferenceResultData::Image(image) => html!(
            <div>
                <img src={format!("data:image/png;base64, {}", base64::encode(image))} style={"display: block;"} />
                <div><b>{"Generated by "}</b>{ &props.result.worker }</div>
            </div>
        ),
    }
}

fn main() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default_with_config(
        WASMLayerConfigBuilder::new()
            .set_max_level(tracing::Level::INFO)
            .build()
        );
    yew::Renderer::<App>::new().render();
}