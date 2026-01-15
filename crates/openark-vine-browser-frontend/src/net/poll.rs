use std::{mem, rc::Rc, time::Duration};

use anyhow::Result;
use chrono::{DateTime, Utc};
use yew::{Html, UseStateHandle, html, platform::spawn_local};

use crate::{
    i18n::DynI18n,
    net::Client,
    widgets::{Error, FileNotFound},
};

#[derive(Clone, Debug)]
pub struct Cached<T> {
    pub(super) expires_at: DateTime<Utc>,
    pub(super) value: T,
}

impl<T> Cached<T> {
    #[inline]
    pub(super) fn new(value: T) -> Self {
        let now = Utc::now();
        let expires_at = now + Duration::from_mins(5);

        Self { expires_at, value }
    }

    fn is_valid(&self) -> bool {
        // Invalidate the cache if expired
        let now = Utc::now();
        now < self.expires_at
    }
}

impl<T> PartialEq for Cached<Rc<T>> {
    fn eq(&self, other: &Self) -> bool {
        // Invalidate the cache if expired
        self.is_valid() && Rc::ptr_eq(&self.value, &other.value)
    }
}

impl<T> PartialEq for Cached<Option<Rc<T>>> {
    fn eq(&self, other: &Self) -> bool {
        match (self.value.as_ref(), other.value.as_ref()) {
            (None, None) => {
                // Invalidate the cache if expired
                self.is_valid()
            }
            (None, Some(_)) | (Some(_), None) => false,
            (Some(l0), Some(r0)) => {
                // Invalidate the cache if expired
                self.is_valid() && Rc::ptr_eq(l0, r0)
            }
        }
    }
}

#[derive(Debug)]
pub enum HttpPoll<T> {
    Pending,
    Fetching,
    Ready(Cached<T>),
    Failed(Cached<Rc<String>>),
}

#[derive(Debug)]
pub enum HttpStateRaw<T> {
    Pending,
    Ready(T),
    NotFound,
    Failed,
}

impl<T> Default for HttpPoll<T> {
    #[inline]
    fn default() -> Self {
        Self::Pending
    }
}

impl<T> PartialEq for HttpPoll<T>
where
    Cached<T>: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Ready(l0), Self::Ready(r0)) => l0 == r0,
            (Self::Failed(l0), Self::Failed(r0)) => l0 == r0,
            _ => mem::discriminant(self) == mem::discriminant(other),
        }
    }
}

#[derive(Debug)]
pub struct HttpContext<K, V> {
    key: Option<K>,
    poll: HttpPoll<V>,
}

impl<K, V> Default for HttpContext<K, V> {
    fn default() -> Self {
        Self {
            key: None,
            poll: Default::default(),
        }
    }
}

impl<K, V> PartialEq for HttpContext<K, V>
where
    K: PartialEq,
    Cached<V>: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.poll == other.poll
    }
}

#[doc(hidden)]
pub trait UseHttpHandleRenderRaw<K, T> {
    #[doc(hidden)]
    fn _fetch_and_render<F, Fut, V, VT, R>(
        &self,
        i18n: &DynI18n,
        key: &K,
        fetch: F,
        validate: V,
        render: R,
    ) -> Html
    where
        Cached<T>: PartialEq,
        K: Clone + PartialEq + 'static,
        F: FnOnce(Client) -> Fut,
        Fut: Future<Output = Result<T>> + 'static,
        V: FnOnce(T) -> Option<VT>,
        VT: Clone + 'static,
        R: FnOnce(HttpStateRaw<VT>) -> Html,
        T: Clone + 'static;

    #[doc(hidden)]
    fn _force_fetch_and_render<F, Fut, VT, R>(&self, key: &K, fetch: F, render: R) -> Html
    where
        Cached<T>: PartialEq,
        K: Clone + 'static,
        F: FnOnce(Client) -> Fut,
        Fut: Future<Output = Result<T>> + 'static,
        R: FnOnce(HttpStateRaw<VT>) -> Html,
        T: Clone + 'static;

