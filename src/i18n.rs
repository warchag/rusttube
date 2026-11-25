use crate::models::AppLanguage;

/// Return either the English or Thai static string based on the active language.
#[inline]
pub fn t(lang: AppLanguage, en: &'static str, th: &'static str) -> &'static str {
    match lang {
        AppLanguage::En => en,
        AppLanguage::Th => th,
    }
}
