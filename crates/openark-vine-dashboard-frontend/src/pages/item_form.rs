use convert_case::{Case, Casing};
use itertools::Itertools;
use openark_vine_dashboard_api::{
    item::{Item, ItemField, ItemFieldKind, ItemMetadata},
    page::PageRef,
};
use serde_json::Value;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::{
    layouts::Scaffold,
    models::json_value::{self, JsonValue, JsonValueAction},
    stores::{
        app::{AppStore, use_app},
        client::{ApiStore, use_api},
    },
    unwrap_response,
    widgets::Dialog,
};

fn build_features() -> Html {
    #[derive(Copy, Clone)]
    struct Feature {
        name: &'static str,
        enabled: bool,
    }

    let features = &[
        Feature {
            name: "Real-time collaboration tools",
            enabled: true,
        },
        Feature {
            name: "Seamless cloud integration",
            enabled: true,
        },
        Feature {
            name: "~3 Business day Support (Online)",
            enabled: true,
        },
        Feature {
            name: "Batch processing capabilities",
            enabled: false,
        },
        Feature {
            name: "Connected Data Lake",
            enabled: false,
        },
        Feature {
            name: "Dedicated Resource Allocation",
            enabled: false,
        },
        Feature {
            name: "24/7 Support (Online, Offline)",
            enabled: false,
        },
    ];

    let mut features_enabled = vec![];
    let mut features_disabled = vec![];

    for &Feature { name, enabled } in features {
        if enabled {
            features_enabled.push(html! {
                <li>
                    <svg xmlns="http://www.w3.org/2000/svg" class="size-4 me-2 inline-block text-success" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                    </svg>
                    <span>{ name }</span>
                </li>
            });
        } else {
            features_disabled.push(html! {
                <li class="opacity-50">
                    <svg xmlns="http://www.w3.org/2000/svg" class="size-4 me-2 inline-block text-base-content/50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                    </svg>
                    <span class="line-through">{ name }</span>
                </li>
            });
        }
    }

    html! {
        <ul class="mt-6 flex justify-evenly text-xs">
            <ul class="flex flex-col gap-2">
                <h3 class="text-lg">{ "What's included" }</h3>
                { for features_enabled }
            </ul>
            <ul class="flex flex-col gap-2">
                <h3 class="text-lg">{ "What's excluded" }</h3>
                { for features_disabled }
            </ul>
        </ul>
    }
}

struct Context<'a> {
    api: ApiStore<AppStore>,
    enabled: UseStateHandle<bool>,
    metadata: &'a ItemMetadata,
    item_name: Option<&'a str>,
    value: UseReducerHandle<JsonValue>,
}

