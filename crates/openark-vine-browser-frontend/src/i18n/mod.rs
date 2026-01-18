mod en_us;
mod ko_kr;

use std::{fmt, rc::Rc};

use chrono::{DateTime, Utc};
use openark_vine_browser_api::user::UserRef;

macro_rules! impl_i18n {
    { $vis:vis trait $ident:ident {
        $(
            $(#[$meta:meta])*
            fn $fn:ident(&$self:ident $(, $arg_field:ident : $arg_ty:ty )* ) -> $ret:ty;
        )*

        ;;

        $(
            $(#[$default_meta:meta])*
            fn $default_fn:ident(&$default_self:ident $(, $default_arg_field:ident : $default_arg_ty:ty )* )
            -> $default_ret:ty
            {
                $($default_stmt:stmt);*
            }
        )*
    } } => {
        /// A common translation trait.
        ///
        $vis trait $ident: fmt::Debug {
            $(
                $(#[$meta])*
                fn $fn(&$self $(, $arg_field : $arg_ty )* ) -> $ret;
            )*

            $(
                $(#[$default_meta])*
                fn $default_fn(&$default_self $(, $default_arg_field : $default_arg_ty )* )
                -> $default_ret
                {
                    $($default_stmt)*
                }
            )*
        }

        impl DynI18n {
            $(
                $(#[$meta])*
                #[inline]
                $vis fn $fn(&$self $(, $arg_field : $arg_ty )* ) -> $ret {
                    $self.0.$fn($($arg_field),*)
                }
            )*

            $(
                $(#[$default_meta])*
                #[inline]
                $vis fn $default_fn(&$default_self $(, $default_arg_field : $default_arg_ty )* )
                -> $default_ret {
                    $default_self.0.$default_fn($($default_arg_field),*)
                }
            )*
        }
    };
}

impl_i18n! {
    pub trait I18n {
        /// Returns the "Empty file entry" alert message.
        fn alert_file_entry_empty(&self) -> &'static str;

        /// Returns the "File not found" alert message.
        fn alert_file_not_found(&self) -> &'static str;

        /// Returns the "Invalid file path" alert message.
        fn alert_invalid_file_path(&self) -> &'static str;

        /// Returns the unknown alert message.
        fn alert_unknown(&self) -> &'static str;

        /// Returns the "Unsupported file preview" alert message.
        fn alert_unsupported_file_preview(&self) -> &'static str;

        /// Returns the "Unsupported audio preview" alert message.
        fn alert_unsupported_file_preview_audio(&self) -> &'static str;

        /// Returns the "Date modified".
        fn date_modified(&self) -> &'static str;

        /// Returns the "File name".
        fn file_name(&self) -> &'static str;

        /// Returns the "File owner".
        fn file_owner(&self) -> &'static str;

        /// Returns the "File size".
        fn file_size(&self) -> &'static str;

        /// Converts the given capacity usage into a [`String`].
        fn format_capacity_usage(&self, usage: u64, capacity: u64) -> String;

        /// Returns the "View as Grid" indicator.
        fn indicator_as_grid(&self) -> &'static str;

        /// Returns the "View as List" indicator.
        fn indicator_as_list(&self) -> &'static str;

        /// Returns the "Download" indicator.
        fn indicator_download(&self) -> &'static str;

        /// Returns the "Downloads / Uploads" indicator.
        fn indicator_downloads_uploads(&self) -> &'static str;

        /// Returns the "Drop to upload" indicator.
        fn indicator_drop_to_upload(&self) -> &'static str;

        /// Returns the "I/O Status" indicator.
        fn indicator_io_status(&self) -> &'static str;

        /// Returns the "My files" indicator.
        fn indicator_my_files(&self) -> &'static str;

        /// Returns the "Search" button.
        fn search_button(&self) -> &'static str;

        /// Returns the "New" sidecar action.
        fn sidebar_action_new(&self) -> &'static str;

        /// Returns the "New folder" sidecar action.
        fn sidebar_action_new_folder(&self) -> &'static str;

        /// Returns the "File upload" sidecar action.
        fn sidebar_action_upload_file(&self) -> &'static str;

        /// Returns the "Folder upload" sidecar action.
        fn sidebar_action_upload_folder(&self) -> &'static str;

        /// Returns the "Activated" status.
        fn status_activated(&self) -> &'static str;

        /// Returns the "Deactivated" status.
        fn status_deactivated(&self) -> &'static str;

        /// Returns the "Anonymous" status.
        fn status_anonymous(&self) -> &'static str;

        /// Returns the "Unknown" status.
        fn status_unknown(&self) -> &'static str;

        /// Returns the "Current Subscription Tier".
        fn subscription_current_tier(&self) -> &'static str;

        /// Returns the current language's [`::timeago::BoxedLanguage`] instance.
        #[allow(dead_code)]
        fn timeago(&self) -> ::timeago::BoxedLanguage;

        /// Returns the "File" unit.
        #[allow(dead_code)]
        fn unit_file(&self, x: u64) -> &'static str;

        ;; // auto-derived

        /// Converts the given time interval into a human-readable [`String`].
        fn format_date(&self, from: Option<DateTime<Utc>>, to: DateTime<Utc>) -> String {
            let from = match from {
                Some(value) => value,
                None => return "-".into(),
            };

            if from <= to {
                // before
                let formatter = ::timeago::Formatter::with_language(self.timeago());
                formatter.convert_chrono(from, to)
            } else {
                // after
                "-".into()
            }
        }

        /// Converts the given user name into an initial.
        fn format_initial(&self, user: &UserRef) -> String {
            user.metadata.initial.clone().unwrap_or_else(|| {
                let name = user.name.as_str();
                name[..name.len().min(2)].into()
            })
        }

        /// Converts the given data size into a human-readable [`String`].
        fn format_size(&self, is_dir: bool, size: Option<u64>) -> String {
            let size = match size {
                Some(value) => value,
                None => return "-".into(),
            };

            let mut base = size.highest_one().unwrap_or(0) / 10;
            let unit = match base {
                0 => "",
                1 => "Ki",
                2 => "Mi",
                3 => "Gi",
                4 => "Ti",
                5 => "Pi",
                6 => "Ei",
                7 => "Zi",
                8 => "Yi",
                9 => "Ri",
                10.. => {
                    base = 10;
                    "Qi"
                }
            };
            let value = (size as f64) / ((1u64 << (base * 10)) as f64);
            let suffix = if is_dir {
                format_args!(" {}", self.unit_file(size))
            } else {
                format_args!("B")
            };
            format!(
                "{value} {unit}{suffix}",
                value = format!("{value:.02}")
                    .trim_end_matches('0')
                    .trim_end_matches('.'),
            )
        }

    }
}

/// A dynamic [`I18n`] instance.
///
#[derive(Clone)]
pub struct DynI18n(Rc<dyn I18n>);

impl DynI18n {
    #[inline]
    fn new(lang: impl I18n + 'static) -> Self {
        DynI18n(Rc::new(lang))
    }

    /// Detects the current user's preferred language.
    ///
    pub fn detect_language() -> Self {
        // Try getting a system language
        let language = ::web_sys::window().and_then(|w| w.navigator().language());

        match language.as_deref() {
            Some("ko-KR") => Self::new(self::ko_kr::Korean),
            // Fallback
            Some("en-US") | _ => Self::new(self::en_us::English),
        }
    }
}

impl fmt::Debug for DynI18n {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (*self.0).fmt(f)
    }
}

impl PartialEq for DynI18n {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
