//! The turntable in the main window's display, where the time was.
//!
//! A picture disc spins at 16⅔ rpm: the playing album's cover shows through
//! the grooves, and a paper label in the middle names the song and the
//! artist. The tone arm tracks across it as the song plays. Classic skins
//! have no bitmaps for it, so it is drawn with shapes in fixed colours on
//! the skin's display colour.

use std::f32::consts::TAU;

use egui::emath::Rot2;
use egui::epaint::{Mesh, PathShape, TextShape, Vertex, WHITE_UV};
use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{Align, Color32, FontId, Id, Pos2, Sense, Stroke, Vec2, pos2, vec2};

use crate::app::{App, NowPlaying};
use crate::model::Action;
use crate::skin::layout::{self, Area};
use crate::skin::{Sheet, Sprite, sprites};

use super::{ShownTime, View, shown_time, stopped};

/// How much taller the main window is with the turntable: the display box
/// grows by this many skin pixels to make room for a square deck.
pub const EXTRA_HEIGHT: u32 = 30;
/// The skin row where the main window is cut and stretched. Everything
/// that starts at or below it moves down by [`EXTRA_HEIGHT`]. It is above
/// the display's contents and below the boxes' top edges, so the
/// stretched row is the inside of the boxes.
pub const SPLIT: u32 = 25;
/// The inside of the display box on the left, in window skin pixels after
/// the stretch. The turntable covers it all: the clutter bar and the
/// visualiser that were in it are left out.
const DISPLAY: Area = Area::new(10, 22, 93, 44 + EXTRA_HEIGHT);
/// The deck, square and centred in the display.
pub const DECK: Area = Area::new(20, 23, 72, 72);

// Half a real LP's 33⅓ rpm on purpose: the real speed looked too fast here.
const RPM: f32 = 50.0 / 3.0;

/// The song box on the right of the display, in the skin's own main
/// bitmap: its left column, its width, and its top and bottom edge rows.
const INFO_X: u32 = 106;
const INFO_WIDTH: u32 = 165;
const INFO_TOP: u32 = 21;
const INFO_BOTTOM: u32 = 55;
/// Rows taken whole from the box's top and bottom when a box of another
/// height is built from it, enough for a bevelled edge.
const INFO_EDGE: u32 = 3;
/// A row of plain panel between the title bar and the boxes.
const PANEL_ROW: u32 = 17;
/// The timer box above the song box, in window rows after the stretch.
const TIMER_TOP: u32 = INFO_TOP;
const TIMER_BOTTOM: u32 = INFO_TOP + EXTRA_HEIGHT - 3;
/// The time digits and the lamp are drawn this many times their size.
const TIMER_SCALE: u32 = 1;

// The deck's own coordinates: a square 100 wide.
const CENTER: Pos2 = pos2(43.0, 54.0);
const PLATTER_RADIUS: f32 = 42.0;
const RECORD_RADIUS: f32 = 40.0;
const LABEL_RADIUS: f32 = 17.0;
/// Where the stylus sits at the first and the last second of a song.
const LEAD_IN_RADIUS: f32 = 38.0;
const RUN_OUT_RADIUS: f32 = 21.0;
/// How strongly the cover shows through the black vinyl.
const PICTURE_ALPHA: u8 = 115;

const PIVOT: Pos2 = pos2(90.0, 11.0);
const ARM_LENGTH: f32 = 58.0;
/// The arm's angle on its rest, clear of the record, in radians from +x.
const REST_ANGLE: f32 = 1.62;

const ALUMINIUM: Color32 = Color32::from_rgb(196, 198, 202);
const ALUMINIUM_DARK: Color32 = Color32::from_rgb(120, 122, 128);
const VINYL: Color32 = Color32::from_rgb(14, 14, 16);
const GROOVE: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 110);
const GROOVE_EDGE: Color32 = Color32::from_rgb(40, 40, 44);
const PAPER: Color32 = Color32::from_rgb(236, 226, 202);
const INK: Color32 = Color32::from_rgb(52, 34, 26);

