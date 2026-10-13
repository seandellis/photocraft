//! View › Developer outline overlays.
//!
//! - **UI Outlines** strokes interactive egui widget rects from the last pass (no labels, no egui
//!   debug hover); works in release builds too.
//! - **Widget Hits / Wireframes / Focus** flip egui [`egui::style::DebugOptions`] (debug builds
//!   only). Hits-only is the lightest mode; wireframes paint every interactive widget and egui's
//!   "Widgets in order" stack text under the pointer.
//! - **Canvas Outlines** paints document/tool hit geometry in screen space at the end of
//!   [`crate::canvas::document_area`].
//!
//! All are visible only while View › Extras is on and the matching Developer flag is on
//! ([`crate::view_cmds::ViewOptions::shows`]).

use egui::{Color32, Id, LayerId, Order, Rect, Stroke};

use crate::PhotocraftApp;
use crate::canvas::ViewXform;
use crate::theme::Tokens;
use crate::view_cmds::ViewOptions;

/// Foreground layer so developer outlines are not buried under canvas painting in the same panel.
fn outline_painter(ctx: &egui::Context) -> egui::Painter {
    ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("photocraft_dev_outlines")))
}

/// Canvas rect to outline: the document view updates [`PhotocraftApp::last_canvas_rect`]; otherwise
/// use the central panel bounds (home screen, empty workspace, stale defaults at the origin).
pub(crate) fn canvas_outline_rect(stored: Rect, central: Rect) -> Rect {
    if stored.width() < 1.0 || stored.height() < 1.0 || !central.expand(4.0).intersects(stored) {
        return central;
    }
    // Default `last_canvas_rect` is 800×600 at (0,0); it intersects a real central panel but is
    // not the drawable document area.
    let inset = stored.min.y >= central.min.y - 4.0 && stored.max.x <= central.max.x + 4.0;
    if inset { stored } else { central }
}

/// Stroke interactive widget rects from the previous pass (View › Developer › UI Outlines).
pub fn paint_ui_outlines(ctx: &egui::Context, view: &ViewOptions) {
    if !view.shows(view.developer.ui_outlines) {
        return;
    }
    let t = Tokens::get(ctx);
    let painter = outline_painter(ctx);
    let stroke = Stroke::new(1.0, t.accent);
    // `interactive_rects_last_pass` is from the completed prior pass (one frame behind).
    for rect in ctx.interactive_rects_last_pass() {
        if rect.is_finite() && rect.width() > 0.5 && rect.height() > 0.5 {
            painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Outside);
        }
    }
}

/// Sync egui's debug overlays with View › Developer widget modes (and Extras).
///
/// Only available in debug builds: egui compiles [`egui::style::DebugOptions`] out of release.
pub fn sync_widget_outline_debug(ctx: &egui::Context, view: &ViewOptions) {
    #[cfg(debug_assertions)]
    {
        let extras = view.extras;
        let dev = &view.developer;
        // Wireframes mode used to include pointer hits; keep that so the stack under the cursor
        // is visible without a second menu toggle.
        let pointer_hits = dev.widget_hits || dev.widget_wireframes;
        // Both dark and light styles: Appearance Mode can switch without reopening the menu.
        ctx.all_styles_mut(|s| {
            let d = &mut s.debug;
            d.show_widget_hits = extras && pointer_hits;
            d.show_interactive_widgets = extras && dev.widget_wireframes;
            d.show_focused_widget = extras && dev.widget_focus;
            if !dev.any_widget_debug() {
                // Reset so toggling off (or Extras off) does not leak debug styling.
                let def = egui::style::DebugOptions::default();
                d.show_widget_hits = def.show_widget_hits;
                d.show_interactive_widgets = def.show_interactive_widgets;
                d.show_focused_widget = def.show_focused_widget;
            }
        });
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = (ctx, view);
    }
}

/// Paint View › Developer › Canvas Outlines over the document area (after normal canvas chrome).
pub fn paint_canvas_outlines(app: &PhotocraftApp, ui: &egui::Ui) {
    if !app.ui.view.shows(app.ui.view.developer.canvas_outlines) {
        return;
    }
    let t = Tokens::get(ui.ctx());
    let painter = outline_painter(ui.ctx());
    let stroke = Stroke::new(1.5, t.accent);
    let label_color = t.accent_text;

    let central = ui.max_rect();
    let canvas_rect = canvas_outline_rect(app.last_canvas_rect, central);
    // Full canvas widget and the drawable area inside the rulers.
    outline_rect(&painter, canvas_rect, "canvas", stroke, label_color);
    let content = crate::rulers::content_rect(app, canvas_rect);
    outline_rect(&painter, content, "ruler_content", stroke, label_color);

    let Some(xf) = ViewXform::active(app) else { return };
    let Some(st) = app.session.active() else { return };
    let doc = &st.doc;

    // Same mapping the active view uses for the document image.
    let image = xf.doc_rect(doc.bounds());
    outline_rect(&painter, image, "image", stroke, label_color);

    if let Some(b) = layer_content_bounds(app) {
        outline_rect(&painter, xf.doc_rect(b), "layer_edges", stroke, label_color);
    }
    if let Some(b) = selection_bounds(app) {
        outline_rect(&painter, xf.doc_rect(b), "selection", stroke, label_color);
    }
    if let Some(r) = transform_doc_rect(app) {
        outline_rect(&painter, xf.doc_rect(r), "transform", stroke, label_color);
    }
    if let Some(r) = crop_doc_rect(app) {
        outline_rect(&painter, xf.doc_rect(r), "crop", stroke, label_color);
    }
}

