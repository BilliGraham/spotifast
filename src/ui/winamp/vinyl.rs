//! The turntable window, between the main window and the equalizer.
//!
//! A record spins at 33⅓ rpm with the playing album's cover as its label,
//! and the tone arm tracks across it as the song plays. Classic skins have
//! no bitmaps for this window, so it is drawn with shapes in fixed colours
//! and only the text comes from the skin's font.

use std::f32::consts::TAU;

use egui::epaint::{Mesh, Vertex, WHITE_UV};
use egui::{Color32, Id, Pos2, Sense, Stroke, Vec2, pos2, vec2};

use crate::app::{App, NowPlaying};
use crate::i18n::gettext;
use crate::model::Action;
use crate::skin::layout::{self, Area};

use super::View;

/// The window's height in skin pixels, the same as the main window's.
pub const HEIGHT: u32 = 116;

const RPM: f32 = 100.0 / 3.0;
const TITLE_BAR: Area = Area::new(0, 0, layout::WINDOW_WIDTH, 12);

const CENTER: Pos2 = pos2(66.0, 64.0);
const PLATTER_RADIUS: f32 = 47.0;
const RECORD_RADIUS: f32 = 44.0;
const LABEL_RADIUS: f32 = 18.0;
/// Where the stylus sits at the first and the last second of a song.
const LEAD_IN_RADIUS: f32 = 42.0;
const RUN_OUT_RADIUS: f32 = 22.0;

const PIVOT: Pos2 = pos2(128.0, 24.0);
const ARM_LENGTH: f32 = 66.0;
/// The arm's angle on its rest, clear of the record, in radians from +x.
const REST_ANGLE: f32 = 1.48;

const TEXT_X: u32 = 150;
const TEXT_WIDTH: u32 = 118;

const WOOD: Color32 = Color32::from_rgb(92, 54, 30);
const WOOD_DARK: Color32 = Color32::from_rgb(58, 32, 17);
const WOOD_LIGHT: Color32 = Color32::from_rgb(128, 80, 46);
const STRIP: Color32 = Color32::from_rgb(24, 20, 18);
const ALUMINIUM: Color32 = Color32::from_rgb(196, 198, 202);
const ALUMINIUM_DARK: Color32 = Color32::from_rgb(120, 122, 128);
const VINYL: Color32 = Color32::from_rgb(14, 14, 16);
const GROOVE: Color32 = Color32::from_rgb(34, 34, 38);
const LED_ON: Color32 = Color32::from_rgb(255, 64, 40);
const LED_OFF: Color32 = Color32::from_rgb(80, 24, 18);

pub(super) fn show(app: &mut App, view: &mut View, now: Option<&NowPlaying>, time: f64) -> bool {
    let playing = now.is_some_and(|now| now.playing);
    let spinning = playing && !crate::motion::reduced(view.ui.ctx());
    let angle = record_angle(view.ui.ctx(), spinning, time);

    plinth(view);
    title_bar(app, view);
    platter(view, angle);
    label(view, now, angle);
    if view
        .interact(
            Area::new(
                (CENTER.x - RECORD_RADIUS) as u32,
                (CENTER.y - RECORD_RADIUS) as u32,
                (RECORD_RADIUS * 2.0) as u32,
                (RECORD_RADIUS * 2.0) as u32,
            ),
            "vinyl-record",
            Sense::click(),
        )
        .on_hover_text(gettext(app.locale, "Play or pause").as_ref())
        .clicked()
        && now.is_some()
    {
        app.actions.push(Action::TogglePlay);
    }
    tone_arm(view, now);
    details(view, now, playing, time);
    spinning
}

/// The record's turn in radians. It only advances while the record spins,
/// so a paused record stays where it stopped instead of jumping ahead.
fn record_angle(ctx: &egui::Context, spinning: bool, time: f64) -> f32 {
    let id = Id::new("winamp-vinyl-angle");
    let (last, angle) = ctx
        .data(|data| data.get_temp::<(f64, f32)>(id))
        .unwrap_or((time, 0.0));
    let step = if spinning {
        // A long gap means the window was hidden, not that the record ran.
        ((time - last).min(0.25) as f32) * RPM / 60.0 * TAU
    } else {
        0.0
    };
    let angle = (angle + step) % TAU;
    ctx.data_mut(|data| data.insert_temp(id, (time, angle)));
    angle
}

