/// Icon data for drink sizes
/// Using SVG icons for consistent cross-platform rendering

use anyhow::{Result, Context};
use std::path::Path;

/// Load and render an SVG file to RGBA data
pub fn load_svg_to_rgba(svg_path: &Path, size: u32) -> Result<Vec<u8>> {
    // Read the SVG file
    let svg_data = std::fs::read(svg_path)
        .with_context(|| format!("Failed to read SVG file: {:?}", svg_path))?;

    // Parse the SVG
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(&svg_data, &opt)
        .with_context(|| format!("Failed to parse SVG file: {:?}", svg_path))?;

    // Create a pixmap to render into
    let mut pixmap = tiny_skia::Pixmap::new(size, size)
        .context("Failed to create pixmap")?;

    // Render the SVG
    let scale = size as f32 / tree.size().width().max(tree.size().height());
    let transform = tiny_skia::Transform::from_scale(scale, scale);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Convert RGBA8 to the format expected by tray-icon
    Ok(pixmap.take())
}

/// Get the SVG file path for a drink size
pub fn get_svg_path_for_size(size: &str) -> &'static str {
    match size {
        "Short" => "assets/icons/short.svg",
        "Tall" => "assets/icons/tall.svg",
        "Grande" => "assets/icons/grande.svg",
        "Venti" => "assets/icons/venti.svg",
        _ => "assets/icons/short.svg", // Default fallback
    }
}

/// Load an icon for a drink size
pub fn load_icon_for_size(size: &str, icon_size: u32) -> Result<Vec<u8>> {
    let svg_path = get_svg_path_for_size(size);
    load_svg_to_rgba(Path::new(svg_path), icon_size)
}

/// Get the tray icon (chair)
pub fn load_tray_icon(size: u32) -> Result<Vec<u8>> {
    load_svg_to_rgba(Path::new("assets/icons/chair.svg"), size)
}

/// Create a menu icon label (just the text now, SVGs handled separately)
#[cfg(target_os = "macos")]
pub fn create_menu_icon(size: &str) -> String {
    size.to_string()
}