    #[doc(hidden)]
    fn _render_fetching<VT, R>(&self, render: R) -> Html
    where
        R: FnOnce(HttpStateRaw<VT>) -> Html;

    #[doc(hidden)]
    fn _render_not_found<VT, R>(&self, i18n: &DynI18n, render: R) -> Html
    where
        R: FnOnce(HttpStateRaw<VT>) -> Html;

    #[doc(hidden)]
    fn _render_error<VT, R>(&self, i18n: &DynI18n, error: Rc<String>, render: R) -> Html
    where
        R: FnOnce(HttpStateRaw<VT>) -> Html;

    #[doc(hidden)]
    fn _poll(&self) -> &HttpPoll<T>;
}

impl<K, T> UseHttpHandleRenderRaw<K, T> for UseStateHandle<HttpContext<K, T>> {
    fn _fetch_and_render<F, Fut, V, VT, R>(
        &self,
        i18n: &DynI18n,
        key: &K,
        fetch: F,
        validate: V,
        render: R,
    ) -> Html
    where
        Cached<T>: PartialEq,
        K: Clone + PartialEq + 'static,
        F: FnOnce(Client) -> Fut,
        Fut: Future<Output = Result<T>> + 'static,
        V: FnOnce(T) -> Option<VT>,
        VT: Clone + 'static,
        R: FnOnce(HttpStateRaw<VT>) -> Html,
        T: Clone + 'static,
    {
        // Invalidate the cache if changed
        if self.key.as_ref() != Some(key) {
            return self._force_fetch_and_render(key, fetch, render);
        }

        match self._poll() {
            HttpPoll::Pending => self._force_fetch_and_render(key, fetch, render),
            HttpPoll::Fetching => self._render_fetching(render),
            HttpPoll::Ready(cached) => {
                // Try hitting the cache
                let now = Utc::now();
                if now < cached.expires_at {
                    match validate(cached.value.clone()) {
                        Some(value) => render(HttpStateRaw::Ready(value)),
                        None => self._render_not_found(i18n, render),
                    }
                } else {
                    // Invalidate the cache if expired
                    self._force_fetch_and_render(key, fetch, render)
                }
            }
            HttpPoll::Failed(cached) => {
                // Try hitting the cache
                let now = Utc::now();
                if now < cached.expires_at {
                    self._render_error(i18n, cached.value.clone(), render)
                } else {
                    // Invalidate the cache if expired
                    self._force_fetch_and_render(key, fetch, render)
                }
            }
        }
    }

    fn _force_fetch_and_render<F, Fut, VT, R>(&self, key: &K, fetch: F, render: R) -> Html
    where
        Cached<T>: PartialEq,
        K: Clone + 'static,
        F: FnOnce(Client) -> Fut,
        Fut: Future<Output = Result<T>> + 'static,
        R: FnOnce(HttpStateRaw<VT>) -> Html,
        T: Clone + 'static,
    {
        // fetch
        let client = Client::new();
        let future = Box::pin(fetch(client));

        // spawn
        {
            let key = key.clone();
            let this = self.clone();
            spawn_local(async move {
                match future.await {
                    Ok(value) => this.set(HttpContext {
                        key: Some(key),
                        poll: HttpPoll::Ready(Cached::new(value)),
                    }),
                    Err(error) => {
                        #[cfg(feature = "tracing")]
                        ::tracing::error!("{error}");
                        let error = Rc::new(error.to_string());
                        this.set(HttpContext {
                            key: Some(key),
                            poll: HttpPoll::Failed(Cached::new(error.clone())),
                        })
                    }
                }
            })
        }

        // register
        self.set(HttpContext {
            key: Some(key.clone()),
            poll: HttpPoll::Fetching,
        });

        // render
        self._render_fetching(render)
    }

    fn _render_fetching<VT, R>(&self, render: R) -> Html
    where
        R: FnOnce(HttpStateRaw<VT>) -> Html,
    {
        html! {
            <div class="relative w-full h-full select-none overflow-hidden">
                // Preview skeletons
                { render(HttpStateRaw::Pending) }

                // Loading splash
                <div class="absolute inset-0 z-10 flex flex-col items-center justify-center backdrop-blur-[2px] transition-all">
                    <div class="flex flex-col items-center gap-3">
                        <span class="loading loading-spinner loading-lg text-primary"></span>
                        <span class="text-sm font-bold tracking-widest text-primary animate-pulse">{ "LOADING" }</span>
                    </div>
                </div>
            </div>
        }
    }