fn at(view: &View, point: Pos2) -> Pos2 {
    view.origin + point.to_vec2() * view.unit
}

fn plinth(view: &View) {
    view.fill(0, 0, layout::WINDOW_WIDTH, HEIGHT, WOOD_DARK);
    view.fill(1, 1, layout::WINDOW_WIDTH - 2, HEIGHT - 2, WOOD_LIGHT);
    view.fill(2, 2, layout::WINDOW_WIDTH - 3, HEIGHT - 3, WOOD_DARK);
    view.fill(2, 2, layout::WINDOW_WIDTH - 4, HEIGHT - 4, WOOD);
    for row in (16..HEIGHT - 4).step_by(7) {
        let shade = if row % 2 == 0 { WOOD_DARK } else { WOOD_LIGHT };
        view.fill(
            3,
            row,
            layout::WINDOW_WIDTH - 6,
            1,
            shade.gamma_multiply(0.35),
        );
    }
}

fn title_bar(app: &mut App, view: &mut View) {
    view.fill(
        TITLE_BAR.x,
        TITLE_BAR.y,
        TITLE_BAR.width,
        TITLE_BAR.height,
        STRIP,
    );
    view.text("TURNTABLE", Area::new(6, 3, TITLE_BAR.width - 12, 6));
    let title = view.interact(TITLE_BAR, "vinyl-title", Sense::click_and_drag());
    if title.drag_started() {
        view.ui
            .ctx()
            .send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    if title.double_clicked() {
        app.actions.push(Action::ToggleWinampVinyl);
    }
}

fn platter(view: &View, angle: f32) {
    let painter = view.ui.painter();
    let unit = view.unit;
    let center = at(view, CENTER);
    painter.circle_filled(center, (PLATTER_RADIUS + 1.0) * unit, ALUMINIUM_DARK);
    painter.circle_filled(center, PLATTER_RADIUS * unit, ALUMINIUM);
    // The strobe dots on the platter's edge turn with it, as on a Technics.
    for dot in 0..60 {
        let theta = angle + dot as f32 / 60.0 * TAU;
        let point = center + Vec2::angled(theta) * (PLATTER_RADIUS - 1.2) * unit;
        painter.circle_filled(point, 0.5 * unit, ALUMINIUM_DARK);
    }
    painter.circle_filled(center, RECORD_RADIUS * unit, VINYL);
    let mut radius = RECORD_RADIUS - 2.0;
    while radius > LABEL_RADIUS + 2.0 {
        painter.circle_stroke(center, radius * unit, Stroke::new(0.35 * unit, GROOVE));
        radius -= 1.6;
    }
    // A record's sheen comes from the room's light, so it stays still while
    // the record turns under it.
    for (start, sweep) in [(-2.4_f32, 0.7_f32), (0.75, 0.7)] {
        sheen(painter, center, unit, start, sweep);
    }
}

fn sheen(painter: &egui::Painter, center: Pos2, unit: f32, start: f32, sweep: f32) {
    let steps = 16;
    let inner = (LABEL_RADIUS + 3.0) * unit;
    let outer = (RECORD_RADIUS - 1.0) * unit;
    let mut mesh = Mesh::default();
    for step in 0..=steps {
        let fraction = step as f32 / steps as f32;
        let theta = start + sweep * fraction;
        let alpha = ((fraction * std::f32::consts::PI).sin() * 34.0) as u8;
        let color = Color32::from_white_alpha(alpha);
        mesh.vertices.push(Vertex {
            pos: center + Vec2::angled(theta) * inner,
            uv: WHITE_UV,
            color,
        });
        mesh.vertices.push(Vertex {
            pos: center + Vec2::angled(theta) * outer,
            uv: WHITE_UV,
            color,
        });
    }
    for step in 0..steps {
        let base = step * 2;
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base + 1, base + 3, base + 2);
    }
    painter.add(mesh);
}

