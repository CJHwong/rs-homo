//! Pixel art Ferris crab loading animation.
//!
//! Provides a CSS box-shadow pixel art walking crab (12×10 grid, scale 4 = 48×40 px)
//! and a full-screen overlay system for smooth page transitions.
//!
//! ## Layers
//!
//! - **Sprite** ([`SPRITE_CSS`], [`SPRITE_HTML`]) — the animated crab itself.
//!   Use these to drop a walking Ferris anywhere in a page.
//! - **Overlay** ([`overlay_css`], [`overlay_html`]) — a fixed full-screen backdrop
//!   that centers the sprite. Includes sprite styles.
//! - **Reveal** ([`reveal_js`], [`CONTENT_WRAP_OPEN`], [`CONTENT_WRAP_CLOSE`]) —
//!   coordinates dismissing the overlay and fading in wrapped content.

// ---------------------------------------------------------------------------
// Sprite layer — the crab itself, no positioning opinions
// ---------------------------------------------------------------------------

/// CSS for the animated Ferris crab sprite.
///
/// Defines the walking entrance (`crab-walk`), vertical bob (`crab-bob`),
/// and two-frame leg animation (`crab-step`). Does **not** include any overlay
/// or positioning — the consumer decides where to place it.
pub const SPRITE_CSS: &str = r#"
.crab-walker { animation: crab-walk 0.35s ease-out forwards; }
.crab-bob { animation: crab-bob 0.3s ease-in-out infinite; }
.crab-sprite {
    width: 1px; height: 1px;
    transform: scale(4); transform-origin: top left;
    animation: crab-step 0.3s steps(1) infinite;
}
@keyframes crab-walk {
    from { transform: translateX(-30px); }
    to   { transform: translateX(0); }
}
@keyframes crab-bob {
    0%, 100% { transform: translateY(0); }
    50%      { transform: translateY(-2px); }
}
@keyframes crab-step {
    0%, 49% {
        box-shadow:
            2px 0 #E04820,3px 0 #E04820,8px 0 #E04820,9px 0 #E04820,
            1px 1px #E04820,2px 1px #E04820,9px 1px #E04820,10px 1px #E04820,
            3px 2px #F46B3F,4px 2px #F46B3F,5px 2px #F46B3F,6px 2px #F46B3F,7px 2px #F46B3F,8px 2px #F46B3F,
            2px 3px #F46B3F,3px 3px #F46B3F,4px 3px #F46B3F,5px 3px #F46B3F,6px 3px #F46B3F,7px 3px #F46B3F,8px 3px #F46B3F,9px 3px #F46B3F,
            2px 4px #F46B3F,3px 4px #F46B3F,4px 4px #FFF,5px 4px #F46B3F,6px 4px #F46B3F,7px 4px #FFF,8px 4px #F46B3F,9px 4px #F46B3F,
            2px 5px #F46B3F,3px 5px #F46B3F,4px 5px #1C1C1C,5px 5px #F46B3F,6px 5px #F46B3F,7px 5px #1C1C1C,8px 5px #F46B3F,9px 5px #F46B3F,
            3px 6px #F46B3F,4px 6px #F46B3F,5px 6px #F46B3F,6px 6px #F46B3F,7px 6px #F46B3F,8px 6px #F46B3F,
            1px 7px #D04422,4px 7px #F46B3F,5px 7px #F46B3F,6px 7px #F46B3F,7px 7px #F46B3F,10px 7px #D04422,
            0 8px #D04422,2px 8px #D04422,5px 8px #F46B3F,6px 8px #F46B3F,9px 8px #D04422,11px 8px #D04422,
            3px 9px #D04422,8px 9px #D04422;
    }
    50%, 100% {
        box-shadow:
            2px 0 #E04820,3px 0 #E04820,8px 0 #E04820,9px 0 #E04820,
            1px 1px #E04820,2px 1px #E04820,9px 1px #E04820,10px 1px #E04820,
            3px 2px #F46B3F,4px 2px #F46B3F,5px 2px #F46B3F,6px 2px #F46B3F,7px 2px #F46B3F,8px 2px #F46B3F,
            2px 3px #F46B3F,3px 3px #F46B3F,4px 3px #F46B3F,5px 3px #F46B3F,6px 3px #F46B3F,7px 3px #F46B3F,8px 3px #F46B3F,9px 3px #F46B3F,
            2px 4px #F46B3F,3px 4px #F46B3F,4px 4px #FFF,5px 4px #F46B3F,6px 4px #F46B3F,7px 4px #FFF,8px 4px #F46B3F,9px 4px #F46B3F,
            2px 5px #F46B3F,3px 5px #F46B3F,4px 5px #1C1C1C,5px 5px #F46B3F,6px 5px #F46B3F,7px 5px #1C1C1C,8px 5px #F46B3F,9px 5px #F46B3F,
            3px 6px #F46B3F,4px 6px #F46B3F,5px 6px #F46B3F,6px 6px #F46B3F,7px 6px #F46B3F,8px 6px #F46B3F,
            2px 7px #D04422,4px 7px #F46B3F,5px 7px #F46B3F,6px 7px #F46B3F,7px 7px #F46B3F,9px 7px #D04422,
            1px 8px #D04422,3px 8px #D04422,5px 8px #F46B3F,6px 8px #F46B3F,8px 8px #D04422,10px 8px #D04422,
            0 9px #D04422,4px 9px #D04422,7px 9px #D04422,11px 9px #D04422;
    }
}
"#;

