mod app;
mod layouts;
mod models;
mod pages;
mod router;
mod stores;
mod widgets;

use app::App;

fn main() {
    ::openark_core::init_once();

    #[cfg(feature = "tracing")]
    ::tracing::info!("Welcome to OpenARK VINE Dashboard UI!");

    ::yew::Renderer::<App>::new().render();
}
