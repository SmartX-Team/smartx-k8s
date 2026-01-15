#[derive(Debug)]
pub struct Korean;

impl super::I18n for Korean {
    #[inline]
    fn alert_file_entry_empty(&self) -> &'static str {
        "비어있음"
    }

    #[inline]
    fn alert_file_not_found(&self) -> &'static str {
        "존재하지 않는 파일입니다."
    }

    #[inline]
    fn alert_invalid_file_path(&self) -> &'static str {
        "문제가 발생했습니다. 파일 경로가 올바르지 않습니다."
    }

    #[inline]
    fn alert_unknown(&self) -> &'static str {
        "문제가 발생했습니다. 잠시 후 다시 시도해 주세요."
    }

    #[inline]
    fn alert_unsupported_file_preview(&self) -> &'static str {
        "미리보기를 지원하지 않는 파일입니다."
    }

    #[inline]
    fn date_modified(&self) -> &'static str {
        "최근 수정"
    }

    #[inline]
    fn file_name(&self) -> &'static str {
        "파일명"
    }

    #[inline]
    fn file_owner(&self) -> &'static str {
        "소유자"
    }

    #[inline]
    fn file_size(&self) -> &'static str {
        "파일 크기"
    }

    fn format_capacity_usage(&self, usage: u64, capacity: u64) -> String {
        let is_dir = false;
        let usage = self.format_size(is_dir, Some(usage));
        let capacity = self.format_size(is_dir, Some(capacity));
        format!("{capacity} 공유 공간 중 {usage} 사용중")
    }

    #[inline]
    fn indicator_as_grid(&self) -> &'static str {
        "격자로 보기"
    }

    #[inline]
    fn indicator_as_list(&self) -> &'static str {
        "목록으로 보기"
    }

    #[inline]
    fn indicator_download(&self) -> &'static str {
        "다운로드"
    }

    #[inline]
    fn indicator_downloads_uploads(&self) -> &'static str {
        "다운로드 / 업로드"
    }

    #[inline]
    fn indicator_drop_to_upload(&self) -> &'static str {
        "여기에 업로드하기"
    }

    #[inline]
    fn indicator_io_status(&self) -> &'static str {
        "입출력 현황"
    }

    #[inline]
    fn indicator_my_files(&self) -> &'static str {
        "내 파일"
    }

    #[inline]
    fn search_button(&self) -> &'static str {
        "검색"
    }

    #[inline]
    fn sidebar_action_new(&self) -> &'static str {
        "신규"
    }

    #[inline]
    fn sidebar_action_new_folder(&self) -> &'static str {
        "새 폴더"
    }

    #[inline]
    fn sidebar_action_upload_file(&self) -> &'static str {
        "파일 업로드"
    }

    #[inline]
    fn sidebar_action_upload_folder(&self) -> &'static str {
        "폴더 업로드"
    }

    #[inline]
    fn status_activated(&self) -> &'static str {
        "구독중"
    }

    #[inline]
    fn status_deactivated(&self) -> &'static str {
        "해지됨"
    }

    #[inline]
    fn status_unknown(&self) -> &'static str {
        "알 수 없음"
    }

    #[inline]
    fn subscription_current_tier(&self) -> &'static str {
        "현재 구독 티어"
    }

    #[inline]
    fn timeago(&self) -> ::timeago::BoxedLanguage {
        ::timeago::Language::clone_boxed(&::timeago::languages::korean::Korean)
    }

    #[inline]
    fn unit_file(&self, _x: u64) -> &'static str {
        "파일"
    }
}
