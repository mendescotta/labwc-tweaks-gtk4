// Additional scalar options from labwc/docs/labwc-config.5.scd.
// Kept separate from the Qt inventory so importing upstream cannot remove them.
use crate::schema::Field;
pub fn pages() -> Vec<(&'static str, Vec<Field>)> {
    vec![
        (
            "Desktop & display",
            vec![
                (
                    "Automatically enable outputs (disabling requires an output manager)",
                    "core/autoEnableOutputs",
                    "yes",
                    "bool",
                ),
                ("Prompt command hook", "core/promptCommand", "", "text"),
                ("Error command hook", "core/errorCommand", "", "text"),
                ("Adaptive sync", "core/adaptiveSync", "no", "text"),
                ("Allow tearing", "core/allowTearing", "no", "text"),
                (
                    "Enable HDR (requires Vulkan renderer)",
                    "core/hdr",
                    "no",
                    "bool",
                ),
                ("Reuse output mode", "core/reuseOutputMode", "no", "bool"),
                (
                    "Keep XWayland running (restart required)",
                    "core/xwaylandPersistence",
                    "no",
                    "bool",
                ),
                (
                    "Primary selection clipboard (restart required)",
                    "core/primarySelection",
                    "yes",
                    "bool",
                ),
                ("Workspace count", "desktops/@number", "1", "integer"),
                ("Initial workspace", "desktops/initial", "", "text"),
                (
                    "Workspace popup duration (ms)",
                    "desktops/popupTime",
                    "1000",
                    "integer",
                ),
                (
                    "Workspace name prefix",
                    "desktops/prefix",
                    "Workspace",
                    "text",
                ),
            ],
        ),
        (
            "Titlebar & menus",
            vec![
                (
                    "Titlebar button layout",
                    "theme/titlebar/layout",
                    "icon:iconify,max,close",
                    "text",
                ),
                (
                    "Show window title",
                    "theme/titlebar/showTitle",
                    "yes",
                    "bool",
                ),
                (
                    "Keep resize border when decorations are hidden",
                    "theme/keepBorder",
                    "yes",
                    "bool",
                ),
                (
                    "Fallback application icon",
                    "theme/fallbackAppIcon",
                    "labwc",
                    "text",
                ),
                ("Show menu icons", "menu/showIcons", "yes", "bool"),
                (
                    "Ignore initial menu release (ms)",
                    "menu/ignoreButtonReleasePeriod",
                    "250",
                    "integer",
                ),
            ],
        ),
        (
            "Window switcher",
            vec![
                (
                    "Preview selected window",
                    "windowSwitcher/@preview",
                    "yes",
                    "bool",
                ),
                (
                    "Outline selected window",
                    "windowSwitcher/@outlines",
                    "yes",
                    "bool",
                ),
                (
                    "Unshade selected window",
                    "windowSwitcher/@unshade",
                    "yes",
                    "bool",
                ),
                ("Window order", "windowSwitcher/@order", "focus", "text"),
                ("Show switcher", "windowSwitcher/osd/@show", "yes", "bool"),
                (
                    "Switcher style",
                    "windowSwitcher/osd/@style",
                    "classic",
                    "text",
                ),
                (
                    "Show on outputs",
                    "windowSwitcher/osd/@output",
                    "all",
                    "text",
                ),
                (
                    "Thumbnail label format",
                    "windowSwitcher/osd/@thumbnailLabelFormat",
                    "%T",
                    "text",
                ),
            ],
        ),
        (
            "More input & snapping",
            vec![
                (
                    "Keyboard layout scope",
                    "keyboard/@layoutScope",
                    "global",
                    "text",
                ),
                (
                    "Inner edge snap range (px)",
                    "snapping/range/inner",
                    "10",
                    "integer",
                ),
                (
                    "Outer edge snap range (px)",
                    "snapping/range/outer",
                    "10",
                    "integer",
                ),
                (
                    "Snap overlay placement",
                    "snapping/overlay/placement",
                    "below",
                    "text",
                ),
                (
                    "Inner edge overlay delay (ms)",
                    "snapping/overlay/delay/inner",
                    "500",
                    "integer",
                ),
                (
                    "Outer edge overlay delay (ms)",
                    "snapping/overlay/delay/outer",
                    "500",
                    "integer",
                ),
                (
                    "Clickfinger button map",
                    "libinput/device/clickfingerButtonMap",
                    "lrm",
                    "text",
                ),
                (
                    "Scroll button (Linux event code)",
                    "libinput/device/scrollButton",
                    "274",
                    "integer",
                ),
                (
                    "Relative device rotation (degrees)",
                    "libinput/device/rotationAngle",
                    "0",
                    "integer",
                ),
            ],
        ),
    ]
}
