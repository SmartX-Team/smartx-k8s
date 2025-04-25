use actix_web::web;
use openark_vine_oauth::User;

use crate::LabelArgs;

pub(crate) fn build_label_selector(
    labels: web::Data<LabelArgs>,
    label_selector: Option<&str>,
    user: &User,
) -> String {
    let mut appended_label_selector = format!(
        "{k1}==true, {k2} in (, {v2})",
        k1 = &labels.label_bind,
        k2 = &labels.label_bind_user,
        v2 = user.username(),
    );

    match label_selector {
        Some(label_selector) => {
            appended_label_selector.push_str(label_selector);
            appended_label_selector
        }
        None => appended_label_selector,
    }
}