/// Draws the deck over the display box. A click on it plays or pauses.
/// Returns whether the record is spinning.
pub(super) fn show(app: &mut App, view: &mut View, now: Option<&NowPlaying>, time: f64) -> bool {
    let playing = now.is_some_and(|now| now.playing);
    let spinning = playing && !crate::motion::reduced(view.ui.ctx());
    let angle = record_angle(view.ui.ctx(), spinning, time);
    let [r, g, b] = view.skin.vis_colors[0];
    // Not view.fill: the deck is placed after the stretch, and fill would
    // move it down a second time.
    let deck = Deck {
        painter: view.ui.painter().clone(),
        origin: view.origin + vec2(DECK.x as f32, DECK.y as f32) * view.unit,
        scale: view.unit * DECK.width as f32 / 100.0,
    };
    deck.painter.rect_filled(
        egui::Rect::from_min_size(
            view.origin + vec2(DISPLAY.x as f32, DISPLAY.y as f32) * view.unit,
            vec2(DISPLAY.width as f32, DISPLAY.height as f32) * view.unit,
        ),
        0.0,
        Color32::from_rgb(r, g, b),
    );
    deck.platter(angle);
    deck.picture(view, now, angle);
    deck.grooves();
    deck.label(now, angle);
    deck.tone_arm(view.ui.ctx(), now);

    let rect = egui::Rect::from_min_size(deck.origin, Vec2::splat(100.0 * deck.scale));
    if view
        .ui
        .interact(rect, Id::new(("winamp", "vinyl-deck")), Sense::click())
        .clicked()
        && now.is_some()
    {
        app.actions.push(Action::TogglePlay);
    }
    spinning
}

