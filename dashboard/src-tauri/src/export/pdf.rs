//! The research report (doc 10 §8).
//!
//! A fixed-grid composer: everything is drawn at measured coordinates on A4,
//! with no layout engine, because the report's shape never changes and a page
//! that always looks the same is a page a reader can scan.
//!
//! The document is labelled a research and engineering report on every page.
//! Nothing here is a diagnosis, and nothing carries a pass or a fail: doc 06
//! defines no threshold that separates a good cycle from a bad one, so the
//! report shows distributions and lets the reader decide.

use krilla::color::rgb;
use krilla::document::Document;
use krilla::geom::{Path, PathBuilder, Point, Rect};
use krilla::page::PageSettings;
use krilla::paint::{Fill, Stroke};
use krilla::surface::Surface;
use krilla::text::{Font, TextDirection};

use crate::store::{Session, StatusChange, StoredCycle, StoredReference, StoredSegment};

/// A4 in points.
const WIDTH: f32 = 595.28;
const HEIGHT: f32 = 841.89;
const MARGIN: f32 = 48.0;
const CONTENT: f32 = WIDTH - 2.0 * MARGIN;

/// The dashboard's own palette, so a printed page matches the screen.
fn ink() -> rgb::Color {
    rgb::Color::new(0x15, 0x18, 0x1c)
}
fn muted() -> rgb::Color {
    rgb::Color::new(0x86, 0x8c, 0x93)
}
fn rule_grey() -> rgb::Color {
    rgb::Color::new(0xdf, 0xe3, 0xe6)
}
fn wash() -> rgb::Color {
    rgb::Color::new(0xec, 0xee, 0xed)
}

/// Latin subsets of the faces the dashboard itself uses, so a printed number
/// lines up with the one on screen.
const SANS: &[u8] = include_bytes!("../../assets/fonts/ibm-plex-sans-400.ttf");
const SANS_BOLD: &[u8] = include_bytes!("../../assets/fonts/ibm-plex-sans-600.ttf");
const MONO: &[u8] = include_bytes!("../../assets/fonts/ibm-plex-mono-400.ttf");

struct Fonts {
    sans: Font,
    bold: Font,
    mono: Font,
}

impl Fonts {
    fn load() -> Option<Self> {
        Some(Self {
            sans: Font::new(SANS.into(), 0)?,
            bold: Font::new(SANS_BOLD.into(), 0)?,
            mono: Font::new(MONO.into(), 0)?,
        })
    }
}

fn fill(color: rgb::Color) -> Fill {
    Fill { paint: color.into(), ..Default::default() }
}

fn stroke(color: rgb::Color, width: f32) -> Stroke {
    Stroke { paint: color.into(), width, ..Default::default() }
}

fn rect_path(x: f32, y: f32, w: f32, h: f32) -> Option<Path> {
    let mut builder = PathBuilder::new();
    builder.push_rect(Rect::from_xywh(x, y, w.max(0.01), h.max(0.01))?);
    builder.finish()
}

fn line_path(points: &[(f32, f32)]) -> Option<Path> {
    let mut builder = PathBuilder::new();
    let (first_x, first_y) = *points.first()?;
    builder.move_to(first_x, first_y);
    for (x, y) in &points[1..] {
        builder.line_to(*x, *y);
    }
    builder.finish()
}

/// Everything drawn on one page, so a page is written in one place.
struct Canvas<'a, 'b> {
    surface: &'a mut Surface<'b>,
    fonts: &'a Fonts,
}

impl Canvas<'_, '_> {
    fn text(&mut self, x: f32, y: f32, size: f32, font: &Font, color: rgb::Color, text: &str) {
        self.surface.set_fill(Some(fill(color)));
        self.surface.draw_text(
            Point::from_xy(x, y),
            font.clone(),
            size,
            text,
            false,
            TextDirection::Auto,
        );
    }

    fn label(&mut self, x: f32, y: f32, text: &str) {
        let font = self.fonts.sans.clone();
        self.text(x, y, 7.5, &font, muted(), text);
    }

    fn body(&mut self, x: f32, y: f32, text: &str) {
        let font = self.fonts.sans.clone();
        self.text(x, y, 9.0, &font, ink(), text);
    }

    fn number(&mut self, x: f32, y: f32, size: f32, text: &str) {
        let font = self.fonts.mono.clone();
        self.text(x, y, size, &font, ink(), text);
    }