    fn _render_not_found<VT, R>(&self, i18n: &DynI18n, render: R) -> Html
    where
        R: FnOnce(HttpStateRaw<VT>) -> Html,
    {
        html! { <>
            { render(HttpStateRaw::NotFound) }
            <div class="px-5">
                <FileNotFound i18n={ i18n.clone() } />
            </div>
        </> }
    }

    fn _render_error<VT, R>(&self, i18n: &DynI18n, error: Rc<String>, render: R) -> Html
    where
        R: FnOnce(HttpStateRaw<VT>) -> Html,
    {
        html! { <>
            { render(HttpStateRaw::Failed) }
            <div class="px-5">
                <Error
                    message={ i18n.alert_unknown() }
                    details={ error }
                />
            </div>
        </> }
    }

    #[inline]
    fn _poll(&self) -> &HttpPoll<T> {
        &self.poll
    }
}

pub type HttpState<T> = HttpStateRaw<Rc<T>>;

pub type HttpStateRef<'a, T> = HttpStateRaw<&'a Rc<T>>;

// pub type UseHttpHandle<K, V> = UseStateHandle<K, HttpContext<Rc<V>>>;

// pub trait UseHttpHandleRender<K, V>: UseHttpHandleRenderRaw<K, Rc<V>> {
//     #[inline]
//     fn fetch_and_render<F, Fut, R>(&self, key: &K, fetch: F, render: R) -> Html
//     where
//         K: Clone + PartialEq,
//         F: FnOnce(Client) -> Fut + 'static,
//         Fut: Future<Output = Result<V>> + 'static,
//         R: FnOnce(HttpState<V>) -> Html,
//         V: 'static,
//     {
//         let fetch = |client| async move { fetch(client).await.map(Rc::new) };
//         let validate = move |value| Some(value);
//         self._fetch_and_render(key, fetch, validate, render)
//     }
// }

// impl<K, V, U> UseHttpHandleRender<K, V> for U where U: UseHttpHandleRenderRaw<K, Rc<V>> {}

pub type UseHttpHandleOption<K, V> = UseStateHandle<HttpContext<K, Option<Rc<V>>>>;

pub trait UseHttpHandleOptionRender<K, V>: UseHttpHandleRenderRaw<K, Option<Rc<V>>> {
    #[inline]
    fn ok(&self) -> Option<&Rc<V>> {
        match self._poll() {
            HttpPoll::Ready(cached) => cached.value.as_ref(),
            _ => None,
        }
    }

    #[inline]
    fn try_get_state(&self) -> HttpStateRef<'_, V> {
        match self._poll() {
            HttpPoll::Pending | HttpPoll::Fetching => HttpStateRef::Pending,
            HttpPoll::Ready(cached) => match cached.value.as_ref() {
                Some(value) => HttpStateRef::Ready(value),
                None => HttpStateRef::NotFound,
            },
            HttpPoll::Failed(_) => HttpStateRef::Failed,
        }
    }

    #[inline]
    fn try_fetch_and_render<F, Fut, R>(&self, i18n: &DynI18n, key: &K, fetch: F, render: R) -> Html
    where
        K: Clone + PartialEq + 'static,
        F: FnOnce(Client) -> Fut + 'static,
        Fut: Future<Output = Result<Option<V>>> + 'static,
        R: FnOnce(HttpState<V>) -> Html,
        V: 'static,
    {
        let fetch = |client| async move { fetch(client).await.map(|option| option.map(Rc::new)) };
        let validate = move |value| value;
        self._fetch_and_render(i18n, key, fetch, validate, render)
    }
}

impl<K, V, U> UseHttpHandleOptionRender<K, V> for U where U: UseHttpHandleRenderRaw<K, Option<Rc<V>>>
{}
