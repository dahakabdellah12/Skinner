#![allow(dead_code)]
pub struct I18n {
    pub is_arabic: bool,
}

impl I18n {
    pub fn new(is_arabic: bool) -> Self {
        Self { is_arabic }
    }

    pub fn t<'a>(&self, key: &'a str) -> &'a str {
        if self.is_arabic {
            match key {
                "app_title" => "Skinner",
                "app_tagline" => "استوديو تعديل وتخصيص أشكال Brawlhalla",
                "search_placeholder" => "ابحث عن بطل...",
                "all_weapons" => "الكل",
                "legends_roster" => "قائمة الأبطال",
                "costume_label" => "الزي:",
                "weapon1_label" => "السلاح الأول:",
                "weapon2_label" => "السلاح الثاني:",
                "color_scheme_label" => "لوحة الألوان:",
                "apply_button" => "تطبيق التعديلات",
                "save_preset_button" => "حفظ في القوالب",
                "restore_all_button" => "استعادة الأصلية",
                "launch_game_button" => "تشغيل اللعبة",
                "back_button" => "العودة للأبطال",
                "presets_title" => "القوالب المحفوظة",
                "diagnostics_title" => "التشخيص وحالة النظام",
                "no_presets" => "لا توجد قوالب محفوظة بعد.",
                "game_detected" => "تم التعرف على اللعبة بنجاح",
                "game_not_found" => "لم يتم العثور على مسار Brawlhalla",
                "status_ready" => "جاهز",
                "status_applied" => "تم تطبيق التعديل بنجاح!",
                "status_restored" => "تمت استعادة الملفات الأصلية بنجاح!",
                _ => key,
            }
        } else {
            match key {
                "app_title" => "Skinner",
                "app_tagline" => "Brawlhalla Cosmetic Modding Studio",
                "search_placeholder" => "Search legends...",
                "all_weapons" => "All",
                "legends_roster" => "Legends Roster",
                "costume_label" => "Costume:",
                "weapon1_label" => "Primary Weapon:",
                "weapon2_label" => "Secondary Weapon:",
                "color_scheme_label" => "Color Scheme:",
                "apply_button" => "Apply Modification",
                "save_preset_button" => "Save Preset",
                "restore_all_button" => "Rollback All",
                "launch_game_button" => "Launch Game",
                "back_button" => "< Back to Roster",
                "presets_title" => "Saved Presets",
                "diagnostics_title" => "System Diagnostics",
                "no_presets" => "No saved presets yet.",
                "game_detected" => "Brawlhalla Detected",
                "game_not_found" => "Game Not Found",
                "status_ready" => "Ready",
                "status_applied" => "Modifications applied successfully!",
                "status_restored" => "Game files restored to pristine state!",
                _ => key,
            }
        }
    }
}