    fn heading(&mut self, y: f32, text: &str) {
        let font = self.fonts.bold.clone();
        self.text(MARGIN, y, 11.0, &font, ink(), text);
        self.rule(y + 5.0);
    }

    fn rule(&mut self, y: f32) {
        if let Some(path) = line_path(&[(MARGIN, y), (WIDTH - MARGIN, y)]) {
            self.surface.set_stroke(Some(stroke(rule_grey(), 0.6)));
            self.surface.set_fill(None);
            self.surface.draw_path(&path);
        }
    }

    fn filled(&mut self, x: f32, y: f32, w: f32, h: f32, color: rgb::Color) {
        if let Some(path) = rect_path(x, y, w, h) {
            self.surface.set_fill(Some(fill(color)));
            self.surface.set_stroke(None);
            self.surface.draw_path(&path);
        }
    }

    fn polyline(&mut self, points: &[(f32, f32)], color: rgb::Color, width: f32) {
        if points.len() < 2 {
            return;
        }
        if let Some(path) = line_path(points) {
            self.surface.set_fill(None);
            self.surface.set_stroke(Some(stroke(color, width)));
            self.surface.draw_path(&path);
        }
    }

    /// A readout: small grey label, the value in mono beneath it.
    fn readout(&mut self, x: f32, y: f32, label: &str, value: &str, unit: &str) {
        self.label(x, y, label);
        self.number(x, y + 13.0, 12.0, value);
        if !unit.is_empty() {
            let width = value.len() as f32 * 7.2;
            let font = self.fonts.sans.clone();
            self.text(x + width + 3.0, y + 13.0, 7.5, &font, muted(), unit);
        }
    }

