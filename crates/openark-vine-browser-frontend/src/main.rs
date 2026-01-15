#![feature(int_lowest_highest_one)]

mod app;
mod i18n;
mod net;
mod pages;
mod router;
mod widgets;

use app::App;

fn main() {
    ::openark_core::init_once();

    #[cfg(feature = "tracing")]
    ::tracing::info!("Welcome to OpenARK VINE Browser UI!");

    ::yew::Renderer::<App>::new().render();
}
