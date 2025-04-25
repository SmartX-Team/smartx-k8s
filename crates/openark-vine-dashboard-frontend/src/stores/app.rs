use std::{collections::BTreeMap, rc::Rc};

use chrono::{DateTime, TimeDelta, Utc};
use openark_vine_dashboard_api::{
    app::App,
    client::ClientExt as _,
    item::ItemMetadata,
    page::{PageRef, PageSpec as OwnedPageSpec},
    table::TableSession,
};
use openark_vine_session_api::{client::ClientExt as _, exec::ExecArgs};
use serde_json::{Map, Value};
use url::Url;
use yew::prelude::*;
use yewdux::prelude::*;

use crate::widgets::{Dialog, DialogAction};

use super::client::{ApiStore, Client, Request, Response};

#[derive(Clone, Debug)]
pub struct Cached<T> {
    pub created_at: DateTime<Utc>,
    pub data: Option<T>,
}

impl<T> PartialEq for Cached<T> {
    fn eq(&self, other: &Self) -> bool {
        // Do not compare data
        self.created_at == other.created_at && self.data.is_some() == other.data.is_some()
    }
}

impl<T> Cached<T> {
    fn try_hit<R, F>(&self, now: DateTime<Utc>, f: F) -> Option<Response<R>>
    where
        F: FnOnce(&T) -> Response<R>,
    {
        match &self.data {
            Some(data) => {
                if now
                    <= self
                        .created_at
                        .checked_add_signed(AppStore::TTL)
                        .unwrap_or(DateTime::<Utc>::MAX_UTC)
                {
                    Some(f(data))
                } else {
                    None
                }
            }
            None => Some(Response::NotFound),
        }
    }
}

#[derive(Clone, Debug)]
pub enum PageSpec {
    Table(Rc<TableSession>),
}

impl From<OwnedPageSpec> for PageSpec {
    fn from(value: OwnedPageSpec) -> Self {
        match value {
            OwnedPageSpec::Table(spec) => Self::Table(Rc::new(spec)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TableRows {
    pub page_ref: PageRef,
    pub data: Value,
}

#[derive(Clone, Debug, Default, PartialEq, Store)]
pub struct AppStore {
    app: Option<Cached<Rc<App>>>,
    pages: BTreeMap<PageRef, Cached<PageSpec>>,
    table_rows: Option<Cached<Rc<TableRows>>>,
}

impl AppStore {
    #[cfg(feature = "ttl")]
    const TTL: TimeDelta = TimeDelta::minutes(5);

    #[cfg(any(not(feature = "ttl")))]
    const TTL: TimeDelta = TimeDelta::MAX;

    /// Reset the states and invoke reloading.
    pub fn clear(&mut self) {
        self.table_rows = None;
    }
}

impl ApiStore<AppStore> {
    pub fn update_item(
        self,
        metadata: &ItemMetadata,
        item_name: Option<String>,
        value: Map<String, Value>,
        callback: Callback<bool>,
    ) {
        let base_url = metadata.base_url.clone();
        let dispatch = self.dispatch.clone();
        let value = Value::Object(value);
        self.call(Request {
            fetch: move |client: Client| async move {
                client
                    .update_item(base_url.clone(), item_name.as_deref(), &value)
                    .await
            },
            ready: true,
            update: move |_: &mut AppStore, result| match result {
                Some(()) => {
                    // Clear cache
                    dispatch.reduce_mut(|store| store.clear());
                    callback.emit(true)
                }
                None => callback.emit(false),
            },
        })
    }

    pub fn delete_table_row(
        self,
        dialog: UseReducerDispatcher<Dialog>,
        base_url: Url,
        item_name: String,
    ) {
        // Disable UI
        dialog.dispatch(DialogAction::Disable);

        self.call(Request {
            fetch: move |client: Client| async move {
                client.delete_table_row(base_url.clone(), item_name).await
            },
            ready: true,
            update: move |_: &mut AppStore, _| dialog.dispatch(DialogAction::Close),
        })
    }

    pub fn vine_session_command_list(self, base_url: Url) {
        self.call(Request {
            fetch: move |client: Client| async move {
                client.vine_session_command_list(base_url.clone()).await
            },
            ready: true,
            update: move |_: &mut AppStore, _| (),
        })
    }

    pub fn vine_session_exec(self, base_url: Url, args: ExecArgs) {
        self.call(Request {
            fetch: move |client: Client| async move {
                client.vine_session_exec(base_url.clone(), &args).await
            },
            ready: true,
            update: move |_: &mut AppStore, _| (),
        })
    }
}

#[hook]
pub fn use_app(api: ApiStore<AppStore>) -> Response<Rc<App>> {
    // Do nothing if the cache is alive.
    let now = Utc::now();
    let mut result = None;
    if let Some(cache) = api.store.app.as_ref() {
        if let Some(cached) = cache.try_hit(now, |data| Response::Ok(data.clone())) {
            result.replace(cached);
        }
    }

    let ready = result.is_none();
    use_effect_with(ready, move |ready| {
        api.call(Request {
            fetch: move |client: Client| async move { client.get_app().await },
            ready: *ready,
            update: move |store: &mut AppStore, app: Option<_>| {
                store.app = Some(Cached {
                    created_at: now,
                    data: app.map(Rc::new),
                })
            },
        })
    });
    result.unwrap_or(Response::Fetching)
}

#[hook]
pub fn use_page(api: ApiStore<AppStore>, page: PageRef) -> Response<PageSpec> {
    // Do nothing if the cache is alive.
    let now = Utc::now();
    let mut result = None;
    if let Some(cache) = api.store.pages.get(&page) {
        if let Some(cached) = cache.try_hit(now, |data| Response::Ok(data.clone())) {
            result.replace(cached);
        }
    }

    let ready = result.is_none();
    use_effect_with(ready, move |ready| {
        api.call(Request {
            fetch: {
                let page = page.clone();
                move |client: Client| async move { client.get_page(&page).await }
            },
            ready: *ready,
            update: move |store: &mut AppStore, data: Option<OwnedPageSpec>| {
                store.pages.insert(
                    page.clone(),
                    Cached {
                        created_at: now,
                        data: data.map(Into::into),
                    },
                );
                store.table_rows = None; // clear rows cache
            },
        })
    });
    result.unwrap_or(Response::Fetching)
}

#[hook]
pub fn use_table_rows(
    api: ApiStore<AppStore>,
    page_ref: PageRef,
    base_url: Url,
) -> Response<Cached<Rc<TableRows>>> {
    // Do nothing if the cache is alive.
    let now = Utc::now();
    let mut result = None;
    if let Some(cache) = api.store.table_rows.as_ref() {
        if let Some(cached) = cache.try_hit(now, |rows| {
            if rows.page_ref == page_ref {
                Response::Ok(cache.clone())
            } else {
                Response::NotFound
            }
        }) {
            result.replace(cached);
        }
    }

    let ready = result.is_none();
    use_effect_with(ready, move |ready| {
        api.call(Request {
            fetch: move |client: Client| async move {
                client.get_table_rows(base_url.clone()).await
            },
            ready: *ready,
            update: move |store: &mut AppStore, rows: Option<_>| {
                store.table_rows = Some(Cached {
                    created_at: now,
                    data: rows.map(|data| {
                        Rc::new(TableRows {
                            page_ref: page_ref.clone(),
                            data,
                        })
                    }),
                })
            },
        })
    });
    result.unwrap_or(Response::Fetching)
}
