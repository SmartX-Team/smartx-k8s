use std::{cell::RefCell, rc::Rc};

use chrono::{DateTime, Utc};
use convert_case::{Case, Casing};
use itertools::Itertools;
use openark_vine_dashboard_api::{
    item::{Item, ItemMetadata},
    page::PageRef,
    table::{
        TableExtraService, TableExtraServiceKind, TablePrinterColumn, TablePrinterColumnKind,
        TablePrinterColumnPrefix, TablePrinterColumnSecondary, TablePrinterColumnTags,
        TableSession,
    },
};
use serde_json::Value;
use url::Url;
use yew::prelude::*;

use crate::{
    router::Route,
    stores::{
        app::{AppStore, Cached, use_table_rows},
        client::{ApiStore, use_api},
    },
    unwrap_response,
    widgets::{DialogAction, dialog::DialogState},
};

use super::Dialog;

trait ValueExt {
    fn to_display_string(&self) -> String;
}

impl ValueExt for Value {
    fn to_display_string(&self) -> String {
        match self {
            Self::String(value) => value.clone(),
            Self::Array(values) => values.iter().map(ValueExt::to_display_string).join(" | "),
            Self::Object(values) => values
                .iter()
                .map(|(key, value)| format!("{key}: {}", value.to_display_string()))
                .join(" | "),
            _ => self.to_string(),
        }
    }
}

#[derive(Debug, Default)]
struct Selections {
    all: Option<bool>,
    items: RefCell<Vec<bool>>,
    timestamp: Option<DateTime<Utc>>,
}

enum SelectionsAction {
    /// Clear the items and reset the timestamp.
    Reset { timestamp: DateTime<Utc> },
    /// Toggle in item.
    Toggle(usize, bool),
    /// Toggle all items.
    ToggleAll(bool),
}

impl Reducible for Selections {
    type Action = SelectionsAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            SelectionsAction::Reset { timestamp } => Rc::new(Self {
                all: None,
                items: Default::default(),
                timestamp: Some(timestamp),
            }),
            SelectionsAction::Toggle(index, checked) => {
                {
                    let mut items = self.items.borrow_mut();
                    // Fill with defaults
                    if index <= items.len() {
                        items.resize(index + 1, self.get_default());
                    }
                    items[index] = checked;
                }
                self
            }
            SelectionsAction::ToggleAll(checked) => Rc::new(Self {
                all: Some(checked),
                items: Default::default(),
                timestamp: self.timestamp,
            }),
        }
    }
}

struct Context<'a> {
    api: ApiStore<AppStore>,
    dialog: &'a UseReducerDispatcher<Dialog>,
    selections: UseReducerHandle<Selections>,
    page_ref: &'a PageRef,
    session: &'a TableSession,
    tab_index: &'a UseStateHandle<Option<usize>>,
}

impl Selections {
    fn get(&self, index: usize) -> bool {
        self.items
            .borrow()
            .get(index)
            .copied()
            .unwrap_or_else(|| self.get_default())
    }

    fn get_default(&self) -> bool {
        self.all.unwrap_or_default()
    }
}

