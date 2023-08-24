use {
    std::{sync::{Arc, Mutex}, rc::Rc},
    yew::prelude::*,
    yew_router::prelude::*,
    serde::Deserialize,
    gloo_storage::{Storage, LocalStorage},
    web_sys::{EventTarget, HtmlInputElement},
    wasm_bindgen::JsCast,
    stylist::{style, yew::styled_component},
    wasm_bindgen_futures::spawn_local,
    rpc::{CreateTaskRequest, TaskParams, task_params::{Params, ImageGenerationParams as RpcImageGenerationParams}},
    crate::utils::{client_with_token, Route},
};

#[derive(Deserialize, Debug)]
struct HomeQuery {
    enable_chat: Option<bool>,
}

#[derive(Clone)]
struct ImageGenerationParams {
    prompt: String,
    number_of_images: u32,
    number_of_images_custom: bool,
}

enum ImageGenerationParamAction {
    UpdatePrompt(String),
    SelectNumberOfImagesOption(u32),
    SetCustomNumberOfImages(u32),
}

impl Default for ImageGenerationParams {
    fn default() -> Self {
        Self {
            prompt: "".to_owned(),
            number_of_images: 1,
            number_of_images_custom: false,
        }
    }
}

impl Reducible for ImageGenerationParams {
    type Action = ImageGenerationParamAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            Self::Action::UpdatePrompt(prompt) => Self {
                prompt,
                ..(*self).clone()
            },
            Self::Action::SelectNumberOfImagesOption(number_of_images) => Self {
                number_of_images,
                number_of_images_custom: false,
                ..(*self).clone()
            },
            Self::Action::SetCustomNumberOfImages(number_of_images) => Self {
                number_of_images,
                number_of_images_custom: true,
                ..(*self).clone()
            }
        }.into()
    }
}

#[styled_component(HomePage)]
pub fn home() -> Html {
    let navigator = use_navigator().unwrap();
    let params = use_reducer(ImageGenerationParams::default);
    let token: UseStateHandle<Option<String>> = use_state(|| LocalStorage::get("access_token").ok());
    let client = Arc::new(Mutex::new(client_with_token((*token).clone())));

    let location: Option<HomeQuery> = use_location().map(|v| v.query().unwrap());
    let enable_chat = location.and_then(|v| v.enable_chat).unwrap_or(false);

    let on_prompt_change = {
        let params = params.clone();

        Callback::from(move |e: Event| {
            let target: Option<EventTarget> = e.target();
            let input = target.and_then(|t| t.dyn_into::<HtmlInputElement>().ok());
            if let Some(input) = input {
                params.dispatch(ImageGenerationParamAction::UpdatePrompt(input.value()));
            }
        })
    };

    let run_inference = {
        let params = params.clone();
        let client = client.clone();
        let navigator = navigator.clone();

        let prompt = params.prompt.clone();

        Callback::from(move |_| {
            let client = client.clone();
            let navigator = navigator.clone();

            let prompt = prompt.clone();
            let params = params.clone();

            spawn_local(async move {
                let mut client = client.lock().unwrap();
                let res = client.create_task(CreateTaskRequest {
                    params: Some(TaskParams {
                        params: Some(Params::ImageGeneration(RpcImageGenerationParams {
                            iterations: 20,
                            number_of_images: params.number_of_images,
                            prompt,
                        })),
                    }),
                }).await.unwrap().into_inner();
                navigator.push(&Route::Task {
                    id: res.id.unwrap().id,
                });
            });
        })
    };

    let page_style = style!(r#"
        margin: 0 auto;
        width: 600px;
    "#).unwrap();

    let input_style = style!(r#"
        padding: 8px;
        font-size: 14pt;
        border-radius: 5px;
        border: 2px solid white;
        outline: none;
        width: 400px;
        font-family: Inter;
        transition: border-color 0.2s ease-out;
        background-color: white;
        color: black;
  
        :focus {
            border: 2px solid #5695DC;
        }
    "#).unwrap();

    let description_style = style!(r#"
        width: 100%;
        text-align: center;
        user-select: none;
        padding-bottom: 16px;
        display: block;
        font-size: 16pt;
        line-height: 24pt;
        color: white;
    "#).unwrap();

    let generate_image_button_style = style!(r#"
        margin-left: 8px;
        padding: 8px 14px;
        font-size: 14pt;
        background-color: #5695DC;
        color: white;
        border: 2px solid #5695DC;
        border-radius: 5px;
        font-family: Inter;
        cursor: pointer;
        width: 192px;
        user-select: none;
        transition:
            color 0.2s ease-out, 
            background-color 0.2s ease-out;

        :hover {
            background-color: #F6F4F3;
            color: #5695DC;
        }
    "#).unwrap();

    let option_row_style = style!(r#"
        display: flex;
        margin-top: 16px;
        height: 32px;
    "#).unwrap();

    let option_name_style = style!(r#"
        flex: 1;
        line-height: 32px;
        user-select: none;
    "#).unwrap();

    let option_selector_style = style!(r#"
        display: flex;

        div {
            border: 1px solid white;
            border-right: 0px;
            user-select: none;
            width: 32px;
            height: 32px;
            line-height: 32px;
            text-align: center;
            cursor: pointer;
            transition: color 0.2s ease-out, background-color 0.2s ease-out;
        }

        div:first-child {
            border-radius: 3px 0 0 3px;
        }

        div:hover {
            background-color: white;
            color: black;
        }

        .selected {
            background-color: white;
            color: black;
        }

        input {
            outline: none;
            border-radius: 0 3px 3px 0;
            border: 1px solid white;
            width: 70px;
            padding: 0 8px;
            text-align: center;
        }
    "#).unwrap();

    let number_of_images_options = [1, 5, 10];

    let number_of_images_components = number_of_images_options
        .into_iter()
        .map(|v| html!(<div 
            class={if v == params.number_of_images && !params.number_of_images_custom { "selected" } else { "" }}
            onclick={
                let params = params.clone();
                move |_| { params.dispatch(ImageGenerationParamAction::SelectNumberOfImagesOption(v)) }
            }>{v.to_string()}</div>))
        .collect::<Vec<_>>();

    let on_number_of_images_custom_change = {
        let params = params.clone();
        
        move |e: Event| {
            let target: Option<EventTarget> = e.target();
            let input = target.and_then(|t| t.dyn_into::<HtmlInputElement>().ok());
            if let Some(input) = input {
                params.dispatch(ImageGenerationParamAction::SetCustomNumberOfImages(input.value().parse().unwrap()));
            }
        }
    };

    let task_type_switch = if enable_chat {
        html!("task type switch")
    } else {
        html!()
    };

    html!(
        <div class={page_style}>
            { task_type_switch }
            <span class={description_style}>{"Provide a text description of an image, and this app will generate it for you!"}</span>
            <input class={input_style} onchange={on_prompt_change} value={params.prompt.clone()} placeholder={"prompt, for example: cute cat"}/>
            <button class={generate_image_button_style} onclick={run_inference}>{"generate image"}</button>
            <div class={option_row_style}>
                <div class={option_name_style}>{"number of images"}</div>
                <div class={option_selector_style}>
                    { number_of_images_components }
                    <input type="number" onchange={on_number_of_images_custom_change} value={if params.number_of_images_custom { Some(params.number_of_images.to_string()) } else { None }} />
                </div>
            </div>
        </div>
    )
}