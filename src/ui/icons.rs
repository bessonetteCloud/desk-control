/// Icon data for Starbucks drink sizes
/// Using Unicode characters to represent coffee cups

pub fn get_icon_for_size(size: &str) -> &'static str {
    match size {
        "Short" => "☕", // Small cup
        "Tall" => "🥤",  // Medium cup
        "Grande" => "🍺", // Large cup (using beer mug as approximation)
        "Venti" => "🏺",  // Extra large (using amphora/vase)
        _ => "❓",
    }
}

/// Get a simple text-based icon for menu items
pub fn get_text_icon(size: &str) -> String {
    let emoji = get_icon_for_size(size);
    format!("{} {}", emoji, size)
}

/// Create an NSImage-compatible icon data
/// For now, we'll use emoji rendering, but this could be extended
/// to use actual PNG data for custom cup icons
#[cfg(target_os = "macos")]
pub fn create_menu_icon(size: &str) -> String {
    get_text_icon(size)
}
