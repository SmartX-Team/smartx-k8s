use std::rc::Rc;

use openark_vine_browser_api::{
    file::FileEntry,
    file_type::{AppType, FileType},
};
use yew::{Html, Properties, function_component, html};

use crate::{
    i18n::DynI18n,
    net::get_file_content_url,
    widgets::{Error, Warn},
};

#[derive(Clone, Debug, Properties)]
pub(super) struct Props {
    pub(super) file_entry: Rc<FileEntry>,
    pub(super) i18n: DynI18n,
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
    let i18n = &props.i18n;

    html! {
        <div class="flex-1 overflow-y-auto">{{
            let url = match get_file_content_url(file) {
                Ok(url) => url.to_string(),
                Err(error) => return html! {
                    <Error
                        message={ i18n.alert_invalid_file_path() }
                        details={ error.to_string() }
                    />
                },
            };

            match &file.metadata.ty {
                Some(FileType::Audio(ty)) => html! {
                    <audio class="w-full" controls=true>
                        <source
                            src={ url }
                            type={ ty.mime_type().to_string() }
                        />
                        { i18n.alert_unsupported_file_preview_audio() }
                    </audio>
                },
                Some(FileType::Document(ty)) => html! {
                    <object
                        class="w-full h-full"
                        data={ url }
                        type={ ty.mime_type().to_string() }
                        width="100%"
                    />
                },
                Some(FileType::Image(_)) => html! {
                    <picture>
                        <img
                            class="max-w-full max-h-full object-contain"
                            src={ url }
                        />
                    </picture>
                },
                Some(FileType::Video(ty)) => html! {
                    <object
                        class="w-full h-full"
                        data={ url }
                        type={ ty.mime_type().to_string() }
                        width="100%"
                    />
                },
                Some(FileType::App(AppType::OctetStream))
                | Some(FileType::App(AppType::Other(_)))
                | None => html! {
                    <div class="select-none">
                        <Warn message={ i18n.alert_unsupported_file_preview() } />
                    </div>
                },
            }
        }}</div>
    }
}
