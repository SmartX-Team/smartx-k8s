use yew::{Html, Properties, function_component, html, html::ChildrenRenderer, virtual_dom::VNode};

#[derive(Clone, Debug, PartialEq, Properties)]
pub struct Props {
    pub message: ChildrenRenderer<VNode>,
}

#[function_component(Warn)]
pub fn render(props: &Props) -> Html {
    let Props { message } = props.clone();

    html! {
        <div class="alert shadow-sm border-l-4 bg-gray-50 border-warning rounded-none m-4 py-3 truncate">
            <svg xmlns="http://www.w3.org/2000/svg" class="stroke-error shrink-0 h-5 w-5" fill="currentColor" viewBox="0 0 24 24">
                // heroicons:exclamation-circle:solid
                <path fill-rule="evenodd" d="M2.25 12c0-5.385 4.365-9.75 9.75-9.75s9.75 4.365 9.75 9.75-4.365 9.75-9.75 9.75S2.25 17.385 2.25 12ZM12 8.25a.75.75 0 0 1 .75.75v3.75a.75.75 0 0 1-1.5 0V9a.75.75 0 0 1 .75-.75Zm0 8.25a.75.75 0 1 0 0-1.5.75.75 0 0 0 0 1.5Z" clip-rule="evenodd" />
            </svg>
            <p class="text-sm text-orange-400 font-medium whitespace-normal">{ message }</p>
        </div>
    }
}

#[derive(Clone, Debug, PartialEq, Properties)]
pub struct EmptyProps {}

#[function_component(Empty)]
pub fn render_empty(props: &EmptyProps) -> Html {
    let EmptyProps {} = props;

    html! {
        <Warn
            message={ "비어있음" }
        />
    }
}
