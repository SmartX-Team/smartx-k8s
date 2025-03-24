use yew::prelude::*;
use yew_router::prelude::*;

use crate::router::{Route, switch};

#[function_component(App)]
pub fn component() -> Html {
    html! {
        <main>
            <BrowserRouter>
                <Switch<Route> render={ switch } />
            </BrowserRouter>
        </main>
    }
}