/// The right side of the taller display: the song box moved down whole to
/// sit on its contents, and above it a timer box with the play lamp and the
/// time. Both boxes are cut from the skin's own song
/// box, so they match any skin. A click on the timer switches between
/// elapsed and remaining time, as on the classic display.
pub(super) fn timer_box(app: &mut App, view: &mut View, now: Option<&NowPlaying>, time: f64) {
    let main =
        |x: u32, y: u32, width: u32, height: u32| Sprite::new(Sheet::Main, x, y, width, height);
    let song_top = INFO_TOP + EXTRA_HEIGHT;
    view.paint_placed(
        main(INFO_X, PANEL_ROW, INFO_WIDTH, 1),
        INFO_X,
        INFO_TOP,
        INFO_WIDTH,
        song_top - INFO_TOP,
    );
    view.paint_placed(
        main(INFO_X, INFO_TOP, INFO_WIDTH, SPLIT - INFO_TOP),
        INFO_X,
        song_top,
        INFO_WIDTH,
        SPLIT - INFO_TOP,
    );

    let edge = |y: u32| main(INFO_X, y, INFO_WIDTH, INFO_EDGE);
    view.paint_placed(edge(INFO_TOP), INFO_X, TIMER_TOP, INFO_WIDTH, INFO_EDGE);
    view.paint_placed(
        main(INFO_X, INFO_TOP + INFO_EDGE, INFO_WIDTH, 1),
        INFO_X,
        TIMER_TOP + INFO_EDGE,
        INFO_WIDTH,
        TIMER_BOTTOM + 1 - TIMER_TOP - 2 * INFO_EDGE,
    );
    view.paint_placed(
        edge(INFO_BOTTOM + 1 - INFO_EDGE),
        INFO_X,
        TIMER_BOTTOM + 1 - INFO_EDGE,
        INFO_WIDTH,
        INFO_EDGE,
    );

    let clock_x = layout::MINUS_EX.x;
    let clock_width = layout::SECOND_ONES.x + layout::SECOND_ONES.width - clock_x;
    let clock_height = layout::MINUS_EX.height;
    let lamp = layout::STATUS;
    let gap = 3;
    let content_width = (lamp.width + gap + clock_width) * TIMER_SCALE;
    let left = INFO_X + (INFO_WIDTH - content_width) / 2;
    let clock_left = left + (lamp.width + gap) * TIMER_SCALE;
    let timer_height = TIMER_BOTTOM + 1 - TIMER_TOP;
    let clock_top = TIMER_TOP + (timer_height - clock_height * TIMER_SCALE) / 2;
    // The lamp sits as far below the digits' top as it does in the classic
    // display.
    let lamp_top = clock_top + (lamp.y - layout::MINUS_EX.y) * TIMER_SCALE;
    let scaled = |view: &View, sprite: Sprite, x: u32, y: u32| {
        view.paint_placed(
            sprite,
            x,
            y,
            sprite.width * TIMER_SCALE,
            sprite.height * TIMER_SCALE,
        );
    };

    let status = match now {
        Some(now) if now.playing || now.loading => sprites::STATUS_PLAYING,
        Some(_) if !stopped(now) => sprites::STATUS_PAUSED,
        _ => sprites::STATUS_STOPPED,
    };
    scaled(view, status, left, lamp_top);
    // The colon is part of the skin's background, not of the digit sheet,
    // so the clock is drawn on a copy of the background under the classic
    // time.
    scaled(
        view,
        main(clock_x, layout::MINUS_EX.y, clock_width, clock_height),
        clock_left,
        clock_top,
    );
    let at = |area: Area| {
        (
            clock_left + (area.x - clock_x) * TIMER_SCALE,
            clock_top + (area.y - layout::MINUS_EX.y) * TIMER_SCALE,
        )
    };
    let extended = view.skin.has_extended_digits();
    let (digits, remaining) = match shown_time(app, now, time) {
        ShownTime::Time { digits, remaining } => (Some(digits), remaining),
        ShownTime::Blank => (None, false),
    };
    for (index, cell) in layout::TIME_DIGITS.into_iter().enumerate() {
        let sprite = match (digits, extended) {
            (Some(digits), true) => sprites::digit_ex(digits[index]),
            (Some(digits), false) => sprites::digit(digits[index]),
            (None, true) => sprites::NUMS_EX_BLANK,
            (None, false) => sprites::NUMBERS_BLANK,
        };
        let (x, y) = at(cell);
        scaled(view, sprite, x, y);
    }
    let (sign, sign_area) = match (extended, remaining) {
        (true, true) => (sprites::NUMS_EX_MINUS, layout::MINUS_EX),
        (true, false) => (sprites::NUMS_EX_BLANK, layout::MINUS_EX),
        (false, true) => (sprites::NUMBERS_MINUS, layout::MINUS),
        (false, false) => (sprites::NUMBERS_NO_MINUS, layout::MINUS),
    };
    let (x, y) = at(sign_area);
    scaled(view, sign, x, y);

    let rect = egui::Rect::from_min_max(
        view.origin + vec2(INFO_X as f32, TIMER_TOP as f32) * view.unit,
        view.origin + vec2((INFO_X + INFO_WIDTH) as f32, (TIMER_BOTTOM + 1) as f32) * view.unit,
    );
    if view
        .ui
        .interact(rect, Id::new(("winamp", "vinyl-timer")), Sense::click())
        .clicked()
    {
        app.winamp.time_remaining = !app.winamp.time_remaining;
    }
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

/// The deck's square on the screen: its top left, and screen points per
/// deck unit.
struct Deck {
    painter: egui::Painter,
    origin: Pos2,
    scale: f32,
}

impl Deck {
    fn at(&self, point: Pos2) -> Pos2 {
        self.origin + point.to_vec2() * self.scale
    }

    fn platter(&self, angle: f32) {
        let center = self.at(CENTER);
        let scale = self.scale;
        self.painter
            .circle_filled(center, (PLATTER_RADIUS + 1.0) * scale, ALUMINIUM_DARK);
        self.painter
            .circle_filled(center, PLATTER_RADIUS * scale, ALUMINIUM);
        // The strobe dots on the platter's edge turn with it, as on a Technics.
        for dot in 0..60 {
            let theta = angle + dot as f32 / 60.0 * TAU;
            let point = center + Vec2::angled(theta) * (PLATTER_RADIUS - 1.0) * scale;
            self.painter
                .circle_filled(point, 0.45 * scale, ALUMINIUM_DARK);
        }
        self.painter
            .circle_filled(center, RECORD_RADIUS * scale, VINYL);
    }

    /// The cover across the whole disc, under the grooves, as on a picture
    /// disc. The label covers its middle.
    fn picture(&self, view: &View, now: Option<&NowPlaying>, angle: f32) {
        let Some(texture) = cover(view, now, RECORD_RADIUS * self.scale) else {
            return;
        };
        let center = self.at(CENTER);
        let tint = Color32::from_white_alpha(PICTURE_ALPHA);
        let (rings, segments) = (6, 96);
        let mut mesh = Mesh::with_texture(texture);
        for ring in 0..=rings {
            let fraction = ring as f32 / rings as f32;
            let radius = LABEL_RADIUS + (RECORD_RADIUS - 1.0 - LABEL_RADIUS) * fraction;
            for segment in 0..=segments {
                let theta = segment as f32 / segments as f32 * TAU;
                let along = Vec2::angled(theta - angle) * (radius / RECORD_RADIUS) * 0.5;
                mesh.vertices.push(Vertex {
                    pos: center + Vec2::angled(theta) * radius * self.scale,
                    uv: pos2(0.5, 0.5) + along,
                    color: tint,
                });
            }
        }
        let row = segments + 1;
        for ring in 0..rings {
            for segment in 0..segments {
                let inner = ring * row + segment;
                let outer = inner + row;
                mesh.add_triangle(inner, outer, inner + 1);
                mesh.add_triangle(inner + 1, outer, outer + 1);
            }
        }
        self.painter.add(mesh);
    }

    /// The grooves over the picture, with the bands between tracks and the
    /// sheen from the room's light, which stays still while the record turns.
    fn grooves(&self) {
        let center = self.at(CENTER);
        let scale = self.scale;
        self.painter.circle_stroke(
            center,
            (RECORD_RADIUS - 0.5) * scale,
            Stroke::new(1.0 * scale, GROOVE_EDGE),
        );
        let mut radius = RECORD_RADIUS - 2.0;
        let mut groove = 0;
        while radius > LABEL_RADIUS + 1.5 {
            let width = if groove % 7 == 6 { 0.9 } else { 0.3 };
            self.painter
                .circle_stroke(center, radius * scale, Stroke::new(width * scale, GROOVE));
            radius -= 1.3;
            groove += 1;
        }
        for (start, sweep) in [(-2.4_f32, 0.7_f32), (0.75, 0.7)] {
            self.sheen(start, sweep);
        }
    }

    fn sheen(&self, start: f32, sweep: f32) {
        let center = self.at(CENTER);
        let steps = 16;
        let inner = (LABEL_RADIUS + 3.0) * self.scale;
        let outer = (RECORD_RADIUS - 1.0) * self.scale;
        let mut mesh = Mesh::default();
        for step in 0..=steps {
            let fraction = step as f32 / steps as f32;
            let theta = start + sweep * fraction;
            let alpha = ((fraction * std::f32::consts::PI).sin() * 34.0) as u8;
            let color = Color32::from_white_alpha(alpha);
            for radius in [inner, outer] {
                mesh.vertices.push(Vertex {
                    pos: center + Vec2::angled(theta) * radius,
                    uv: WHITE_UV,
                    color,
                });
            }
        }
        for step in 0..steps {
            let base = step * 2;
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base + 1, base + 3, base + 2);
        }
        self.painter.add(mesh);
    }

    /// The paper label: the song's title above the spindle, the artist
    /// below it, and STEREO and 33⅓ RPM at the sides, all turning with the
    /// record.
    fn label(&self, now: Option<&NowPlaying>, angle: f32) {
        let center = self.at(CENTER);
        let scale = self.scale;
        self.painter
            .circle_filled(center, LABEL_RADIUS * scale, PAPER);
        self.painter.circle_stroke(
            center,
            (LABEL_RADIUS - 1.1) * scale,
            Stroke::new(0.25 * scale, INK.gamma_multiply(0.5)),
        );
        let rotation = Rot2::from_angle(angle);
        if let Some(now) = now {
            self.print(&now.title, 3.0, 27.0, 2, vec2(0.0, -8.5), rotation);
            let artist = now
                .artists
                .first()
                .map_or(now.subtitle.as_str(), |artist| artist.name.as_str())
                .to_uppercase();
            self.print(&artist, 3.4, 27.0, 1, vec2(0.0, 7.5), rotation);
        }
        let stereo = self.print("STEREO", 1.9, 11.0, 1, vec2(-10.0, 1.8), rotation);
        self.frame(vec2(-10.0, 1.8), stereo + vec2(1.2, 0.7), rotation);
        // Not "33⅓": the fallback fonts have no glyph for the fraction.
        self.print("33 1/3 RPM", 1.9, 12.0, 1, vec2(10.0, 1.8), rotation);

        self.painter
            .circle_filled(center, 2.3 * scale, PAPER.gamma_multiply(0.75));
        self.painter.circle_filled(center, 1.4 * scale, ALUMINIUM);
        self.painter
            .circle_filled(center, 0.7 * scale, ALUMINIUM_DARK);
    }

    /// Centred print on the label, at most `width` deck units wide and
    /// `rows` lines tall, cut short with an ellipsis. `offset` is from the
    /// spindle before the record's turn. Returns its size in deck units.
    fn print(
        &self,
        text: &str,
        size: f32,
        width: f32,
        rows: usize,
        offset: Vec2,
        rotation: Rot2,
    ) -> Vec2 {
        let font = FontId::proportional(size * self.scale);
        let mut job = LayoutJob::single_section(text.to_owned(), TextFormat::simple(font, INK));
        job.wrap = TextWrapping {
            max_width: width * self.scale,
            max_rows: rows,
            break_anywhere: false,
            overflow_character: Some('…'),
        };
        job.halign = Align::Center;
        let galley = self.painter.layout_job(job);
        let measured = galley.rect.size() / self.scale;
        let middle = galley.rect.center().to_vec2();
        let pos = self.at(CENTER) + rotation * (offset * self.scale - middle);
        self.painter
            .add(TextShape::new(pos, galley, INK).with_angle(rotation.angle()));
        measured
    }

    /// A thin box of `size` deck units around `offset`, turned with the
    /// label.
    fn frame(&self, offset: Vec2, size: Vec2, rotation: Rot2) {
        let half = size / 2.0;
        let corners = [
            vec2(-half.x, -half.y),
            vec2(half.x, -half.y),
            vec2(half.x, half.y),
            vec2(-half.x, half.y),
        ]
        .map(|corner| self.at(CENTER) + rotation * ((offset + corner) * self.scale));
        self.painter.add(PathShape::closed_line(
            corners.to_vec(),
            Stroke::new(0.25 * self.scale, INK),
        ));
    }

    fn tone_arm(&self, ctx: &egui::Context, now: Option<&NowPlaying>) {
        let angle = ctx.animate_value_with_time(
            Id::new("winamp-vinyl-arm"),
            arm_angle(now),
            crate::motion::seconds(ctx, 0.8),
        );
        let scale = self.scale;
        let pivot = self.at(PIVOT);
        let direction = Vec2::angled(angle);
        let stylus = pivot + direction * ARM_LENGTH * scale;

        let rest = self.at(PIVOT + Vec2::angled(REST_ANGLE) * (ARM_LENGTH - 9.0));
        self.painter
            .circle_filled(rest, 2.6 * scale, ALUMINIUM_DARK);
        self.painter.circle_filled(rest, 1.7 * scale, ALUMINIUM);

        self.painter
            .circle_filled(pivot, 7.0 * scale, ALUMINIUM_DARK);
        self.painter.circle_filled(pivot, 5.6 * scale, ALUMINIUM);
        let counterweight = pivot - direction * 8.0 * scale;
        self.painter.line_segment(
            [pivot, counterweight],
            Stroke::new(1.8 * scale, ALUMINIUM_DARK),
        );
        self.painter
            .circle_filled(counterweight, 3.0 * scale, Color32::from_rgb(40, 40, 44));
        let shadow = vec2(0.6, 0.8) * scale;
        self.painter.line_segment(
            [pivot + shadow, stylus + shadow],
            Stroke::new(1.5 * scale, Color32::from_black_alpha(90)),
        );
        self.painter
            .line_segment([pivot, stylus], Stroke::new(1.3 * scale, ALUMINIUM));
        let head_end = stylus + Vec2::angled(angle + 0.5) * 5.5 * scale;
        self.painter.line_segment(
            [stylus, head_end],
            Stroke::new(2.8 * scale, Color32::from_rgb(30, 30, 34)),
        );
        self.painter
            .circle_filled(pivot, 1.8 * scale, ALUMINIUM_DARK);
    }
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
    let stylus = [foot + normal * across, foot - normal * across]
        .into_iter()
        .max_by(|a, b| a.y.total_cmp(&b.y))?;
    Some((stylus - PIVOT).angle())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skin::layout;

    #[test]
    fn the_stylus_reaches_every_groove_from_lead_in_to_run_out() {
        for radius in [LEAD_IN_RADIUS, 30.0, RUN_OUT_RADIUS] {
            let angle = stylus_angle(radius).expect("the arm reaches the groove");
            let stylus = PIVOT + Vec2::angled(angle) * ARM_LENGTH;
            assert!(((stylus - CENTER).length() - radius).abs() < 0.01);
        }
    }

    #[test]
    fn the_arm_rests_clear_of_the_record_and_inside_the_deck() {
        let stylus = PIVOT + Vec2::angled(REST_ANGLE) * ARM_LENGTH;
        assert!((stylus - CENTER).length() > PLATTER_RADIUS + 2.0);
        assert!((0.0..100.0).contains(&stylus.x) && (0.0..100.0).contains(&stylus.y));
    }

    #[test]
    fn the_deck_fits_inside_the_display() {
        assert_eq!(DECK.width, DECK.height);
        assert!(DECK.x >= DISPLAY.x && DECK.y >= DISPLAY.y);
        assert!(DECK.x + DECK.width <= DISPLAY.x + DISPLAY.width);
        assert!(DECK.y + DECK.height <= DISPLAY.y + DISPLAY.height);
        for (name, area) in layout::ALL {
            let crosses = area.y < SPLIT && area.y + area.height > SPLIT;
            // The clutter bar is not drawn with the turntable.
            assert!(
                !crosses || *name == "CLUTTER_BAR",
                "{name} crosses the stretched row"
            );
        }
    }

    #[test]
    fn the_timer_box_fits_its_digits_above_the_song_box() {
        let clock = layout::MINUS_EX.height * TIMER_SCALE;
        assert!(TIMER_BOTTOM + 1 - TIMER_TOP >= clock + 2 * INFO_EDGE);
        assert!(TIMER_BOTTOM < INFO_TOP + EXTRA_HEIGHT);
    }
}
