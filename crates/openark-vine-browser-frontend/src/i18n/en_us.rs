#[derive(Debug)]
pub struct English;

impl super::I18n for English {
    #[inline]
    fn alert_file_entry_empty(&self) -> &'static str {
        "Empty"
    }

    #[inline]
    fn alert_file_not_found(&self) -> &'static str {
        "The file does not exist."
    }

    #[inline]
    fn alert_invalid_file_path(&self) -> &'static str {
        "Something went wrong. The file path is incorrect."
    }

    #[inline]
    fn alert_unknown(&self) -> &'static str {
        "Something went wrong. Please try again later."
    }

    #[inline]
    fn alert_unsupported_file_preview(&self) -> &'static str {
        "This file does not support preview."
    }

    #[inline]
    fn alert_unsupported_file_preview_audio(&self) -> &'static str {
        "Your browser does not support the audio element."
    }

    #[inline]
    fn date_modified(&self) -> &'static str {
        "Date modified"
    }

    #[inline]
    fn file_name(&self) -> &'static str {
        "Name"
    }

    #[inline]
    fn file_owner(&self) -> &'static str {
        "Owner"
    }

    #[inline]
    fn file_size(&self) -> &'static str {
        "File size"
    }

    fn format_capacity_usage(&self, usage: u64, capacity: u64) -> String {
        let is_dir = false;
        let usage = self.format_size(is_dir, Some(usage));
        let capacity = self.format_size(is_dir, Some(capacity));
        format!("{usage} of shared {capacity} used")
    }

    #[inline]
    fn indicator_as_grid(&self) -> &'static str {
        "View as Grid"
    }

    #[inline]
    fn indicator_as_list(&self) -> &'static str {
        "View as List"
    }

    #[inline]
    fn indicator_download(&self) -> &'static str {
        "Download"
    }

    #[inline]
    fn indicator_downloads_uploads(&self) -> &'static str {
        "Downloads / Uploads"
    }

    #[inline]
    fn indicator_drop_to_upload(&self) -> &'static str {
        "Drop to upload"
    }

    #[inline]
    fn indicator_io_status(&self) -> &'static str {
        "I/O Status"
    }

    #[inline]
    fn indicator_my_files(&self) -> &'static str {
        "My files"
    }

    #[inline]
    fn search_button(&self) -> &'static str {
        "Search"
    }

    #[inline]
    fn sidebar_action_new(&self) -> &'static str {
        "New"
    }

    #[inline]
    fn sidebar_action_new_folder(&self) -> &'static str {
        "New folder"
    }

    #[inline]
    fn sidebar_action_upload_file(&self) -> &'static str {
        "File upload"
    }

    #[inline]
    fn sidebar_action_upload_folder(&self) -> &'static str {
        "Folder upload"
    }

    #[inline]
    fn status_activated(&self) -> &'static str {
        "Activated"
    }

    #[inline]
    fn status_deactivated(&self) -> &'static str {
        "Deactivated"
    }

    #[inline]
    fn status_anonymous(&self) -> &'static str {
        "Anonymous"
    }

    #[inline]
    fn status_unknown(&self) -> &'static str {
        "Unknown"
    }

    #[inline]
    fn subscription_current_tier(&self) -> &'static str {
        "Current Tier"
    }

    #[inline]
    fn timeago(&self) -> ::timeago::BoxedLanguage {
        ::timeago::Language::clone_boxed(&::timeago::languages::english::English)
    }

    fn unit_file(&self, x: u64) -> &'static str {
        if x == 1 { "file" } else { "files" }
    }
}