fn build_header_printer_columns(session: &TableSession) -> Vec<Html> {
    let builtin_services = session.spec.services.update.enabled
        && !session.spec.schema.fields.is_empty()
        || session.spec.services.delete.enabled;
    let extra_services = || {
        session
            .spec
            .extra_services
            .as_ref()
            .is_some_and(|services| !services.is_empty())
    };

    let extra_services = if builtin_services || extra_services() {
        Some(html! {
            <th>{ "Services" }</th>
        })
    } else {
        None
    };

    session
        .spec
        .printer_columns
        .as_ref()
        .map(|columns| {
            columns
                .iter()
                .map(|TablePrinterColumn { name, .. }| {
                    html! {
                        <th>{ name }</th>
                    }
                })
                .chain(extra_services)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn build_header(context: &Context) -> Html {
    let selections = context.selections.clone();
    let checked = context.selections.all.unwrap_or_default();
    let onchange = move |_: Event| selections.dispatch(SelectionsAction::ToggleAll(!checked));

    html! {
        <thead>
            <tr>
                <th>
                    <label>
                        <input type="checkbox" class="checkbox" { checked } { onchange } />
                    </label>
                </th>
                { build_header_printer_columns(context.session) }
                <th />
            </tr>
        </thead>
    }
}

fn build_row_column_value(
    name: Option<&str>,
    kind: TablePrinterColumnKind,
    json_path: &str,
    value: &Value,
) -> Option<Html> {
    let value = value.pointer(json_path)?;

    Some(match kind {
        TablePrinterColumnKind::ElapsedTime => match value {
            Value::String(value) => html! { value.clone() },
            value => html! { value.to_string() },
        },
        TablePrinterColumnKind::ImageUrl => match value {
            Value::String(src) => html! {
                <div class="avatar">
                    <div class="mask mask-squircle h-12 w-12">
                        <img
                            src={ src.clone() }
                            alt={ name.map(ToString::to_string).unwrap_or_default() }
                        />
                    </div>
                </div>
            },
            _ => html! {},
        },
        TablePrinterColumnKind::Level => {
            let kind = match value.as_str()?.to_case(Case::Pascal).as_str() {
                "Trace" => "status-primary",
                "Debug" | "Unknown" => "status-neutral",
                "Completed" | "Ready" => "status-success",
                "Creating" | "Info" => "status-info",
                "Pending" | "Warn" | "Warning" => "status-warning",
                "Error" => "status-error",
                _ => return None,
            };
            html! {
                <div class="inline-grid *:[grid-area:1/1]">
                    <div class={ format!("status {kind} animate-ping") }></div>
                    <div class={ format!("status {kind}") }></div>
                </div>
            }
        }
        TablePrinterColumnKind::String => html! { value.to_display_string() },
    })
}

fn build_row_column_prefixes(
    prefixes: &[TablePrinterColumnPrefix],
    value: &Value,
) -> impl Iterator<Item = Html> {
    prefixes.iter().filter_map(
        |TablePrinterColumnPrefix {
             name,
             kind,
             json_path,
         }| build_row_column_value(name.as_deref(), *kind, json_path, value),
    )
}

fn build_row_column_tag(tag: String) -> Html {
    html! {
        <span class="badge badge-ghost badge-sm">{ tag }</span>
    }
}

fn build_row_column_tag_value(tag: &Value) -> Option<Html> {
    match tag {
        Value::Null => None,
        Value::String(tag) => Some(build_row_column_tag(tag.clone())),
        tag => Some(build_row_column_tag(tag.to_display_string())),
    }
}

fn build_row_column_tag_key_value(key: &str, tag: &Value) -> Option<Html> {
    match tag {
        Value::Null => None,
        Value::String(value) => Some(build_row_column_tag(format!("{key}: {value}"))),
        value => Some(build_row_column_tag(format!("{key}: {value}"))),
    }
}

fn build_row_column_tags(column_tags: &TablePrinterColumnTags, value: &Value) -> Html {
    let TablePrinterColumnTags { json_path } = column_tags;

    let tags = match value.pointer(json_path) {
        Some(Value::Array(tags)) => tags
            .iter()
            .filter_map(|tag| build_row_column_tag_value(tag))
            .collect(),
        Some(Value::Object(tags)) => tags
            .iter()
            .filter_map(|(key, tag)| build_row_column_tag_key_value(key, tag))
            .collect(),
        Some(Value::String(tag)) => vec![build_row_column_tag(tag.clone())],
        Some(tag) => vec![build_row_column_tag(tag.to_string())],
        None => return Default::default(),
    };

    html! {
        <div class="join">
            { tags }
        </div>
    }
}

fn build_row_column(printer_column: &TablePrinterColumn, value: &Value) -> Html {
    let TablePrinterColumn {
        name,
        kind,
        json_path,
        description,
        prefixes,
        secondary,
        tags,
    } = printer_column;

    // Draw main value
    let body = build_row_column_value(Some(name), *kind, json_path, value).unwrap_or_default();

    // Append secondary
    let body = match secondary {
        Some(TablePrinterColumnSecondary { json_path }) => html! {
            <div>
                <div class="font-bold">{ body }</div>
                <div class="text-sm opacity-50">{
                    value
                        .pointer(json_path)
                        .and_then(|value| value.as_str())
                        .unwrap_or_default()
                }</div>
            </div>
        },
        None => body,
    };

    // Append tags
    let body = match tags {
        Some(tags) => html! {
            <div class="flex flex-col gap-1">
                { body }
                { build_row_column_tags(tags, value) }
            </div>
        },
        None => body,
    };

    // Append prefixes
    let body = match prefixes {
        Some(prefixes) => html! {
            <div class="flex items-center gap-3">
                { for build_row_column_prefixes(prefixes, value) }
                { body }
            </div>
        },
        None => body,
    };

    html! {
        <td>{ body }</td>
    }
}

fn build_row_column_extra_service(service: &TableExtraService, value: &Value) -> Option<Html> {
    if !service.visible || !service.single {
        return None;
    }

    Some(match service.kind {
        TableExtraServiceKind::Navigate | TableExtraServiceKind::VNC => html! {
            <button
                class={
                    if service.side_effect {
                        "btn btn-warning"
                    } else {
                        "btn btn-ghost"
                    }
                }
            >
                <a href={ value.pointer(service.json_path.as_deref()?)?.as_str()?.to_string() } >
                    { service.name.clone() }
                </a>
            </button>
        },
    })
}

fn build_row_column_extra_services(services: &[TableExtraService], value: &Value) -> Vec<Html> {
    services
        .iter()
        .filter_map(|service| build_row_column_extra_service(service, value))
        .collect()
}

fn build_row_column_services(context: &Context, value: &Value) -> Html {
    let session = context.session;
    let page_ref = context.page_ref;

    let builtin_service_update = if session.spec.services.update.enabled
        && !session.spec.schema.fields.is_empty()
    {
        value
            .get("uid")
            .and_then(|value| value.as_str().map(ToString::to_string))
            .and_then(|item_name| {
                let item = Item {
                    metadata: ItemMetadata {
                        base_url: session.spec.base_url.clone(),
                        template: session.spec.schema.clone(),
                    },
                    spec: Some(value.as_object()?.clone()),
                };
                let navigator = context.api.navigator.clone();
                let page_ref = page_ref.clone();
                let onclick = move |_| {
                    let route = Route::page_item_update(page_ref.clone(), Some(item_name.clone()));
                    let state = item.clone();
                    navigator.push_with_state(&route, state)
                };
                Some(html! {
                    <button class="btn btn-warning" { onclick }>
                        { "Update" }
                    </button>
                })
            })
    } else {
        None
    };

    let builtin_service_delete = if session.spec.services.delete.enabled {
        value
            .get("name")
            .and_then(|value| value.as_str().map(ToString::to_string))
            .map(|item_name| {
                let ondelete = {
                    let api = context.api.clone();
                    let dialog = context.dialog.clone();
                    let base_url = session.spec.base_url.clone();
                    let item_name = item_name.clone();
                    Callback::from(move |_| {
                        api.clone().delete_table_row(
                            dialog.clone(),
                            base_url.clone(),
                            item_name.clone(),
                        )
                    })
                };
                let onclick = {
                    let dialog = context.dialog.clone();
                    move |_| {
                        dialog.dispatch(DialogAction::Request(DialogState::DeleteSingle {
                            name: item_name.clone(),
                            ondelete: ondelete.clone(),
                        }))
                    }
                };
                html! {
                    <button class="btn btn-error" { onclick }>
                        { "Delete" }
                    </button>
                }
            })
    } else {
        None
    };

    let extra_services = session
        .spec
        .extra_services
        .as_ref()
        .filter(|&services| !services.is_empty())
        .map(|services| build_row_column_extra_services(services, value))
        .unwrap_or_default();

    html! {
        <td>
            <div class="join">{
                for builtin_service_update
                    .into_iter()
                    .chain(builtin_service_delete)
                    .chain(extra_services)
            }</div>
        </td>
    }
}

fn build_row(context: &Context, index: usize, value: &Value) -> Html {
    let selections = context.selections.clone();
    let checked = selections.get(index);
    let onchange = move |_: Event| selections.dispatch(SelectionsAction::Toggle(index, !checked));

    html! {
        <tr>
            <th>
                <label>
                    <input type="checkbox" class="checkbox" { checked } { onchange } />
                </label>
            </th>
            {
                context
                    .session
                    .spec
                    .printer_columns
                    .as_ref()
                    .map(|columns| {
                        columns
                            .iter()
                            .map(|column| build_row_column(column, value))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            }
            {
                build_row_column_services(context, value)
            }
        </tr>
    }
}

fn build_body(context: &Context, rows: &Value) -> Html {
    html! {
        <tbody>{
            rows.as_array()
                .map(|rows| {
                    rows.iter().enumerate()
                        .map(|(index, value)| build_row(
                            context,
                            index,
                            value,
                        ))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        }</tbody>
    }
}

fn build_extra_service_tab(
    context: &Context,
    service: &TableExtraService,
    index: usize,
) -> Option<Html> {
    if !service.visible || !service.multiple {
        return None;
    }

    match service.kind {
        TableExtraServiceKind::VNC => {
            let class = if **context.tab_index == Some(index) {
                "tab tab-active"
            } else {
                "tab"
            };
            let tab_index = context.tab_index.clone();
            let onclick = move |_| tab_index.set(Some(index));
            Some(html! {
                <a role="tab" { class } { onclick }>
                    { service.name.clone() }
                </a>
            })
        }
        TableExtraServiceKind::Navigate => None,
    }
}

fn build_extra_service_tabs(context: &Context) -> Vec<Html> {
    context
        .session
        .spec
        .extra_services
        .as_ref()
        .map(|services| {
            services
                .iter()
                .enumerate()
                .filter_map(|(index, service)| build_extra_service_tab(context, service, index))
                .collect()
        })
        .unwrap_or_default()
}

fn build_extra_service_tab_content_vnc_item(
    context: &Context,
    service: &TableExtraService,
    value: &Value,
) -> Option<Html> {
    let url: Url = value
        .pointer(service.json_path.as_deref()?)?
        .as_str()?
        .parse()
        .ok()?;

    let base_url = &context.session.spec.base_url;
    let host = url.host_str()?;
    let port = url.port().unwrap_or(443);
    let src = format!(
        "{base_url}/vnc/?autoconnect=true&host={host}&port={port}&reconnect=true&resize=scale&shared=true&view_only=true&quality=5"
    );

    Some(html! {
        <div class="mockup-window bg-base-100 border border-base-300">
            <iframe class="aspect-video w-full" { src } />
        </div>
    })
}

fn build_extra_service_tab_content_vnc(
    context: &Context,
    service: &TableExtraService,
    rows: &Value,
) -> Html {
    let items = rows
        .as_array()
        .map(|rows| {
            #[inline]
            fn get_str<'a>(row: &'a Value, key: &str) -> Option<&'a str> {
                row.pointer(key).and_then(|value| value.as_str())
            }

            rows.iter()
                .sorted_by_key(|&row| {
                    (
                        get_str(row, "/metadata/labels/dash.ulagbulag.io~1alias"),
                        get_str(row, "/metadata/name"),
                    )
                })
                .filter_map(|value| {
                    build_extra_service_tab_content_vnc_item(context, service, value)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if items.is_empty() {
        return html! {};
    }

    let button_cmdline = html! {
        <div class="flex join w-full">
            <div class="w-full">
                <label class="input join-item w-full">
                    <input type="text" placeholder="Type here" required=true />
                </label>
            </div>
            <button class="btn btn-neutral join-item">{ "Enter" }</button>
        </div>
    };

    html! {
        <div class="flex flex-col p-4">
            { button_cmdline }
            <div class="divider" />
            <div class="grid grid-cols-4 gap-4">
                { for items }
            </div>
        </div>
    }
}

fn build_extra_service_tab_content(context: &Context, rows: &Value) -> Html {
    // Get a selected tab service
    let service = match context
        .session
        .spec
        .extra_services
        .as_ref()
        .and_then(|services| services.get(context.tab_index.unwrap_or_default()))
    {
        Some(service) => service,
        None => return html! {},
    };

    // Draw content
    match service.kind {
        TableExtraServiceKind::VNC => build_extra_service_tab_content_vnc(context, service, rows),
        TableExtraServiceKind::Navigate => html! {},
    }
}

fn build_create_button(context: &Context, page_ref: &PageRef) -> Html {
    let navigator = context.api.navigator.clone();
    let page_ref = page_ref.clone();
    let item = Item {
        metadata: ItemMetadata {
            base_url: context.session.spec.base_url.clone(),
            template: context.session.spec.schema.clone(),
        },
        spec: None,
    };
    let onclick = move |_| {
        let route = Route::page_item_update(page_ref.clone(), None);
        let state = item.clone();
        navigator.push_with_state(&route, state)
    };

    html! {
        <div class="absolute bottom-8 right-8">
            <button class="btn btn-ghost text-6xl w-12 h-12" { onclick }>
                { "🆕" }
            </button>
        </div>
    }
}

#[derive(Clone, Debug, Properties)]
pub struct TableWidgetProps {
    pub dialog: UseReducerDispatcher<Dialog>,
    pub page_ref: PageRef,
    pub session: Rc<TableSession>,
}

impl PartialEq for TableWidgetProps {
    fn eq(&self, other: &Self) -> bool {
        self.dialog == other.dialog
            && self.page_ref == other.page_ref
            && Rc::ptr_eq(&self.session, &other.session)
    }
}

#[function_component(TableWidget)]
pub fn component(props: &TableWidgetProps) -> Html {
    let TableWidgetProps {
        dialog,
        page_ref,
        session,
    } = props;

    // Define states

    let api = use_api();
    let rows = use_table_rows(api.clone(), page_ref.clone(), session.spec.base_url.clone());
    let selections = use_reducer(Selections::default);
    let tab_index = use_state_eq(|| None);

    // Validate states

    let Cached {
        created_at: timestamp,
        data: rows,
    } = unwrap_response!(&api, rows);

    // Release all selections if the rows are updated
    if Some(timestamp) != selections.timestamp {
        selections.dispatch(SelectionsAction::Reset { timestamp });
    }

    // Do nothing if the rows are not ready.
    let rows = match rows.as_ref().map(|data| data.data.clone()) {
        Some(rows) => rows,
        None => return Default::default(),
    };

    let context = Context {
        api,
        dialog,
        selections,
        page_ref,
        session: &*session,
        tab_index: &tab_index,
    };

    let floating_button = if tab_index.is_none()
        && session.spec.services.create.enabled
        && !session.spec.schema.fields.is_empty()
    {
        Some(build_create_button(&context, page_ref))
    } else {
        None
    };

    let content = if tab_index.is_none() {
        html! {
            <table class="table">
                { build_header(&context) }
                { build_body(&context, &rows) }
            </table>
        }
    } else {
        build_extra_service_tab_content(&context, &rows)
    };

    let tabs = build_extra_service_tabs(&context);
    let body = if tabs.is_empty() {
        content
    } else {
        let class = match *tab_index {
            Some(_) => "tab",
            None => "tab tab-active",
        };
        let onclick = move |_| tab_index.set(None);
        html! {
            <>
                <div role="tablist" class="flex tabs tabs-border">
                    <a role="tab" { class } { onclick } >{ "List" }</a>
                    { tabs }
                </div>
                <div>
                    { content }
                </div>
            </>
        }
    };

    html! {
        <div>
            { body }
            { for floating_button }
        </div>
    }
}
