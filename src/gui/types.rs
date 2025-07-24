#![allow(unexpected_cfgs)]

use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use objc::runtime::Object;
use objc::{class, msg_send, sel, sel_impl};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum FontFamily {
    #[default]
    System, // -apple-system, BlinkMacSystemFont
    Menlo,     // SF Mono, Menlo
    Monaco,    // Monaco
    Helvetica, // Helvetica Neue
}

impl FontFamily {
    pub fn css_value(&self) -> &'static str {
        match self {
            FontFamily::System => {
                "-apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, Helvetica, Arial, sans-serif"
            }
            FontFamily::Menlo => "\"SF Mono\", \"Menlo\", \"Monaco\", monospace",
            FontFamily::Monaco => "\"Monaco\", \"SF Mono\", \"Menlo\", monospace",
            FontFamily::Helvetica => "\"Helvetica Neue\", Helvetica, Arial, sans-serif",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ThemeMode {
    Light,
    Dark,
    #[default]
    System, // Follow system preference
}

impl ThemeMode {
    pub fn css_color_scheme(&self) -> &'static str {
        match self {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
            ThemeMode::System => "light dark",
        }
    }
}

// Simplified style preferences without toolbar-specific state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StylePreferences {
    pub font_family: FontFamily,
    pub font_size: f32,
    pub theme: ThemeMode,
}

impl Default for StylePreferences {
    fn default() -> Self {
        Self {
            font_family: FontFamily::default(),
            font_size: 14.0,
            theme: ThemeMode::default(),
        }
    }
}

impl StylePreferences {
    const PREFERENCES_KEY: &'static str = "StylePreferences";

    /// Load preferences from macOS UserDefaults
    pub fn load_from_user_defaults() -> Self {
        unsafe {
            let user_defaults: *mut Object =
                msg_send![class!(NSUserDefaults), standardUserDefaults];
            let key = CFString::new(Self::PREFERENCES_KEY);
            let key_ptr = key.as_concrete_TypeRef();

            let data: *mut Object = msg_send![user_defaults, dataForKey: key_ptr];

            if !data.is_null() {
                let length: usize = msg_send![data, length];
                let bytes: *const u8 = msg_send![data, bytes];
                let slice = std::slice::from_raw_parts(bytes, length);

                if let Ok(prefs) = serde_json::from_slice::<StylePreferences>(slice) {
                    return prefs;
                }
            }
        }

        // Return default preferences if loading fails
        Self::default()
    }

    /// Save preferences to macOS UserDefaults
    pub fn save_to_user_defaults(&self) {
        if let Ok(json_data) = serde_json::to_vec(self) {
            unsafe {
                let user_defaults: *mut Object =
                    msg_send![class!(NSUserDefaults), standardUserDefaults];
                let key = CFString::new(Self::PREFERENCES_KEY);
                let key_ptr = key.as_concrete_TypeRef();

                let data: *mut Object = msg_send![class!(NSData), dataWithBytes: json_data.as_ptr() length: json_data.len()];
                let _: () = msg_send![user_defaults, setObject: data forKey: key_ptr];
                let _: () = msg_send![user_defaults, synchronize];
            }
        }
    }

    pub fn increase_font_size(&mut self) {
        let new_size = match self.font_size as i32 {
            8..=9 => 10.0,
            10..=11 => 12.0,
            12..=13 => 14.0,
            14..=15 => 16.0,
            16..=17 => 18.0,
            18..=21 => 22.0,
            22..=27 => 28.0,
            28..=35 => 36.0,
            36..=47 => 48.0,
            48..=71 => 72.0,
            _ => self.font_size,
        };
        self.font_size = new_size;
    }

    pub fn decrease_font_size(&mut self) {
        let new_size = match self.font_size as i32 {
            9..=10 => 8.0,
            11..=12 => 10.0,
            13..=14 => 12.0,
            15..=16 => 14.0,
            17..=18 => 16.0,
            19..=22 => 18.0,
            23..=28 => 22.0,
            29..=36 => 28.0,
            37..=48 => 36.0,
            49..=72 => 48.0,
            _ => self.font_size,
        };
        self.font_size = new_size;
    }