/// HTML for just the animated crab sprite (no overlay wrapper).
pub const SPRITE_HTML: &str =
    r#"<div class="crab-walker"><div class="crab-bob"><div class="crab-sprite"></div></div></div>"#;

// ---------------------------------------------------------------------------
// Overlay layer — full-screen backdrop that centers the sprite
// ---------------------------------------------------------------------------

/// CSS positioning for the full-screen overlay container.
const OVERLAY_CONTAINER_CSS: &str = r#"
#crab-overlay {
    position: fixed; top: 0; left: 0; right: 0; bottom: 0;
    z-index: 9999;
    display: flex; align-items: center; justify-content: center;
    background: var(--page-bg, #fff);
    transition: opacity 0.2s ease;
}
"#;

/// Returns CSS for the full-screen loading overlay (container + sprite styles).
///
/// Uses `var(--page-bg)` for the backdrop so it matches the active theme.
pub fn overlay_css() -> String {
    format!("{OVERLAY_CONTAINER_CSS}{SPRITE_CSS}")
}

/// Returns HTML for the loading overlay element (wraps the sprite).
pub fn overlay_html() -> String {
    format!(r#"<div id="crab-overlay">{SPRITE_HTML}</div>"#)
}

// ---------------------------------------------------------------------------
// Reveal layer — content wrapper + JS coordination
// ---------------------------------------------------------------------------

/// Opening tag for the content wrapper hidden during the transition.
pub const CONTENT_WRAP_OPEN: &str =
    r#"<div id="content-wrap" style="opacity:0;transition:opacity 0.2s ease">"#;

/// Closing tag for the content wrapper.
pub const CONTENT_WRAP_CLOSE: &str = "</div>";

/// Generates the `<script>` tag that coordinates overlay dismissal.
///
/// `min_display_ms` is the minimum time (in milliseconds) the crab overlay
/// stays visible — ensures the animation is perceived as intentional rather
/// than a glitch. After this minimum elapses (or immediately if the page
/// took longer to settle), the content wrapper fades in and the overlay
/// fades out simultaneously.
pub fn reveal_js(min_display_ms: u32) -> String {
    format!(
        r#"<script>
window.__crabStart = Date.now();
window.revealContent = function() {{
    var elapsed = Date.now() - window.__crabStart;
    var delay = Math.max(0, {min_display_ms} - elapsed);
    setTimeout(function() {{
        var wrap = document.getElementById('content-wrap');
        if (wrap) wrap.style.opacity = '1';
        var overlay = document.getElementById('crab-overlay');
        if (overlay) {{
            overlay.style.opacity = '0';
            setTimeout(function() {{ overlay.remove(); }}, 200);
        }}
    }}, delay);
}};
</script>"#
    )
}
