use yew::{Html, Properties, function_component, html, html::ChildrenRenderer, virtual_dom::VNode};

#[derive(Clone, Debug, PartialEq, Properties)]
pub struct Props {
    pub message: ChildrenRenderer<VNode>,

    #[prop_or_default]
    pub details: ChildrenRenderer<VNode>,
}

#[function_component(Error)]
pub fn render(props: &Props) -> Html {
    let Props { message, details } = props.clone();

    html! {
        <div class="alert grid-cols-1 grid-rows-2 gap-y-2 shadow-sm border-l-4 bg-gray-50 border-error rounded-none ml-8 mr-8 py-3 truncate">
            <div class="flex space-x-2">
                <svg xmlns="http://www.w3.org/2000/svg" class="stroke-error shrink-0 h-5 w-5" fill="currentColor" viewBox="0 0 24 24">
                    // heroicons:exclamation-triangle:solid
                    <path fill-rule="evenodd" d="M9.401 3.003c1.155-2 4.043-2 5.197 0l7.355 12.748c1.154 2-.29 4.5-2.599 4.5H4.645c-2.309 0-3.752-2.5-2.598-4.5L9.4 3.003ZM12 8.25a.75.75 0 0 1 .75.75v3.75a.75.75 0 0 1-1.5 0V9a.75.75 0 0 1 .75-.75Zm0 8.25a.75.75 0 1 0 0-1.5.75.75 0 0 0 0 1.5Z" clip-rule="evenodd" />
                </svg>
                <p class="text-sm font-medium text-red-400 whitespace-normal">{ message }</p>
            </div>
            <p class="text-sm font-medium text-gray-400 whitespace-normal">{ details }</p>
        </div>
    }
}

#[derive(Clone, Debug, PartialEq, Properties)]
pub struct NotFoundProps {}

#[function_component(NotFound)]
pub fn render_not_found(props: &NotFoundProps) -> Html {
    let NotFoundProps {} = props;

    html! {
        <Error
            message={ "존재하지 않는 파일입니다." }
        />
    }
}