    pub fn reset_font_size(&mut self) {
        self.font_size = 14.0; // Reset to default size
    }

    pub fn generate_css(&self) -> String {
        let font_family = self.font_family.css_value();
        let font_size = self.font_size;
        let color_scheme = self.theme.css_color_scheme();

        // Start with theme-specific CSS variables first
        let mut css = format!(":root {{\n    color-scheme: {color_scheme};\n");

        // Add theme-specific variables based on current theme
        match self.theme {
            ThemeMode::Light => {
                css.push_str(
                    r#"    --border-color: #d1d9e0;
    --code-bg-color: rgba(175, 184, 193, 0.2);
    --pre-bg-color: #f6f8fa;
    --muted-text-color: #57606a;
    --table-row-bg: #ffffff;
    --table-row-alt-bg: #f6f8fa;
    --table-header-bg: #f6f8fa;
    --table-row-hover-bg: #f5f8ff;
    --table-row-alt-hover-bg: #eef4ff;
"#,
                );
            }
            ThemeMode::Dark => {
                css.push_str(
                    r#"    --border-color: #30363d;
    --code-bg-color: rgba(110, 118, 129, 0.4);
    --pre-bg-color: #161b22;
    --muted-text-color: #8b949e;
    --table-row-bg: #0d1117;
    --table-row-alt-bg: #161b22;
    --table-header-bg: #21262d;
    --table-row-hover-bg: #1c2128;
    --table-row-alt-hover-bg: #262c36;
"#,
                );
            }
            ThemeMode::System => {
                css.push_str(
                    r#"    --border-color: #d1d9e0;
    --code-bg-color: rgba(175, 184, 193, 0.2);
    --pre-bg-color: #f6f8fa;
    --muted-text-color: #57606a;
    --table-row-bg: #ffffff;
    --table-row-alt-bg: #f6f8fa;
    --table-header-bg: #f6f8fa;
    --table-row-hover-bg: #f5f8ff;
    --table-row-alt-hover-bg: #eef4ff;
"#,
                );
            }
        }

        css.push_str("}\n");

        // Add the main styles that use the variables
        css.push_str(&format!(
            r#"body {{
    font-family: {font_family};
    font-size: {font_size}px;
    font-weight: normal;
    line-height: 1.6;
    padding: 20px;
    margin: 0;
}}
h1, h2, h3, h4, h5, h6 {{
    border-bottom: 1px solid var(--border-color);
    padding-bottom: .3em;
    margin-top: 24px;
    margin-bottom: 16px;
}}
code {{
    font-family: "SF Mono", "Menlo", "Monaco", monospace;
    background-color: var(--code-bg-color);
    padding: .2em .4em;
    margin: 0;
    font-size: 85%;
    border-radius: 6px;
}}
pre {{
    font-family: "SF Mono", "Menlo", "Monaco", monospace;
    background-color: var(--pre-bg-color);
    padding: 16px;
    border-radius: 6px;
    overflow: auto;
}}
pre > code {{
    padding: 0;
    margin: 0;
    font-size: 100%;
    background-color: transparent;
    border: none;
}}
blockquote {{
    border-left: .25em solid var(--border-color);
    padding: 0 1em;
    color: var(--muted-text-color);
}}
table {{
    border-collapse: collapse;
    border-spacing: 0;
    margin: 16px 0;
    width: 100%;
    overflow: visible;
    display: table;
    border: 1px solid var(--border-color);
    border-radius: 6px;
}}
table thead {{
    display: table-header-group;
}}
table tbody {{
    display: table-row-group;
}}
table thead tr {{
    background-color: var(--table-header-bg);
    border-top: none;
}}
table tbody tr {{
    background-color: var(--table-row-bg);
    border-top: 1px solid var(--border-color);
}}
table tr:first-child {{
    border-top: none;
}}
table th,
table td {{
    padding: 8px 12px;
    border-right: 1px solid var(--border-color);
    display: table-cell;
    text-align: left;
    vertical-align: top;
    line-height: 1.5;
}}
table th:last-child,
table td:last-child {{
    border-right: none;
}}
table th {{
    font-weight: 600;
    background-color: var(--table-header-bg);
    border-bottom: 1px solid var(--border-color);
}}
table td {{
    font-weight: normal;
    background-color: var(--table-row-bg);
}}
table tbody tr:hover {{
    background-color: var(--table-row-hover-bg);
}}
/* Mermaid diagram styling */
.mermaid-container {{
    position: relative;
    margin: 16px 0;
}}
.mermaid-buttons {{
    position: absolute;
    top: 8px;
    right: 8px;
    z-index: 10;
    display: flex;
    gap: 4px;
}}
.mermaid-toggle-btn,
.mermaid-copy-btn {{
    background: var(--table-header-bg);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    padding: 4px 8px;
    font-size: 12px;
    cursor: pointer;
    opacity: 0.7;
    transition: opacity 0.2s ease;
}}
.mermaid-toggle-btn:hover,
.mermaid-copy-btn:hover {{
    opacity: 1;
    background: var(--table-row-hover-bg);
}}
.mermaid-raw {{
    font-family: "SF Mono", "Menlo", "Monaco", monospace;
    background-color: var(--pre-bg-color);
    padding: 16px;
    border-radius: 6px;
    border: 1px solid var(--border-color);
    overflow: auto;
    margin: 0;
}}
.mermaid-raw code {{
    background: transparent;
    padding: 0;
    border: none;
    font-size: 14px;
}}
.mermaid {{
    text-align: center;
    padding: 16px;
    background-color: var(--pre-bg-color);
    border-radius: 6px;
    border: 1px solid var(--border-color);
    overflow: auto;
}}
.mermaid svg {{
    max-width: 100%;
    height: auto;
}}
/* Ensure mermaid diagrams are visible in both themes */
.mermaid .node rect,
.mermaid .node circle,
.mermaid .node ellipse,
.mermaid .node polygon {{
    stroke: var(--border-color);
    stroke-width: 1px;
}}
.mermaid .edgePath path {{
    stroke: var(--muted-text-color);
    stroke-width: 1.5px;
}}
.mermaid .edgeLabel {{
    background-color: var(--table-row-bg);
    border: 1px solid var(--border-color);
    border-radius: 3px;
    padding: 2px 4px;
}}
"#
        ));

        // Add dark mode body styling and system theme media query if needed
        match self.theme {
            ThemeMode::Dark => {
                css.push_str(
                    r#"body {
    background-color: #0d1117;
    color: #f0f6fc;
}
/* Ensure code blocks have bright text in dark mode */
pre, pre code, code {
    color: #f0f6fc !important;
}
pre code span {
    opacity: 1 !important;
}
"#,
                );
            }
            ThemeMode::System => {
                css.push_str(
                    r#"
/* Dark theme overrides for system theme */
@media (prefers-color-scheme: dark) {
    :root {
        --border-color: #30363d;
        --code-bg-color: rgba(110, 118, 129, 0.4);
        --pre-bg-color: #161b22;
        --muted-text-color: #8b949e;
        --table-row-bg: #0d1117;
        --table-row-alt-bg: #161b22;
        --table-header-bg: #21262d;
        --table-row-hover-bg: #1c2128;
        --table-row-alt-hover-bg: #262c36;
    }
    body {
        background-color: #0d1117;
        color: #f0f6fc;
    }
    /* Ensure code blocks have bright text in dark mode */
    pre, pre code, code {
        color: #f0f6fc !important;
    }
    pre code span {
        opacity: 1 !important;
    }
}
"#,
                );
            }
            _ => {}
        }

        css
    }
}
