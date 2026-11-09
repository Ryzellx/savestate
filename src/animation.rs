//! Small animation helpers shared by the GUI.
//!
//! Everything here is driven through egui's `animate_value_with_time`,
//! which only requests a repaint while a value is still moving, so an
//! idle app does not burn CPU repainting. When reduced motion is on, every
//! helper returns the target value at once: the change still happens, it
//! just happens instantly.

/// Cubic ease-out: fast start, gentle landing. Used for entrances.
pub fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

/// Cubic ease-in-out: gentle at both ends. Used for sidebar width and
/// other two-way motion.
pub fn ease_in_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// Animate a value toward `target` over `seconds`, returning the current
/// animated value. Pass a stable `id` per animated property. With reduced
/// motion the target is returned immediately.
pub fn animate_to(
    ctx: &egui::Context,
    id: impl Into<egui::Id>,
    target: f32,
    seconds: f32,
    reduced_motion: bool,
) -> f32 {
    if reduced_motion {
        target
    } else {
        ctx.animate_value_with_time(id.into(), target, seconds)
    }
}

/// Opacity for a page that just appeared: 0 at `started`, 1 after
/// `seconds`. Returns 1.0 immediately with reduced motion.
pub fn page_fade(elapsed_secs: f32, seconds: f32, reduced_motion: bool) -> f32 {
    if reduced_motion {
        1.0
    } else {
        ease_out_cubic(elapsed_secs / seconds)
    }
}
