use std::rc::Rc;

use openark_vine_browser_api::{file::FileEntry, file_type::FileType};
use yew::{Html, Properties, function_component, html};

use crate::{
    net::get_file_content_url,
    widgets::{Error, Warn},
};

#[derive(Clone, Debug, Properties)]
pub(super) struct Props {
    pub(super) file_entry: Rc<FileEntry>,
}

impl PartialEq for Props {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.file_entry, &other.file_entry)
    }
}

#[function_component(Preview)]
pub(super) fn render(props: &Props) -> Html {
    let file = &props.file_entry.r;

    html! {
        <div class="flex-1">{{
            let url = match get_file_content_url(file) {
                Ok(url) => url.to_string(),
                Err(error) => return html! {
                    <Error
                        message={ "문제가 발생했습니다. 파일 경로가 올바르지 않습니다." }
                        details={ error.to_string() }
                    />
                },
            };

            match file.metadata.ty {
                Some(FileType::Image(_)) => html! {
                    <img
                        class="object-cover"
                        src={ url }
                    />
                },
                _ => html! {
                    <Warn
                        message={ "미리보기를 지원하지 않는 파일입니다." }
                    />
                },
            }
        }}</div>
    }
}
