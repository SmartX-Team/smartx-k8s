use openark_vine_dashboard_api::page::PageRef;
use yew::prelude::*;

use crate::{
    layouts::Scaffold,
    stores::{
        app::{PageSpec, use_app, use_page},
        client::use_api,
    },
    unwrap_response,
    widgets::{Dialog, TableWidget},
};

#[derive(Clone, Debug, PartialEq, Eq, Properties)]
pub struct DynPageProps {
    pub page_ref: PageRef,
}

#[function_component(DynPage)]
pub fn component(props: &DynPageProps) -> Html {
    let DynPageProps { page_ref } = props;
    let page_ref = page_ref.clone();

    // Define states

    let api = use_api();
    let dialog = use_reducer(Dialog::default);

    let app = use_app(api.clone());
    let page = use_page(api.clone(), page_ref.clone());

    // Validate states

    let app = unwrap_response!(&api, app);
    let page = unwrap_response!(&api, page);

    let body = {
        let dialog = dialog.dispatcher();
        let page_ref = page_ref.clone();
        match page {
            PageSpec::Table(session) => html! {
                <TableWidget { dialog } { page_ref } { session } />
            },
        }
    };

    html! {
        <Scaffold { app } { dialog } { page_ref }>
            <div class="flex-grow overflow-x-auto">
                { body }
            </div>
        </Scaffold>
    }
}