fn outline_rect(painter: &egui::Painter, rect: Rect, label: &str, stroke: Stroke, label_color: Color32) {
    if !rect.is_finite() || rect.width() < 0.5 || rect.height() < 0.5 {
        return;
    }
    painter.rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Outside);
    painter.debug_text(rect.left_top() + egui::vec2(2.0, 2.0), egui::Align2::LEFT_TOP, label_color, label);
}

/// Active layer content bounds at the offset the canvas is showing (no Show › Layer Edges gate).
fn layer_content_bounds(app: &PhotocraftApp) -> Option<photocraft_geom::Rect> {
    let idx = app.session.active_index()?;
    let st = app.session.documents().get(idx)?;
    let id = st.active_layer?;
    let b = st.doc.layer(id)?.surface().map(photocraft_compose::bounds::content_bounds)?;
    if b.is_empty() {
        return None;
    }
    let (dx, dy) = crate::move_ui::shown_offset(app, st.doc.id, st.revision, id).unwrap_or((0, 0));
    Some(b.translate(dx, dy))
}

fn selection_bounds(app: &PhotocraftApp) -> Option<photocraft_geom::Rect> {
    let b = app.session.active()?.doc.selection.as_ref()?.content_bounds();
    (!b.is_empty()).then_some(b)
}

fn transform_doc_rect(app: &PhotocraftApp) -> Option<photocraft_geom::Rect> {
    let t = app.ui.transform.as_ref()?;
    let [x0, y0, x1, y1] = t.rect;
    let r = photocraft_geom::Rect::new(x0.floor() as i32, y0.floor() as i32, x1.ceil() as i32, y1.ceil() as i32);
    (!r.is_empty()).then_some(r)
}

fn crop_doc_rect(app: &PhotocraftApp) -> Option<photocraft_geom::Rect> {
    let [x0, y0, x1, y1] = app.ui.crop_rect?;
    if !(x0.is_finite() && y0.is_finite() && x1.is_finite() && y1.is_finite()) {
        return None;
    }
    let r = photocraft_geom::Rect::new(x0.floor() as i32, y0.floor() as i32, x1.ceil() as i32, y1.ceil() as i32);
    (!r.is_empty()).then_some(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Pos2, vec2};
    use serde_json::json;

    fn app() -> (PhotocraftApp, egui::Context) {
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default());
        app.run("file.new", json!({"width": 64, "height": 48})).unwrap();
        app.sync_views();
        (app, egui::Context::default())
    }

    #[test]
    fn ui_outlines_noop_when_off() {
        let (app, ctx) = app();
        paint_ui_outlines(&ctx, &app.ui.view);
    }

    #[test]
    fn canvas_outline_rect_uses_central_when_stored_is_stale() {
        let central = Rect::from_min_size(Pos2::new(40.0, 80.0), vec2(900.0, 700.0));
        let stale = Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0));
        assert_eq!(canvas_outline_rect(stale, central), central);
        let inner = Rect::from_min_size(Pos2::new(50.0, 120.0), vec2(400.0, 300.0));
        assert_eq!(canvas_outline_rect(inner, central), inner);
    }

    #[test]
    #[cfg(debug_assertions)]
    fn widget_debug_modes_flip_egui_debug_options() {
        let (mut app, ctx) = app();
        assert!(!ctx.global_style().debug.show_widget_hits);
        app.ui.view.developer.widget_hits = true;
        app.ui.view.extras = true;
        sync_widget_outline_debug(&ctx, &app.ui.view);
        assert!(ctx.global_style().debug.show_widget_hits);
        assert!(!ctx.global_style().debug.show_interactive_widgets);
        app.ui.view.developer.widget_wireframes = true;
        sync_widget_outline_debug(&ctx, &app.ui.view);
        assert!(ctx.global_style().debug.show_interactive_widgets);
        assert!(ctx.global_style().debug.show_widget_hits);
        app.ui.view.developer.widget_focus = true;
        sync_widget_outline_debug(&ctx, &app.ui.view);
        assert!(ctx.global_style().debug.show_focused_widget);
        // Extras off hides overlays without clearing the Developer flags.
        app.ui.view.extras = false;
        sync_widget_outline_debug(&ctx, &app.ui.view);
        assert!(!ctx.global_style().debug.show_widget_hits);
        assert!(app.ui.view.developer.widget_hits);
    }
}