    /// A line chart in a box, with its own y range printed. Returns nothing:
    /// a series with no finite value draws the frame and says "not measured",
    /// which is the honest picture of a feature the device never produced.
    fn chart(&mut self, x: f32, y: f32, w: f32, h: f32, title: &str, values: &[f32]) {
        self.label(x, y - 4.0, title);
        self.filled(x, y, w, h, wash());
        let finite: Vec<f32> = values.iter().copied().filter(|v| v.is_finite()).collect();
        if finite.len() < 2 {
            let font = self.fonts.sans.clone();
            self.text(x + 6.0, y + h / 2.0, 8.0, &font, muted(), "not measured");
            return;
        }
        let low = finite.iter().copied().fold(f32::INFINITY, f32::min);
        let high = finite.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let span = if (high - low).abs() < 1e-6 { 1.0 } else { high - low };
        let points: Vec<(f32, f32)> = finite
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let px = x + 4.0 + (w - 8.0) * i as f32 / (finite.len() - 1).max(1) as f32;
                let py = y + h - 4.0 - (h - 8.0) * (v - low) / span;
                (px, py)
            })
            .collect();
        self.polyline(&points, ink(), 1.0);
        let font = self.fonts.mono.clone();
        self.text(x + w - 40.0, y + 9.0, 6.5, &font, muted(), &format!("{high:.2}"));
        self.text(x + w - 40.0, y + h - 3.0, 6.5, &font, muted(), &format!("{low:.2}"));
    }

    /// A histogram of values already bucketed into counts.
    fn histogram(&mut self, x: f32, y: f32, w: f32, h: f32, counts: &[usize], labels: &[&str]) {
        self.filled(x, y, w, h, wash());
        let peak = counts.iter().copied().max().unwrap_or(0).max(1) as f32;
        let step = w / counts.len() as f32;
        for (i, count) in counts.iter().enumerate() {
            let bar = (h - 14.0) * *count as f32 / peak;
            self.filled(x + i as f32 * step + 2.0, y + h - 12.0 - bar, step - 4.0, bar, ink());
            if let Some(label) = labels.get(i) {
                let font = self.fonts.mono.clone();
                self.text(x + i as f32 * step + 2.0, y + h - 3.0, 6.0, &font, muted(), label);
            }
        }
    }

    /// Two columns of `label: value`, returning the y it finished at.
    fn pairs(&mut self, x: f32, mut y: f32, width: f32, rows: &[(&str, String)]) -> f32 {
        for (label, value) in rows {
            self.label(x, y, label);
            let font = self.fonts.mono.clone();
            self.text(x + 132.0, y, 8.5, &font, ink(), value);
            let _ = width;
            y += 14.0;
        }
        y
    }

    /// A wrapped paragraph in the small grey face, returning the y it ended at.
    ///
    /// The line width is estimated from a mean advance rather than measured:
    /// IBM Plex Sans averages close to 0.52 em over lowercase Latin, and the
    /// report's prose is all lowercase Latin. An estimate is enough because
    /// nothing here is set flush right; a line that comes out a few points
    /// short is invisible, and the alternative is shaping every candidate line
    /// twice.
    fn paragraph(&mut self, x: f32, mut y: f32, width: f32, size: f32, text: &str) -> f32 {
        let per_line = ((width / (size * 0.52)) as usize).max(20);
        let mut line = String::new();
        let font = self.fonts.sans.clone();
        for word in text.split_whitespace() {
            if !line.is_empty() && line.len() + 1 + word.len() > per_line {
                self.text(x, y, size, &font, muted(), &line);
                y += size * 1.45;
                line.clear();
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        if !line.is_empty() {
            self.text(x, y, size, &font, muted(), &line);
            y += size * 1.45;
        }
        y
    }

    fn footer(&mut self, page: usize, pages: usize, session: &str) {
        self.rule(HEIGHT - MARGIN + 10.0);
        let font = self.fonts.sans.clone();
        self.text(
            MARGIN,
            HEIGHT - MARGIN + 22.0,
            7.0,
            &font,
            muted(),
            "Research and engineering report. Not a validated clinical diagnostic report.",
        );
        let mono = self.fonts.mono.clone();
        self.text(
            WIDTH - MARGIN - 96.0,
            HEIGHT - MARGIN + 22.0,
            7.0,
            &mono,
            muted(),
            &format!("{session}  {page}/{pages}"),
        );
    }
}

fn median(values: &[f32]) -> Option<f32> {
    let mut sorted: Vec<f32> = values.iter().copied().filter(|v| v.is_finite()).collect();
    if sorted.is_empty() {
        return None;
    }
    sorted.sort_by(|a, b| a.partial_cmp(b).expect("no NaN after the filter"));
    Some(sorted[sorted.len() / 2])
}

fn show(value: Option<f32>, digits: usize) -> String {
    match value {
        Some(v) => format!("{v:.digits$}"),
        None => "—".into(),
    }
}

/// Builds the report. Everything comes from the store, so the same session
/// produces the same document whenever it is asked for.
pub fn report(
    session: &Session,
    cycles: &[StoredCycle],
    segments: &[StoredSegment],
    status: &[StatusChange],
    reference: Option<&StoredReference>,
    classes: &[&str],
) -> super::Result<Vec<u8>> {
    let fonts = Fonts::load()
        .ok_or_else(|| super::ExportError::Encode("the bundled report fonts are unreadable".into()))?;
    let mut document = Document::new();
    document.set_metadata(
        krilla::metadata::Metadata::new()
            .title(format!("EAD V1 gait session report — {}", session.session_id))
            .creator("EAD V1 dashboard".into())
            .producer(format!("EAD V1 dashboard {}", env!("CARGO_PKG_VERSION")))
            .description(
                "Research and engineering report. Not a validated clinical diagnostic report."
                    .into(),
            )
            .authors(vec!["EAD V1 dashboard".into()]),
    );
    let pages = 3;

    let valid: Vec<&StoredCycle> = cycles.iter().filter(|c| c.valid).collect();
    let measured: Vec<&StoredCycle> = valid.iter().copied().filter(|c| c.zupt_quality >= 0.15).collect();
    let scored: Vec<&StoredCycle> = if session.reference_id.is_some() {
        valid.iter().copied().filter(|c| c.confidence > 0.0).collect()
    } else {
        Vec::new()
    };
    let distance: f32 = measured.iter().map(|c| c.distance_m).sum();
    let walking: f32 = measured.iter().map(|c| c.cycle_time_s).sum();

    // ---- page 1: what the session was, and what it came to -----------------
    {
        let mut page = document.start_page_with(PageSettings::new(
            krilla::geom::Size::from_wh(WIDTH, HEIGHT).expect("A4"),
        ));
        let mut surface = page.surface();
        let mut c = Canvas { surface: &mut surface, fonts: &fonts };

        c.text(MARGIN, MARGIN + 6.0, 16.0, &fonts.bold, ink(), "EAD V1 gait session report");
        c.paragraph(
            MARGIN,
            MARGIN + 22.0,
            CONTENT,
            8.5,
            "Right-leg wearable inertial measurement. Research and engineering data: this \
document is not a validated clinical diagnostic report.",
        );
        c.rule(MARGIN + 32.0);

        let mut y = MARGIN + 52.0;
        c.heading(y, "Session");
        y += 20.0;
        let left = vec![
            ("Patient", format!("{} ({})", session.patient_name, session.patient_id)),
            ("Session", session.session_id.clone()),
            ("Kind", session.kind.replace('_', " ")),
            ("Started", session.started_at.replace('T', " ")),
            ("Stopped", session.stopped_at.clone().unwrap_or_else(|| "still open".into()).replace('T', " ")),
            ("Firmware", session.firmware.clone().unwrap_or_else(|| "unknown".into())),
        ];
        c.pairs(MARGIN, y, CONTENT / 2.0, &left);
        let right = vec![
            (
                "Reference profile",
                match reference {
                    Some(r) => format!("v{} ({} cycles)", r.version, r.cycles),
                    None => "none: this session was not scored".into(),
                },
            ),
            ("Reference locked", reference.map(|r| r.locked.to_string()).unwrap_or_else(|| "—".into())),
            ("Frames stored", session.frames_stored.to_string()),
            ("Frames never arrived", session.frames_missing.to_string()),
            ("Cycles detected", cycles.len().to_string()),
            ("Cycles valid", valid.len().to_string()),
        ];
        c.pairs(MARGIN + CONTENT / 2.0 + 10.0, y, CONTENT / 2.0, &right);
        y += 6.0 * 14.0 + 16.0;

        c.heading(y, "Session measures");
        y += 24.0;
        let column = CONTENT / 4.0;
        c.readout(
            MARGIN,
            y,
            "Mean speed",
            &show((walking > 0.0).then_some(distance / walking), 2),
            "m/s",
        );
        c.readout(
            MARGIN + column,
            y,
            "Cadence (median)",
            &show(median(&valid.iter().map(|c| c.cadence_steps_per_min).collect::<Vec<_>>()), 0),
            "steps/min",
        );
        c.readout(
            MARGIN + 2.0 * column,
            y,
            "Stance ratio (median)",
            &show(median(&valid.iter().map(|c| c.stance_ratio).collect::<Vec<_>>()), 2),
            "",
        );
        c.readout(
            MARGIN + 3.0 * column,
            y,
            "Swing ratio (median)",
            &show(median(&valid.iter().map(|c| c.swing_ratio).collect::<Vec<_>>()), 2),
            "",
        );
        y += 38.0;
        c.readout(
            MARGIN,
            y,
            "Unilateral cycle symmetry proxy",
            &show(median(&valid.iter().filter_map(|c| c.symmetry_proxy).collect::<Vec<_>>()), 2),
            "",
        );
        c.readout(
            MARGIN + column,
            y,
            "ZUPT quality (median)",
            &show(median(&valid.iter().map(|c| c.zupt_quality).collect::<Vec<_>>()), 2),
            "",
        );
        c.readout(MARGIN + 2.0 * column, y, "Distance (adequate ZUPT)", &format!("{distance:.2}"), "m");
        c.readout(
            MARGIN + 3.0 * column,
            y,
            "Error score (median)",
            &show(median(&scored.iter().map(|c| c.error_score).collect::<Vec<_>>()), 2),
            "",
        );
        y += 40.0;
        y = c.paragraph(
            MARGIN,
            y,
            CONTENT,
            7.5,
            &format!(
                "Distance and speed sum only the {} of {} valid cycles whose zero-velocity \
quality reaches 0.15; the rest are reported per cycle but not counted, never corrected \
(doc 05 §8). The symmetry proxy is cycle repeatability against the previous valid cycle \
and is never a left-versus-right claim: only the right leg is instrumented.",
                measured.len(),
                valid.len()
            ),
        ) + 10.0;

        c.heading(y, "Error score distribution");
        y += 22.0;
        if scored.is_empty() {
            c.body(MARGIN, y + 10.0, "Not scored: this session ran without a reference profile.");
            y += 30.0;
        } else {
            let mut buckets = [0usize; 10];
            for cycle in &scored {
                let index = ((cycle.error_score * 10.0) as usize).min(9);
                buckets[index] += 1;
            }
            let labels = ["0.0", "0.1", "0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8", "0.9"];
            c.histogram(MARGIN, y, CONTENT, 84.0, &buckets, &labels);
            y += 94.0;
            y = c.paragraph(
                MARGIN,
                y,
                CONTENT,
                7.5,
                &format!(
                    "{} scored cycles. An error score is a weighted distance from this \
patient's own medians. Doc 06 defines no threshold separating a good cycle from a bad \
one, so none is drawn.",
                    scored.len()
                ),
            ) + 10.0;
        }

        c.heading(y, "Error classes");
        y += 20.0;
        let mut counts: Vec<(usize, usize)> = Vec::new();
        for cycle in &scored {
            if cycle.confidence < super::CONFIDENCE_FOR_DISPLAY {
                continue;
            }
            let index = cycle.primary_class as usize;
            match counts.iter_mut().find(|(class, _)| *class == index) {
                Some((_, count)) => *count += 1,
                None => counts.push((index, 1)),
            }
        }
        counts.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        if counts.is_empty() {
            c.paragraph(
                MARGIN,
                y + 8.0,
                CONTENT,
                7.5,
                "No classification reached the 0.50 confidence doc 06 §7 requires before one \
may be shown.",
            );
        } else {
            for (row, (class, count)) in counts.iter().enumerate() {
                let line = y + 8.0 + row as f32 * 13.0;
                c.body(MARGIN, line, &classes.get(*class).copied().unwrap_or("unknown").replace('_', " "));
                c.number(MARGIN + 220.0, line, 8.5, &count.to_string());
            }
        }

        c.footer(1, pages, &session.session_id);
    }

    // ---- page 2: the seven trend plots (doc 10 §8) -------------------------
    {
        let mut page = document.start_page_with(PageSettings::new(
            krilla::geom::Size::from_wh(WIDTH, HEIGHT).expect("A4"),
        ));
        let mut surface = page.surface();
        let mut c = Canvas { surface: &mut surface, fonts: &fonts };
        c.heading(MARGIN + 6.0, "Trends across the session");
        c.paragraph(
            MARGIN,
            MARGIN + 24.0,
            CONTENT,
            7.5,
            "One point per valid cycle, in order. The vertical range of each plot is printed \
at its right edge; a flat plot and an empty one are different things, and the second says \
so.",
        );

        let series: [(&str, Vec<f32>); 7] = [
            ("Cadence (steps/min)", valid.iter().map(|c| c.cadence_steps_per_min).collect()),
            (
                "Unilateral cycle symmetry proxy",
                valid.iter().map(|c| c.symmetry_proxy.unwrap_or(f32::NAN)).collect(),
            ),
            (
                "Error score",
                valid.iter().map(|c| if c.confidence > 0.0 { c.error_score } else { f32::NAN }).collect(),
            ),
            ("Stance ratio", valid.iter().map(|c| c.stance_ratio).collect()),
            ("Peak dorsiflexion (deg)", valid.iter().map(|c| c.peak_dorsiflexion_deg).collect()),
            // Doc 10 §8 asks for a haptic-response plot. There are no ERM
            // drivers (DEC-006), so the panel is drawn empty and labelled
            // rather than omitted: a reader must see that it was asked for and
            // that nothing could answer it.
            ("Haptic response — no drivers fitted (DEC-006)", Vec::new()),
            ("ZUPT quality", valid.iter().map(|c| c.zupt_quality).collect()),
        ];
        // Seven charts have to fit above the footer: 7 x (72 + 20) = 644 pt
        // below y = 100, which lands the last one at 744 on an 841.89 pt page.
        let chart_height = 72.0;
        for (index, (title, values)) in series.iter().enumerate() {
            let y = MARGIN + 52.0 + index as f32 * (chart_height + 20.0);
            c.chart(MARGIN, y, CONTENT, chart_height, title, values);
        }
        c.footer(2, pages, &session.session_id);
    }

    // ---- page 3: the timeline, the segments, and the data quality ----------
    {
        let mut page = document.start_page_with(PageSettings::new(
            krilla::geom::Size::from_wh(WIDTH, HEIGHT).expect("A4"),
        ));
        let mut surface = page.surface();
        let mut c = Canvas { surface: &mut surface, fonts: &fonts };

        c.heading(MARGIN + 6.0, "Event timeline");
        let mut y = MARGIN + 28.0;
        match (cycles.first(), cycles.last()) {
            (Some(first), Some(last)) => {
                let start = first.start_us;
                let end = last.start_us + (last.cycle_time_s as f64 * 1e6) as i64;
                let span = (end - start).max(1) as f32;
                c.filled(MARGIN, y, CONTENT, 40.0, wash());
                for cycle in cycles {
                    let x = MARGIN + CONTENT * (cycle.start_us - start) as f32 / span;
                    let w = CONTENT * (cycle.cycle_time_s * 1e6) / span;
                    // A classified cycle is filled dark; a valid clean one is a
                    // light bar; a rejected one is a hairline.
                    let shade = if !cycle.valid {
                        rule_grey()
                    } else if cycle.primary_class != 0
                        && cycle.confidence >= super::CONFIDENCE_FOR_DISPLAY
                    {
                        ink()
                    } else {
                        muted()
                    };
                    c.filled(x, y + 8.0, w.max(0.6), 24.0, shade);
                }
                y += 48.0;
                y = c.paragraph(
                    MARGIN,
                    y,
                    CONTENT,
                    7.5,
                    &format!(
                        "{:.1} s of walking, {} cycles. Dark bars carry a classification at or \
above 0.50 confidence; grey bars are valid and unclassified; hairlines are cycles the \
0.45-3.00 s temporal guards rejected, shown rather than hidden.",
                        span / 1e6,
                        cycles.len()
                    ),
                ) + 12.0;
            }
            _ => {
                c.body(MARGIN, y + 10.0, "No cycles were detected in this session.");
                y += 32.0;
            }
        }

        c.heading(y, "Segments");
        y += 22.0;
        if segments.is_empty() {
            c.body(MARGIN, y, "Not segmented: segment limits apply to evaluations (doc 12 §5).");
            y += 24.0;
        } else {
            c.label(MARGIN, y, "SEGMENT");
            c.label(MARGIN + 80.0, y, "VALID CYCLES");
            c.label(MARGIN + 180.0, y, "ERRORS");
            c.label(MARGIN + 260.0, y, "CLOSED BY");
            y += 13.0;
            for segment in segments {
                c.number(MARGIN, y, 8.5, &segment.segment_index.to_string());
                c.number(MARGIN + 80.0, y, 8.5, &segment.valid_cycles.to_string());
                c.number(MARGIN + 180.0, y, 8.5, &segment.errors.to_string());
                c.body(
                    MARGIN + 260.0,
                    y,
                    &segment.closed_by.clone().unwrap_or_else(|| "open".into()).replace('_', " "),
                );
                y += 13.0;
            }
            y += 10.0;
            y = c.paragraph(
                MARGIN,
                y,
                CONTENT,
                7.5,
                &format!(
                    "Limits entered for this session: {} valid cycles or {} errors, whichever \
came first.",
                    session.max_cycles_per_segment.map(|v| v.to_string()).unwrap_or_else(|| "—".into()),
                    session.max_errors_per_segment.map(|v| v.to_string()).unwrap_or_else(|| "—".into()),
                ),
            ) + 12.0;
        }

        c.heading(y, "Data quality");
        y += 22.0;
        let rows = vec![
            ("Frames stored", session.frames_stored.to_string()),
            ("Frames never arrived", session.frames_missing.to_string()),
            ("Cycles rejected by the guards", (cycles.len() - valid.len()).to_string()),
            (
                "Cycles without adequate ZUPT",
                (valid.len() - measured.len()).to_string(),
            ),
            ("Device state or fault changes", status.len().to_string()),
            (
                "Faults reported",
                status.iter().filter(|s| s.faults != 0).count().to_string(),
            ),
        ];
        y = c.pairs(MARGIN, y, CONTENT, &rows);
        y += 8.0;
        c.paragraph(
            MARGIN,
            y,
            CONTENT,
            7.5,
            "Haptics: no ERM drivers are fitted (DEC-006). The motor GPIOs are held low and no \
haptic command was issued, so there is no haptic response to summarise. On-device flash \
storage is not implemented in V1, so there is no storage recovery state to report.",
        );

        c.footer(3, pages, &session.session_id);
    }

    document.finish().map_err(|e| super::ExportError::Encode(format!("{e:?}")))
}