fn build_field(context: &Context, field: &ItemField) -> Option<Html> {
    let ItemField {
        name,
        kind,
        optional,
        title,
        description,
        default,
        placeholder,
        max_length,
        min_length,
        max_value,
        min_value,
    } = field.clone();

    let body_optional = if optional {
        Some(html! { <p class="fieldset-label">{ "Optional" }</p> })
    } else {
        None
    };

    let default_value = json_value::to_form_string(&default);
    let placeholder = placeholder.or_else(|| default_value.clone());
    let required = !optional;
    let value = context
        .value
        .borrow()
        .get(&name)
        .and_then(json_value::to_form_string)
        .or_else(|| default_value.clone());

    // Fill with the default value
    if let Some(value) = default_value.as_ref() {
        if context.value.borrow().get(&name).is_none() {
            context.value.dispatch(JsonValueAction::Update {
                name: name.clone(),
                value: Value::String(value.clone()),
            });
        }
    }

    let mut max = None;
    let mut maxlength = None;
    let mut min = None;
    let mut minlength = None;
    let r#type;

    match kind {
        ItemFieldKind::Integer => {
            max = json_value::to_form_i64(&max_value).map(|v| v.to_string());
            min = json_value::to_form_i64(&min_value).map(|v| v.to_string());
            r#type = Some("number");
        }
        ItemFieldKind::String => {
            maxlength = max_length.map(|v| v.to_string());
            {
                let min_length = if optional && min_length == 0 {
                    1
                } else {
                    min_length
                };
                minlength = Some(min_length.to_string());
            }
            r#type = Some("text");
        }
    };

    // NOTE: Ordered
    let mut validator_hints = vec![];
    if let Some(value) = &min {
        validator_hints.push(format!("Must be more than {value}"));
    }
    if let Some(value) = &max {
        validator_hints.push(format!("Must be less than {value}"));
    }
    if let Some(length) = &minlength {
        validator_hints.push(format!("Must be more than {length} characters"));
    }
    if let Some(length) = &maxlength {
        validator_hints.push(format!("Must be less than {length} characters"));
    }

    let body_validator_hints = Itertools::intersperse(
        validator_hints.into_iter().map(|message| {
            html! { message }
        }),
        html! { <br/> },
    );

    let onchange = {
        let dispatcher = context.value.dispatcher();
        let name = name.clone();
        move |event: Event| {
            if let Some(input) = event.target_dyn_into::<HtmlInputElement>() {
                dispatcher.dispatch(JsonValueAction::Update {
                    name: name.clone(),
                    value: Value::String(input.value()),
                });
            }
        }
    };

    let disabled = !*context.enabled;
    let body = html! {
        <div>
            <label class="input validator w-full">
                <input type={ r#type } { required } { disabled } { onchange }
                    { placeholder } { value }
                    { max } { min }
                    { maxlength } { minlength }
                />
            </label>
            <p class="validator-hint hidden">
                { for body_validator_hints }
            </p>
        </div>
    };

    let title = title.unwrap_or_else(|| name.to_case(Case::Title));
    Some(html! {
        <li>
            <fieldset class="fieldset">
                <legend class="fieldset-legend">{ title }</legend>
                { body }
                { for body_optional }
            </fieldset>
        </li>
    })
}

fn build_form(context: Context) -> Html {
    let fields = context
        .metadata
        .template
        .fields
        .iter()
        .filter_map(|field| build_field(&context, field));

    let features = build_features();

    let onsubmit = {
        let api = context.api.clone();
        let enabled = context.enabled.setter();
        let metadata = context.metadata.clone();
        let item_name = context.item_name.map(ToString::to_string);
        let value = context.value.clone();
        let callback = {
            let navigator = context.api.navigator.clone();
            let enabled = context.enabled.setter();
            Callback::from(move |succeeded| {
                if succeeded {
                    navigator.back()
                } else {
                    enabled.set(true)
                }
            })
        };
        move |_| {
            if !value.is_valid(&api.dispatch_client, &metadata) {
                return;
            }
            enabled.set(false);
            api.clone().update_item(
                &metadata,
                item_name.clone(),
                value.borrow().clone(),
                callback.clone(),
            )
        }
    };

    let btn_status = if *context.enabled { "" } else { "btn-disabled" };

    html! {
        <div class="card-body">
            <span class="badge badge-xs badge-warning">{ "Most Popular" }</span>
            <div class="flex justify-between">
                <h2 class="text-3xl font-bold">{ "Create" }</h2>
                <span class="text-xl">{ "$29/mo" }</span>
            </div>
            <ul class="mt-6 flex flex-col gap-2 text-xs">
                { for fields }
            </ul>
            { features }
            <div class="mt-6">
                <button
                    class={ format!("btn btn-primary btn-block {btn_status}") }
                    onclick={ onsubmit }
                >
                    { "Submit" }
                </button>
            </div>
        </div>
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Properties)]
pub struct ItemFormProps {
    pub page_ref: PageRef,

    #[prop_or_default]
    pub item_name: Option<String>,
}

#[function_component(ItemForm)]
pub fn component(props: &ItemFormProps) -> Html {
    let ItemFormProps {
        page_ref,
        item_name,
    } = props;

    // Define states

    let api = use_api();
    let dialog = use_reducer(Dialog::default);
    let location = use_location().unwrap();

    let app = use_app(api.clone());
    let enabled = use_state_eq(|| true);
    let item = location.state::<Item>();
    let value = use_reducer(|| {
        JsonValue::new(
            item.as_ref()
                .and_then(|item| item.spec.clone())
                .unwrap_or_default(),
        )
    });

    // Validate states

    let app = unwrap_response!(&api, app);

    // Do nothing if the item is not set.
    let Item { metadata, spec: _ } = match item.as_ref() {
        Some(item) => &**item,
        None => {
            api.navigator.back();
            return Default::default();
        }
    };

    let body = build_form(Context {
        api,
        enabled,
        metadata,
        item_name: item_name.as_deref(),
        value,
    });

    let page_ref = page_ref.clone();
    html! {
        <Scaffold { app } { dialog } { page_ref }>
            <div class="flex justify-center p-8">
                <div class="card bg-base-100 w-full shadow-sm">
                    { body }
                </div>
            </div>
        </Scaffold>
    }
}
