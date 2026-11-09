pub fn bounds(key: &str) -> (f64, f64, f64) {
    match key {
        "desktops/@number" => (1., 1024., 1.),
        "libinput/device/rotationAngle" => (0., 359., 1.),
        "libinput/device/scrollButton" => (0., 767., 1.),
        "desktops/popupTime"
        | "menu/ignoreButtonReleasePeriod"
        | "snapping/overlay/delay/inner"
        | "snapping/overlay/delay/outer" => (0., 60000., 1.),
        "snapping/range/inner" | "snapping/range/outer" => (0., 10000., 1.),

        "theme/cornerRadius" => (0., 24., 1.),
        "core/gap" => (0., 80., 1.),
        "focus/raiseOnFocusDelay" => (0., 3000., 1.),
        "resistance/screenEdgeStrength" | "resistance/windowEdgeStrength" => (-99., 99., 1.),
        "resistance/unMaximizeTreshold" => (0., 600., 10.),
        "magnifier/width" | "magnifier/height" => (-1., 2000., 50.),
        "magnifier/initScale" => (0., 8., 0.1),
        "magnifier/increment" => (0., 1., 0.1),
        "libinput/device/pointerSpeed" => (-1., 1., 0.1),
        "libinput/device/scrollFactor" => (0.1, 5., 0.1),
        "keyboard/repeatDelay" => (0., 999., 1.),
        "XCURSOR_SIZE" => (16., 60., 1.),
        _ => (0., 99., 1.),
    }
}
