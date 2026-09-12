use crate::config::Config;

pub const DOCUMENT_EXTENSIONS: [&str; 16] = [
    "pdf", "doc", "docx", "odt", "rtf", "xls", "xlsx", "ods", "csv", "ppt", "pptx", "odp", "txt",
    "md", "epub", "mobi",
];

pub const IMAGE_EXTENSIONS: [&str; 11] = [
    "jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "svg", "webp", "heic", "avif",
];

pub const AUDIO_EXTENSIONS: [&str; 8] = ["mp3", "wav", "flac", "aac", "ogg", "m4a", "wma", "opus"];

pub const VIDEO_EXTENSIONS: [&str; 8] = ["mp4", "avi", "mkv", "mov", "wmv", "flv", "webm", "m4v"];

pub const ARCHIVE_EXTENSIONS: [&str; 8] = ["zip", "rar", "7z", "tar", "gz", "bz2", "xz", "iso"];

pub const CODE_EXTENSIONS: [&str; 29] = [
    "js", "mjs", "cjs", "ts", "tsx", "jsx", "py", "java", "c", "h", "cpp", "hpp", "cc", "cs", "go",
    "rs", "rb", "php", "html", "css", "scss", "json", "xml", "yml", "yaml", "sh", "sql", "toml",
    "lock",
];

pub const APP_EXTENSIONS: [&str; 3] = ["dmg", "pkg", "app"];

pub const INSTALLER_EXTENSIONS: [&str; 5] = ["exe", "msi", "deb", "rpm", "appimage"];

pub fn category_for(ext: &str) -> &'static str {
    if DOCUMENT_EXTENSIONS.contains(&ext) {
        "Documents"
    } else if IMAGE_EXTENSIONS.contains(&ext) {
        "Images"
    } else if AUDIO_EXTENSIONS.contains(&ext) {
        "Audio"
    } else if VIDEO_EXTENSIONS.contains(&ext) {
        "Video"
    } else if ARCHIVE_EXTENSIONS.contains(&ext) {
        "Archives"
    } else if CODE_EXTENSIONS.contains(&ext) {
        "Code"
    } else if APP_EXTENSIONS.contains(&ext) {
        "Apps"
    } else if INSTALLER_EXTENSIONS.contains(&ext) {
        "Installers"
    } else {
        "Other"
    }
}

pub fn builtin_destination(ext: &str) -> String {
    match ext {
        "pdf" => "Documents/PDFs".to_string(),
        "doc" | "docx" | "odt" | "rtf" => "Documents/Word Files".to_string(),
        "xls" | "xlsx" | "ods" | "csv" => "Documents/Spreadsheets".to_string(),
        "ppt" | "pptx" | "odp" => "Documents/Presentations".to_string(),
        "txt" | "md" => "Documents/Text Files".to_string(),
        "epub" | "mobi" => "Documents/E-books".to_string(),
        _ => category_for(ext).to_string(),
    }
}

pub fn resolve_destination(config: &Config, ext: &str) -> String {
    if let Some(dest) = config.rules.get(ext) {
        return dest.clone();
    }
    if let Some(dest) = config.destination_map.get(ext) {
        return dest.clone();
    }
    let category = category_for(ext);
    if let Some(dest) = config.category_destinations.get(category) {
        return dest.clone();
    }
    if config.flatten_to_categories {
        return category.to_string();
    }
    builtin_destination(ext)
}

pub fn extension_of(name: &str) -> String {
    match name.rfind('.') {
        Some(index) if index + 1 < name.len() => name[index + 1..].to_ascii_lowercase(),
        _ => String::new(),
    }
}