/// The album cover, cut to a circle and turned with the record.
fn label(view: &View, now: Option<&NowPlaying>, angle: f32) {
    let painter = view.ui.painter();
    let unit = view.unit;
    let center = at(view, CENTER);
    let radius = LABEL_RADIUS * unit;
    match cover(view, now, radius) {
        Some(texture) => {
            let segments = 64;
            let mut mesh = Mesh::with_texture(texture);
            mesh.vertices.push(Vertex {
                pos: center,
                uv: pos2(0.5, 0.5),
                color: Color32::WHITE,
            });
            for segment in 0..=segments {
                let theta = segment as f32 / segments as f32 * TAU;
                let uv = pos2(0.5, 0.5) + Vec2::angled(theta - angle) * 0.5;
                mesh.vertices.push(Vertex {
                    pos: center + Vec2::angled(theta) * radius,
                    uv,
                    color: Color32::WHITE,
                });
            }
            for segment in 0..segments {
                mesh.add_triangle(0, segment + 1, segment + 2);
            }
            painter.add(mesh);
        }
        None => {
            painter.circle_filled(center, radius, Color32::from_rgb(178, 34, 34));
            let mark = center + Vec2::angled(angle) * radius * 0.6;
            painter.circle_filled(mark, 1.5 * unit, Color32::from_rgb(240, 220, 180));
        }
    }
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(0.5 * unit, Color32::from_black_alpha(160)),
    );
    painter.circle_filled(center, 1.6 * unit, ALUMINIUM);
    painter.circle_filled(center, 0.8 * unit, ALUMINIUM_DARK);
}

/// The playing cover's texture, once it is loaded.
fn cover(view: &View, now: Option<&NowPlaying>, radius: f32) -> Option<egui::TextureId> {
    let now = now?;
    let url = now.art_url.as_deref().or(now.art_small.as_deref())?;
    let ctx = view.ui.ctx();
    let pixels = (radius * 2.0 * ctx.pixels_per_point()).ceil();
    let scaled = (url.starts_with("https://") || url.starts_with("http://"))
        .then(|| crate::images::scaled_uri(url, crate::images::cover_pixels(pixels)));
    let uri = scaled.as_deref().unwrap_or(url);
    let image = egui::Image::new(uri)
        .texture_options(crate::images::COVER_TEXTURE)
        .show_loading_spinner(false);
    match image.load_for_size(ctx, Vec2::splat(radius * 2.0)) {
        Ok(egui::load::TexturePoll::Ready { texture }) => {
            crate::images::forget_decoded(ctx, uri);
            Some(texture.id)
        }
        _ => None,
    }
}

/// Where the stylus rests for a song this far through, as the arm's angle.
fn arm_angle(now: Option<&NowPlaying>) -> f32 {
    let Some(now) = now.filter(|now| now.playing || now.position_ms > 0) else {
        return REST_ANGLE;
    };
    let progress = if now.duration_ms == 0 {
        0.0
    } else {
        (now.position_ms as f32 / now.duration_ms as f32).clamp(0.0, 1.0)
    };
    let radius = LEAD_IN_RADIUS + (RUN_OUT_RADIUS - LEAD_IN_RADIUS) * progress;
    stylus_angle(radius).unwrap_or(REST_ANGLE)
}

/// The arm's angle that puts the stylus `radius` from the spindle: where
/// the circle the stylus swings on meets the groove's circle, on the side
/// nearer the arm's rest.
fn stylus_angle(radius: f32) -> Option<f32> {
    let to_center = CENTER - PIVOT;
    let distance = to_center.length();
    let along =
        (ARM_LENGTH * ARM_LENGTH - radius * radius + distance * distance) / (2.0 * distance);
    let across = (ARM_LENGTH * ARM_LENGTH - along * along).sqrt();
    if across.is_nan() {
        return None;
    }
    let direction = to_center / distance;
    let foot = PIVOT + direction * along;
    let normal = vec2(-direction.y, direction.x);
    let candidates = [foot + normal * across, foot - normal * across];
    let stylus = candidates.into_iter().max_by(|a, b| a.y.total_cmp(&b.y))?;
    Some((stylus - PIVOT).angle())
}

fn tone_arm(view: &View, now: Option<&NowPlaying>) {
    let ctx = view.ui.ctx();
    let angle = ctx.animate_value_with_time(
        Id::new("winamp-vinyl-arm"),
        arm_angle(now),
        crate::motion::seconds(ctx, 0.8),
    );
    let painter = view.ui.painter();
    let unit = view.unit;
    let pivot = at(view, PIVOT);
    let direction = Vec2::angled(angle);
    let stylus = pivot + direction * ARM_LENGTH * unit;

    let rest = at(view, PIVOT + Vec2::angled(REST_ANGLE) * (ARM_LENGTH - 10.0));
    painter.circle_filled(rest, 3.0 * unit, ALUMINIUM_DARK);
    painter.circle_filled(rest, 2.0 * unit, ALUMINIUM);

    painter.circle_filled(pivot, 8.0 * unit, ALUMINIUM_DARK);
    painter.circle_filled(pivot, 6.5 * unit, ALUMINIUM);
    let counterweight = pivot - direction * 9.0 * unit;
    painter.line_segment(
        [pivot, counterweight],
        Stroke::new(2.0 * unit, ALUMINIUM_DARK),
    );
    painter.circle_filled(counterweight, 3.5 * unit, Color32::from_rgb(40, 40, 44));
    painter.line_segment(
        [
            pivot + vec2(0.6, 0.8) * unit,
            stylus + vec2(0.6, 0.8) * unit,
        ],
        Stroke::new(1.6 * unit, Color32::from_black_alpha(90)),
    );
    painter.line_segment([pivot, stylus], Stroke::new(1.4 * unit, ALUMINIUM));
    let head_direction = Vec2::angled(angle + 0.5);
    let head_end = stylus + head_direction * 6.0 * unit;
    painter.line_segment(
        [stylus, head_end],
        Stroke::new(3.0 * unit, Color32::from_rgb(30, 30, 34)),
    );
    painter.circle_filled(pivot, 2.0 * unit, ALUMINIUM_DARK);
}

fn details(view: &mut View, now: Option<&NowPlaying>, playing: bool, time: f64) {
    view.fill(TEXT_X - 3, 22, TEXT_WIDTH + 4, 44, STRIP);
    if let Some(now) = now {
        let artists = now
            .artists
            .iter()
            .map(|artist| artist.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let lines = [
            now.title.as_str(),
            artists.as_str(),
            now.album_name.as_str(),
        ];
        for (index, line) in lines.into_iter().enumerate() {
            let y = 27 + index as u32 * 12;
            view.text(&ticker(line, time), Area::new(TEXT_X, y, TEXT_WIDTH, 6));
        }
    }
    view.text("33 RPM", Area::new(TEXT_X, 78, 40, 6));
    let led = at(view, pos2(TEXT_X as f32 + 2.0, 98.0));
    let painter = view.ui.painter();
    painter.circle_filled(led, 2.4 * view.unit, Color32::from_black_alpha(160));
    painter.circle_filled(led, 1.8 * view.unit, if playing { LED_ON } else { LED_OFF });
    let state = if playing { "PLAY" } else { "STOP" };
    view.text(state, Area::new(TEXT_X + 8, 95, 40, 6));
}

/// A line of text that scrolls when it is wider than the display.
fn ticker(text: &str, time: f64) -> String {
    let width = (TEXT_WIDTH / 5) as usize;
    let characters: Vec<char> = text.chars().collect();
    if characters.len() <= width {
        return text.to_owned();
    }
    let looped: Vec<char> = characters
        .iter()
        .copied()
        .chain("  ***  ".chars())
        .collect();
    let offset = (time * 4.0) as usize % looped.len();
    looped.iter().cycle().skip(offset).take(width).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stylus_reaches_every_groove_from_lead_in_to_run_out() {
        for radius in [LEAD_IN_RADIUS, 30.0, RUN_OUT_RADIUS] {
            let angle = stylus_angle(radius).expect("the arm reaches the groove");
            let stylus = PIVOT + Vec2::angled(angle) * ARM_LENGTH;
            assert!(((stylus - CENTER).length() - radius).abs() < 0.01);
        }
    }

    #[test]
    fn the_arm_rests_clear_of_the_record() {
        let stylus = PIVOT + Vec2::angled(REST_ANGLE) * ARM_LENGTH;
        assert!((stylus - CENTER).length() > PLATTER_RADIUS + 2.0);
        assert!(stylus.y < HEIGHT as f32);
    }

    #[test]
    fn long_lines_scroll_and_short_lines_stay() {
        assert_eq!(ticker("Short", 10.0), "Short");
        let long = "A title much longer than the turntable's display can show";
        assert_ne!(ticker(long, 0.0), ticker(long, 1.0));
        assert_eq!(ticker(long, 3.0).chars().count(), (TEXT_WIDTH / 5) as usize);
    }
}
