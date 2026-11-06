//! Zeichenfläche: Ansicht, Auswahl, Werkzeuge, Vorschau, Lineale.

use eframe::egui::{self, Align2, Color32, FontId, PointerButton, Pos2, Rect, Sense, Shape, Stroke, Vec2};

use crate::app::{App, Drag, Need, Tool};
use crate::canvas::{self, View};
use crate::geom::*;
use crate::model::*;
use crate::{extract, ops, presets, render};

const SEL_COLOR: Color32 = Color32::from_rgb(0, 120, 215);

/// DIN 14090: Mindestbreite der Kurve in Abhängigkeit vom Außenradius (m).
pub fn din14090_width(r_out_m: f32) -> f32 {
    match r_out_m {
        r if r <= 12.0 => 5.0,
        r if r <= 15.0 => 4.5,
        r if r <= 20.0 => 4.0,
        r if r <= 40.0 => 3.5,
        r if r <= 70.0 => 3.2,
        _ => 3.0,
    }
}

fn nice_meters(target: f32) -> f32 {
    let p = 10f32.powf(target.max(0.1).log10().floor());
    for m in [1.0, 2.0, 5.0, 10.0] {
        if m * p >= target {
            return m * p;
        }
    }
    10.0 * p
}

impl App {
    fn layer_for(&mut self, kind: &ObjKind) -> u32 {
        let auto = self.settings.auto_layers;
        let doc = &mut self.tabs[self.active].doc;
        if !auto {
            return doc.active_layer;
        }
        let name = match kind {
            ObjKind::Wall { .. } | ObjKind::Door { .. } | ObjKind::Window { .. } | ObjKind::Stairs { .. } => "Wände",
            ObjKind::Area { .. } => "Flächen",
            ObjKind::Route { .. } => "Fluchtwege",
            ObjKind::Symbol { .. } => "Symbole",
            ObjKind::Text { .. } | ObjKind::Dimension { .. } | ObjKind::Note { .. } | ObjKind::Table { .. } | ObjKind::Room { .. } | ObjKind::Legend { .. } | ObjKind::Block { .. } => "Beschriftung",
            _ => return doc.active_layer,
        };
        doc.find_layer_by_name(name)
    }

    /// Objekt aus Klickpunkten für das aktuelle Werkzeug (auch für die Vorschau).
    pub fn make_obj(&self, pts: &[Pos]) -> Option<(ObjKind, Style)> {
        let s = &self.settings;
        let doc = self.doc();
        let black = Style::line(farbe::SCHWARZ, 0.35);
        let p0 = *pts.first()?;
        let p1 = pts.get(1).copied();
        let p2 = pts.get(2).copied();
        Some(match &self.tool {
            Tool::Line => (ObjKind::Line { a: p0, b: p1? }, black),
            Tool::Rect => (ObjKind::Rect { a: p0, b: p1?, radius: 0.0 }, black),
            Tool::Container => (ObjKind::Rect { a: p0, b: p1?, radius: 2.0 }, Style { stroke: Some(StrokeStyle::new(farbe::SCHWARZ, 0.4)), fill: Fill::Farbe(farbe::WEISS) }),
            Tool::Circle => (ObjKind::Circle { c: p0, r: p0.dist(p1?).max(0.2) }, black),
            Tool::Ellipse => {
                let b = BBox::from_two(p0, p1?);
                (ObjKind::Ellipse { c: b.center(), rx: (b.w() / 2.0).max(0.2), ry: (b.h() / 2.0).max(0.2) }, black)
            }
            Tool::NGon => (ObjKind::NGon { c: p0, r: p0.dist(p1?).max(0.2), n: s.ngon_n, rot: (p1? - p0).angle_deg() + 90.0 }, black),
            Tool::Arc => {
                let p1 = p1?;
                let r = p0.dist(p1).max(0.2);
                let start = (p1 - p0).angle_deg();
                let mut end = p2.map(|p| (p - p0).angle_deg()).unwrap_or(start + 90.0);
                if end <= start {
                    end += 360.0;
                }
                (ObjKind::Arc { c: p0, r, start, end }, black)
            }
            Tool::Dimension => (ObjKind::Dimension { a: p0, b: p1?, offset: 6.0 }, Style::line(farbe::SCHWARZ, 0.25)),
            Tool::Window => (ObjKind::Window { a: p0, b: p1?, thickness: s.wall_mm.max(1.0) }, Style::line(farbe::SCHWARZ, 0.25)),
            Tool::Door | Tool::DoubleDoor => {
                let p1 = p1?;
                (ObjKind::Door { pos: p0, width: p0.dist(p1).max(2.0), rot: (p1 - p0).angle_deg(), mirror: false, double: self.tool == Tool::DoubleDoor }, Style::line(farbe::SCHWARZ, 0.35))
            }
            Tool::Stairs => {
                let p1 = p1?;
                let steps = ((doc.real_m(p0.dist(p1)) / 0.28).round() as u32).clamp(3, 40);
                (ObjKind::Stairs { a: p0, b: p1, width: doc.sheet_mm(1.2).clamp(4.0, 40.0), steps }, Style::line(farbe::SCHWARZ, 0.3))
            }
            Tool::Grid => {
                let b = BBox::from_two(p0, p1?);
                let nx = ((b.w() / 45.0).round() as u32).max(1);
                let ny = ((b.h() / 45.0).round() as u32).max(1);
                (ObjKind::Grid { a: b.min, b: b.max, nx, ny }, Style::line(Color::rgb(0x60, 0x60, 0x60), 0.2))
            }
            Tool::FloorSection => {
                let b = BBox::from_two(p0, p1?);
                let floors: Vec<String> = ["2. OG", "1. OG", "EG", "KG"].map(String::from).to_vec();
                let g = doc.info.geschoss.trim().to_lowercase();
                let current = floors
                    .iter()
                    .position(|f| f.to_lowercase() == g || (g.starts_with("erd") && f == "EG") || (g.starts_with("keller") && f == "KG"))
                    .unwrap_or(2);
                (ObjKind::FloorSection { a: b.min, b: b.max, floors, current }, Style::default())
            }
            Tool::Block(art) => {
                let b = BBox::from_two(p0, p1?);
                let fill = if *art == BlockArt::Notruf { Fill::Keine } else { Fill::Farbe(farbe::WEISS) };
                (ObjKind::Block { a: b.min, b: b.max, art: *art }, Style { stroke: (*art != BlockArt::Notruf).then(|| StrokeStyle::new(farbe::SCHWARZ, 0.4)), fill })
            }
            Tool::Legend => {
                let b = BBox::from_two(p0, p1?);
                (ObjKind::Legend { a: b.min, b: b.max, cols: if b.w() > b.h() * 2.0 { 3 } else { 1 }, size: 3.2 }, Style { stroke: Some(StrokeStyle::new(farbe::SCHWARZ, 0.3)), fill: Fill::Farbe(farbe::WEISS) })
            }
            Tool::FireCurve => {
                let p1 = p1?;
                let mut r_out = p0.dist(p1);
                if doc.real_m(r_out) < 10.5 {
                    r_out = doc.sheet_mm(10.5);
                }
                let width = doc.sheet_mm(din14090_width(doc.real_m(r_out)));
                let start = (p1 - p0).angle_deg();
                let mut end = p2.map(|p| (p - p0).angle_deg()).unwrap_or(start + 90.0);
                if end <= start {
                    end += 360.0;
                }
                (ObjKind::FireCurve { c: p0, r_out, width, start, end }, Style::default())
            }
            Tool::Polyline => (ObjKind::Polyline { pts: pts.to_vec(), closed: false }, black),
            Tool::Polygon => (ObjKind::Polyline { pts: pts.to_vec(), closed: true }, black),
            Tool::Hose => (ObjKind::Hose { pts: pts.to_vec(), width: 1.5 }, Style::line(farbe::SCHWARZ, 0.25)),
            Tool::Spline => (ObjKind::Spline { pts: pts.to_vec() }, black),
            Tool::Wall | Tool::WallOrtho => (ObjKind::Wall { pts: pts.to_vec(), thickness: s.wall_mm, closed: false }, Style::line(farbe::SCHWARZ, s.wall_mm)),
            Tool::Room => (ObjKind::Room { pts: pts.to_vec(), name: "Raum".into(), show_area: false, size: 3.0 }, Style::NONE),
            Tool::Area(k) => (ObjKind::Area { pts: pts.to_vec(), preset: k.clone() }, presets::find(k).map(|p| p.style).unwrap_or_default()),
            Tool::Route | Tool::RouteCurved => (
                ObjKind::Route { pts: pts.to_vec(), width: s.route_mm, curved: self.tool == Tool::RouteCurved, arrows: true },
                Style::line(farbe::FLUCHT_HELL, s.route_mm),
            ),
            Tool::Laufweg => (ObjKind::Route { pts: pts.to_vec(), width: 2.5, curved: false, arrows: true }, Style::line(farbe::ROT, 2.5)),
            Tool::Cloud => (ObjKind::RevisionCloud { pts: pts.to_vec() }, Style::line(farbe::ROT, 0.35)),
            Tool::Fence => (ObjKind::Fence { pts: pts.to_vec() }, Style::line(farbe::SCHWARZ, 0.25)),
            Tool::Rail => (ObjKind::Rail { pts: pts.to_vec() }, Style::line(farbe::SCHWARZ, 0.25)),
            Tool::Text => (ObjKind::Text { pos: p0, text: "Text".into(), size: s.text_mm, bold: false, italic: false, align: TAlign::Links, rot: 0.0 }, Style::line(farbe::SCHWARZ, 0.2)),
            Tool::Symbol(id) => {
                let size = if id.starts_with("N") && id.len() <= 2 { s.symbol_mm * 2.0 } else { s.symbol_mm };
                (ObjKind::Symbol { id: id.clone(), pos: p0, size, rot: 0.0 }, Style::NONE)
            }
            Tool::Note => (ObjKind::Note { pos: p0, text: "Notiz".into() }, Style::NONE),
            Tool::ScaleBar => (ObjKind::ScaleBar { pos: p0, meters: nice_meters(doc.real_m(50.0)) }, Style::default()),
            Tool::Table => {
                let (r, c) = (s.table_rows.max(1), s.table_cols.max(1));
                let mut cells = vec![vec![String::new(); c]; r];
                for (i, cell) in cells[0].iter_mut().enumerate() {
                    *cell = format!("Spalte {}", i + 1);
                }
                (ObjKind::Table { pos: p0, cols: vec![30.0; c], rows: vec![7.0; r], cells, size: 3.0 }, Style { stroke: Some(StrokeStyle::new(farbe::SCHWARZ, 0.3)), fill: Fill::Farbe(farbe::WEISS) })
            }
            _ => return None,
        })
    }

    pub fn finish_tool(&mut self, pts: Vec<Pos>) {
        let tool = self.tool.clone();
        match &tool {
            Tool::ZoomBox => {
                let b = BBox::from_two(pts[0], pts[1]);
                self.zoom_to(b);
            }
            Tool::MeasureM | Tool::MeasureMm => {
                let d = pts[0].dist(pts[1]);
                let txt = if tool == Tool::MeasureM { format!("Distanz: {:.2} m (Maßstab 1:{})", self.doc().real_m(d), self.doc().scale_n) } else { format!("Distanz auf dem Blatt: {d:.1} mm") };
                self.status = txt.clone();
                self.measure = Some(txt);
            }
            Tool::MeasureAngle => {
                let a = (pts[0] - pts[1]).angle_deg();
                let b = (pts[2] - pts[1]).angle_deg();
                let mut w = (b - a).abs() % 360.0;
                if w > 180.0 {
                    w = 360.0 - w;
                }
                let txt = format!("Winkel: {w:.1}°");
                self.status = txt.clone();
                self.measure = Some(txt);
            }
            Tool::ScaleByDistance => {
                self.dlg.scale_dist = Some(pts[0].dist(pts[1]));
            }
            Tool::Qr => {
                self.dlg.qr = Some(pts[0]);
                self.dlg.qr_text = "https://".into();
            }
            Tool::Guide => {}
            Tool::CutPlan => {
                let b = BBox::from_two(pts[0], pts[1]);
                let doc_scale = self.doc().clone();
                let tab = self.tab_mut();
                tab.push_undo();
                let before = tab.doc.objects.len();
                tab.doc.objects.retain(|o| {
                    let ob = ops::bounds_doc(&doc_scale, &o.kind);
                    !(ob.max.x < b.min.x || ob.min.x > b.max.x || ob.max.y < b.min.y || ob.min.y > b.max.y)
                });
                let n = before - tab.doc.objects.len();
                tab.sel.clear();
                self.status = format!("{n} Element(e) außerhalb des Bereichs entfernt.");
            }
            Tool::RoomArea | Tool::InsertPoint | Tool::SegmentCut | Tool::Fillet => {
                let p = pts[0];
                let tol = 6.0 / self.tab().zoom;
                let hit = self.hit_test(p, tol);
                let Some(id) = hit else {
                    self.status = "Kein passendes Element an dieser Stelle.".into();
                    return;
                };
                let fillet_r = self.settings.fillet_mm;
                let tab = self.tab_mut();
                let Some(i) = tab.doc.index_of(id) else { return };
                let before = tab.doc.clone();
                let ok = match (&tool, &mut tab.doc.objects[i].kind) {
                    (Tool::RoomArea, ObjKind::Room { show_area, .. }) => {
                        *show_area = !*show_area;
                        true
                    }
                    (Tool::InsertPoint, k) => ops::insert_point(k, p),
                    (Tool::SegmentCut, ObjKind::Wall { .. } | ObjKind::Polyline { .. }) => {
                        let o = tab.doc.objects[i].clone();
                        if let Some(parts) = extract::cut_segment(&o.kind, p) {
                            tab.doc.objects.remove(i);
                            for k in parts {
                                let id = tab.doc.alloc_id();
                                tab.doc.objects.insert(i, Obj { id, kind: k, ..o.clone() });
                            }
                            true
                        } else {
                            false
                        }
                    }
                    (Tool::Fillet, k) => extract::fillet(k, p, fillet_r, tol * 2.0),
                    _ => false,
                };
                if ok {
                    tab.undo.push(before);
                    tab.redo.clear();
                    tab.dirty = true;
                } else {
                    self.status = "Für dieses Element nicht möglich.".into();
                }
            }
            Tool::FillPolygon(preset) => match extract::fill_region(self.doc(), pts[0]) {
                Some(poly) => {
                    let st = presets::find(preset).map(|p| p.style).unwrap_or_default();
                    let kind = ObjKind::Area { pts: poly, preset: preset.clone() };
                    let layer = self.layer_for(&kind);
                    let tab = self.tab_mut();
                    tab.push_undo();
                    let id = tab.doc.add(kind, st, layer);
                    // Flächen unter die Wände legen
                    let o = tab.doc.objects.pop().unwrap();
                    let first_wall = tab.doc.objects.iter().position(|o| matches!(o.kind, ObjKind::Wall { .. })).unwrap_or(tab.doc.objects.len());
                    tab.doc.objects.insert(first_wall, o);
                    tab.sel = vec![id];
                    ops::sync_legend(&mut tab.doc);
                }
                None => self.status = "Keine geschlossene Fläche gefunden (Wände/Linien müssen den Bereich umschließen).".into(),
            },
            _ => {
                let Some((kind, style)) = self.make_obj(&pts) else { return };
                if let ObjKind::FireCurve { r_out, .. } = &kind {
                    let r = self.doc().real_m(*r_out);
                    self.status = format!("Kurve nach DIN 14090: Außenradius {r:.1} m, Breite {:.1} m.", din14090_width(r));
                }
                let layer = self.layer_for(&kind);
                let is_text = matches!(kind, ObjKind::Text { .. } | ObjKind::Note { .. });
                let is_area = matches!(kind, ObjKind::Area { .. });
                let tab = self.tab_mut();
                tab.push_undo();
                let id = tab.doc.add(kind, style, layer);
                if is_area {
                    let o = tab.doc.objects.pop().unwrap();
                    let first_wall = tab.doc.objects.iter().position(|o| matches!(o.kind, ObjKind::Wall { .. } | ObjKind::Door { .. })).unwrap_or(tab.doc.objects.len());
                    tab.doc.objects.insert(first_wall, o);
                }
                tab.sel = vec![id];
                ops::sync_legend(&mut tab.doc);
                if is_text {
                    self.focus_text = true;
                    self.tool = Tool::Select;
                }
            }
        }
    }

    pub fn hit_test(&self, p: Pos, tol: f32) -> Option<u64> {
        let doc = self.doc();
        let sichtbar = || doc.objects.iter().rev().filter(|o| doc.layer_visible(o.layer) && !doc.layer_locked(o.layer));
        let massstab = |o: &Obj| matches!(o.kind, ObjKind::ScaleBar { .. }) && ops::bounds_doc(doc, &o.kind).expand(tol).contains(p);
        // erst Linien, Symbole, Texte – dann Innenflächen
        sichtbar()
            .find(|o| massstab(o) || ops::hit_obj_umriss(o, p, tol))
            .or_else(|| sichtbar().find(|o| ops::hit_obj(o, p, tol)))
            .map(|o| o.id)
    }

    /// Liegt `p` auf einem Bibliothekssymbol oder einem Griffpunkt der Einzelauswahl?
    fn over_symbol(&self, p: Pos, zoom: f32) -> bool {
        let tab = self.tab();
        if tab.sel.len() == 1 {
            if let Some(o) = tab.doc.objects.iter().find(|o| o.id == tab.sel[0]) {
                if !o.locked && ops::handles(&o.kind).iter().any(|h| h.dist(p) * zoom < 7.0) {
                    return true;
                }
            }
        }
        self.hit_test(p, 5.0 / zoom)
            .and_then(|id| tab.doc.objects.iter().find(|o| o.id == id))
            .is_some_and(|o| matches!(o.kind, ObjKind::Symbol { .. }))
    }

    pub fn zoom_to(&mut self, b: BBox) {
        let tab = self.tab_mut();
        if b.w() < 1.0 || b.h() < 1.0 {
            return;
        }
        tab.fit = false;
        // Größe der Zeichenfläche ist erst im nächsten Frame bekannt -> merken
        tab.offset = Vec2::new(-b.min.x, -b.min.y);
        tab.zoom = -b.w().max(b.h()); // Signal: im Canvas auf Bereich zoomen
    }

    fn snap(&self, p: Pos, zoom: f32, last: Option<Pos>, shift: bool, ortho_axis: bool) -> Pos {
        let mut q = p;
        if let Some(l) = last {
            if ortho_axis {
                let d = q - l;
                q = if d.x.abs() > d.y.abs() { Pos::new(q.x, l.y) } else { Pos::new(l.x, q.y) };
                return q;
            }
            if shift {
                return snap_ortho(l, q);
            }
        }
        if self.settings.point_snap {
            let tol = 7.0 / zoom;
            let doc = self.doc();
            let mut best = (tol, None);
            for o in &doc.objects {
                if !doc.layer_visible(o.layer) || matches!(o.kind, ObjKind::Image { .. } | ObjKind::Group { .. }) {
                    continue;
                }
                for h in ops::handles(&o.kind) {
                    let d = h.dist(p);
                    if d < best.0 {
                        best = (d, Some(h));
                    }
                }
            }
            for d in &self.draft {
                if d.dist(p) < best.0 {
                    best = (d.dist(p), Some(*d));
                }
            }
            if let Some(h) = best.1 {
                return h;
            }
        }
        if self.settings.grid_snap && self.settings.grid_mm > 0.1 {
            let g = self.settings.grid_mm;
            q = Pos::new((q.x / g).round() * g, (q.y / g).round() * g);
        }
        q
    }

    pub fn canvas_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = resp.rect;
        let bg = self.settings.bg;
        painter.rect_filled(rect, 0.0, Color32::from_rgb(bg[0], bg[1], bg[2]));
        let (pw, ph) = self.doc().page.size();
        let ruler = if self.settings.rulers { 18.0 } else { 0.0 };

        // Ansicht
        {
            let tab = &mut self.tabs[self.active];
            if tab.fit {
                tab.zoom = ((rect.width() - 40.0 - ruler) / pw).min((rect.height() - 40.0 - ruler) / ph).max(0.05);
                tab.offset = Vec2::new((rect.width() + ruler - pw * tab.zoom) / 2.0, (rect.height() + ruler - ph * tab.zoom) / 2.0);
                tab.fit = false;
            } else if tab.zoom < 0.0 {
                // Bereich zoomen (siehe zoom_to)
                let size = -tab.zoom;
                let min = Pos::new(-tab.offset.x, -tab.offset.y);
                tab.zoom = ((rect.width() - 40.0) / size).min((rect.height() - 40.0) / size).clamp(0.05, 200.0);
                tab.offset = Vec2::new(20.0 - min.x * tab.zoom, 20.0 - min.y * tab.zoom);
            }
            if let Some(m) = resp.hover_pos() {
                // Mausrad: zoomen; Strg+Rad: senkrecht, Umschalt+Rad: waagerecht verschieben;
                // waagerechtes Rad/Touchpad verschiebt seitlich, Touchpad-Spreizen zoomt.
                let (scroll, pan, pinch) = ui.input(|i| {
                    let (mut scroll, mut pan, mut pinch) = (0.0f32, Vec2::ZERO, 1.0f32);
                    for e in &i.events {
                        match e {
                            egui::Event::MouseWheel { unit, delta, modifiers, .. } => {
                                let d = *delta
                                    * match unit {
                                        egui::MouseWheelUnit::Point => 1.0,
                                        egui::MouseWheelUnit::Line => 50.0,
                                        egui::MouseWheelUnit::Page => rect.height(),
                                    };
                                if modifiers.command {
                                    pan += d;
                                } else if modifiers.shift {
                                    pan.x += if d.x != 0.0 { d.x } else { d.y };
                                } else {
                                    scroll += d.y;
                                    pan.x += d.x;
                                }
                            }
                            egui::Event::Zoom(f) => pinch *= f,
                            _ => {}
                        }
                    }
                    (scroll, pan, pinch)
                });
                tab.offset += pan;
                let f = (scroll * 0.0025).exp() * pinch;
                if (f - 1.0).abs() > 1e-4 {
                    let before = (m - rect.min - tab.offset) / tab.zoom;
                    tab.zoom = (tab.zoom * f).clamp(0.05, 200.0);
                    tab.offset = m - rect.min - before * tab.zoom;
                }
            }
        }
        let space = ui.input(|i| i.key_down(egui::Key::Space));
        let panning = resp.dragged_by(PointerButton::Middle) || (resp.dragged_by(PointerButton::Primary) && (self.tool == Tool::Pan || space));
        if panning {
            self.tabs[self.active].offset += resp.drag_delta();
        }

        let tab = &self.tabs[self.active];
        let view = View { origin: rect.min + tab.offset, zoom: tab.zoom };
        let zoom = tab.zoom;
        let mouse = resp.hover_pos().map(|p| view.to_mm(p));
        self.mouse_mm = mouse;
        let (shift, cmd) = ui.input(|i| (i.modifiers.shift, i.modifiers.command));
        let ortho_axis = self.tool == Tool::WallOrtho;
        let snapped = mouse.map(|m| self.snap(m, zoom, self.draft.last().copied(), shift, ortho_axis));

        // ---------- Interaktion ----------
        // Mit aktivem Symbol-Werkzeug: über einem Symbol oder Griffpunkt verschieben/skalieren statt neu platzieren
        let symbol_edit = matches!(self.tool, Tool::Symbol(_))
            && self.draft.is_empty()
            && (self.drag.is_some() || mouse.is_some_and(|m| self.over_symbol(m, zoom)));
        if !panning {
            if self.tool == Tool::Select || symbol_edit {
                self.select_interaction(&resp, &view, shift || cmd);
            } else if self.tool != Tool::Pan {
                let need = self.tool.need();
                if resp.clicked_by(PointerButton::Primary) || resp.double_clicked() {
                    if let Some(p) = snapped {
                        if self.tool == Tool::Guide {
                            let tab = self.tab_mut();
                            tab.push_undo();
                            tab.doc.guides.push(Guide { vertical: shift, at: if shift { p.x } else { p.y } });
                        } else {
                            let dbl = resp.double_clicked();
                            if !(dbl && need == Need::Multi && self.draft.last().is_some_and(|l| l.dist(p) < 0.5)) {
                                self.draft.push(p);
                            }
                            let n = self.draft.len();
                            let done = match need {
                                Need::One => n >= 1,
                                Need::Two => n >= 2,
                                Need::Three => n >= 3,
                                Need::Multi => dbl && n >= 2,
                            };
                            if done {
                                let mut pts = std::mem::take(&mut self.draft);
                                pts.dedup_by(|a, b| a.dist(*b) < 0.3);
                                let min_pts = match need {
                                    Need::Multi => match self.tool {
                                        Tool::Polygon | Tool::Room | Tool::Area(_) | Tool::Cloud => 3,
                                        _ => 2,
                                    },
                                    Need::Two => 2,
                                    Need::Three => 3,
                                    Need::One => 1,
                                };
                                if pts.len() >= min_pts || need == Need::One {
                                    if need == Need::Two && pts.len() < 2 {
                                        // gleicher Punkt zweimal
                                    } else {
                                        self.finish_tool(pts);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if resp.secondary_clicked() && self.tool == Tool::Select {
                if let Some(m) = mouse {
                    if let Some(id) = self.hit_test(m, 6.0 / zoom) {
                        if !self.tab().sel.contains(&id) {
                            self.tab_mut().sel = vec![id];
                        }
                    }
                }
            }
        }
        if resp.secondary_clicked() && self.tool != Tool::Select {
            // Rechtsklick beendet Linienzüge bzw. das Werkzeug
            if self.tool.need() == Need::Multi && self.draft.len() >= 2 {
                let pts = std::mem::take(&mut self.draft);
                self.finish_tool(pts);
            } else {
                self.reset_tool();
            }
        }
        resp.context_menu(|ui| self.context_menu(ui));

        // ---------- Zeichnen ----------
        let painter = painter.with_clip_rect(rect);
        let tab = &self.tabs[self.active];
        let view = View { origin: rect.min + tab.offset, zoom: tab.zoom };
        let sheet = Rect::from_min_max(view.to_screen(Pos::new(0.0, 0.0)), view.to_screen(Pos::new(pw, ph)));
        painter.rect_filled(sheet.translate(Vec2::splat(4.0)), 0.0, Color32::from_black_alpha(70));
        painter.rect_filled(sheet, 0.0, Color32::WHITE);
        if self.settings.grid_show {
            let g = self.settings.grid_mm;
            if g * zoom > 6.0 {
                let col = Color32::from_rgb(0xd8, 0xe4, 0xf0);
                let mut x = 0.0;
                while x <= pw {
                    let sx = view.to_screen(Pos::new(x, 0.0)).x;
                    painter.line_segment([Pos2::new(sx, sheet.top()), Pos2::new(sx, sheet.bottom())], Stroke::new(0.5, col));
                    x += g;
                }
                let mut y = 0.0;
                while y <= ph {
                    let sy = view.to_screen(Pos::new(0.0, y)).y;
                    painter.line_segment([Pos2::new(sheet.left(), sy), Pos2::new(sheet.right(), sy)], Stroke::new(0.5, col));
                    y += g;
                }
            }
        }
        let doc = &tab.doc;
        let prims = render::page_prims(doc, false);
        canvas::paint_prims(&painter, &ctx, &mut self.textures, doc, &prims, view);

        if self.settings.guides {
            for g in &doc.guides {
                let (a, b) = if g.vertical { (Pos::new(g.at, -1000.0), Pos::new(g.at, ph + 1000.0)) } else { (Pos::new(-1000.0, g.at), Pos::new(pw + 1000.0, g.at)) };
                painter.add(Shape::dashed_line(&[view.to_screen(a), view.to_screen(b)], Stroke::new(1.0, Color32::from_rgb(0, 190, 220)), 6.0, 4.0));
            }
        }

        // Auswahl
        let sel_objs: Vec<&Obj> = doc.objects.iter().filter(|o| tab.sel.contains(&o.id)).collect();
        for o in &sel_objs {
            let b = ops::bounds_doc(doc, &o.kind);
            let r = Rect::from_min_max(view.to_screen(b.min), view.to_screen(b.max)).expand(3.0);
            painter.add(Shape::dashed_line(&[r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()], Stroke::new(1.2, SEL_COLOR), 5.0, 3.0));
        }
        if sel_objs.len() == 1 {
            for h in ops::handles(&sel_objs[0].kind) {
                let s = view.to_screen(h);
                painter.rect(Rect::from_center_size(s, Vec2::splat(7.0)), 0.0, Color32::WHITE, Stroke::new(1.2, SEL_COLOR), egui::StrokeKind::Middle);
            }
        }
        if let Some(Drag::Box { start }) = &self.drag {
            if let Some(m) = mouse {
                let r = Rect::from_two_pos(view.to_screen(*start), view.to_screen(m));
                let crossing = m.x < start.x;
                painter.rect(r, 0.0, Color32::from_rgba_unmultiplied(0, 120, 215, 25), Stroke::new(1.0, SEL_COLOR), egui::StrokeKind::Middle);
                if crossing {
                    painter.text(r.left_top() + Vec2::new(4.0, 4.0), Align2::LEFT_TOP, "schneidend", FontId::proportional(11.0), SEL_COLOR);
                }
            }
        }

        // Vorschau des Werkzeugs
        if !self.draft.is_empty() {
            if let Some(m) = snapped {
                let mut pts = self.draft.clone();
                pts.push(m);
                let preview = self.make_obj(&pts);
                if let Some((kind, style)) = preview {
                    let o = Obj { id: u64::MAX - 1, layer: 0, kind, style, print: true, locked: false };
                    let mut pp = Vec::new();
                    let doc = &self.tabs[self.active].doc;
                    render::obj_prims(&mut pp, doc, &o, false);
                    canvas::paint_prims(&painter, &ctx, &mut self.textures, doc, &pp, view);
                }
                let sp: Vec<Pos2> = pts.iter().map(|p| view.to_screen(*p)).collect();
                painter.add(Shape::dashed_line(&sp, Stroke::new(1.0, SEL_COLOR), 4.0, 3.0));
                for p in &self.draft {
                    painter.circle_stroke(view.to_screen(*p), 3.0, Stroke::new(1.0, SEL_COLOR));
                }
                // Länge des aktuellen Segments
                let last = *self.draft.last().unwrap();
                let d = last.dist(m);
                let label = format!("{:.2} m  ({d:.1} mm)", self.doc().real_m(d));
                painter.text(view.to_screen(m) + Vec2::new(12.0, 12.0), Align2::LEFT_TOP, label, FontId::proportional(12.0), Color32::from_rgb(0, 60, 140));
            }
        }
        // Fangpunkt anzeigen
        if let (Some(m), Some(s)) = (mouse, snapped) {
            if self.tool != Tool::Select && self.tool != Tool::Pan && m.dist(s) > 0.01 {
                painter.rect_stroke(Rect::from_center_size(view.to_screen(s), Vec2::splat(9.0)), 0.0, Stroke::new(1.5, Color32::from_rgb(230, 120, 0)), egui::StrokeKind::Middle);
            }
        }
        if let Some(t) = &self.measure {
            painter.text(rect.left_top() + Vec2::new(ruler + 10.0, ruler + 10.0), Align2::LEFT_TOP, t, FontId::proportional(14.0), Color32::from_rgb(0, 60, 140));
        }

        if self.settings.rulers {
            draw_rulers(&painter, rect, view, mouse);
        }
        if self.tool == Tool::Pan || space {
            ctx.set_cursor_icon(egui::CursorIcon::Grab);
        } else if self.tool != Tool::Select && resp.hovered() {
            ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
        }
    }

    fn select_interaction(&mut self, resp: &egui::Response, view: &View, additive: bool) {
        let zoom = view.zoom;
        let pointer = resp.interact_pointer_pos().map(|p| view.to_mm(p));
        if resp.drag_started_by(PointerButton::Primary) {
            if let Some(p) = pointer {
                // Griffpunkt der Einzelauswahl?
                let tab = &self.tabs[self.active];
                let mut handle = None;
                if tab.sel.len() == 1 {
                    if let Some(o) = tab.doc.objects.iter().find(|o| o.id == tab.sel[0]) {
                        if !o.locked {
                            for (i, h) in ops::handles(&o.kind).iter().enumerate() {
                                if h.dist(p) * zoom < 7.0 {
                                    handle = Some((o.id, i));
                                }
                            }
                        }
                    }
                }
                // Alt: immer Auswahlrahmen (auch wenn unter dem Mauszeiger ein Objekt liegt)
                let alt = resp.ctx.input(|i| i.modifiers.alt);
                if let Some((id, idx)) = handle {
                    self.tab_mut().push_undo();
                    self.drag = Some(Drag::Handle { id, idx });
                } else if let Some(id) = self.hit_test(p, 5.0 / zoom).filter(|_| !alt) {
                    let tab = self.tab_mut();
                    if !tab.sel.contains(&id) {
                        if additive {
                            tab.sel.push(id);
                        } else {
                            tab.sel = vec![id];
                        }
                    }
                    tab.push_undo();
                    self.drag = Some(Drag::Move { last: p });
                } else {
                    self.drag = Some(Drag::Box { start: p });
                }
            }
        }
        if resp.dragged_by(PointerButton::Primary) {
            if let Some(p) = pointer {
                match self.drag {
                    Some(Drag::Move { last }) => {
                        let d = p - last;
                        let tab = self.tab_mut();
                        let sel = tab.sel.clone();
                        for o in tab.doc.objects.iter_mut().filter(|o| sel.contains(&o.id) && !o.locked) {
                            ops::translate(&mut o.kind, d);
                        }
                        self.drag = Some(Drag::Move { last: p });
                    }
                    Some(Drag::Handle { id, idx }) => {
                        let q = self.snap(p, zoom, None, false, false);
                        let tab = self.tab_mut();
                        if let Some(i) = tab.doc.index_of(id) {
                            ops::set_handle(&mut tab.doc.objects[i].kind, idx, q);
                        }
                    }
                    _ => {}
                }
            }
        }
        if resp.drag_stopped() {
            if let (Some(Drag::Box { start }), Some(m)) = (&self.drag, self.mouse_mm) {
                let b = BBox::from_two(*start, m);
                let crossing = m.x < start.x;
                let tab = &mut self.tabs[self.active];
                let doc = &tab.doc;
                let found: Vec<u64> = doc
                    .objects
                    .iter()
                    .filter(|o| doc.layer_visible(o.layer) && !doc.layer_locked(o.layer))
                    .filter(|o| {
                        let ob = ops::bounds_doc(doc, &o.kind);
                        if crossing { !(ob.max.x < b.min.x || ob.min.x > b.max.x || ob.max.y < b.min.y || ob.min.y > b.max.y) } else { ob.inside(&b) }
                    })
                    .map(|o| o.id)
                    .collect();
                if additive {
                    for id in found {
                        if !tab.sel.contains(&id) {
                            tab.sel.push(id);
                        }
                    }
                } else {
                    tab.sel = found;
                }
            }
            if matches!(self.drag, Some(Drag::Move { .. }) | Some(Drag::Handle { .. })) {
                ops::sync_legend(&mut self.tab_mut().doc);
            }
            self.drag = None;
        }
        if resp.clicked_by(PointerButton::Primary) {
            if let Some(p) = pointer {
                let hit = self.hit_test(p, 5.0 / zoom);
                let tab = self.tab_mut();
                match (hit, additive) {
                    (Some(id), true) => {
                        if let Some(i) = tab.sel.iter().position(|s| *s == id) {
                            tab.sel.remove(i);
                        } else {
                            tab.sel.push(id);
                        }
                    }
                    (Some(id), false) => tab.sel = vec![id],
                    (None, false) => tab.sel.clear(),
                    _ => {}
                }
            }
        }
        if resp.double_clicked() {
            let tab = self.tab();
            if tab.sel.len() == 1 {
                if let Some(o) = tab.doc.objects.iter().find(|o| o.id == tab.sel[0]) {
                    if matches!(o.kind, ObjKind::Text { .. } | ObjKind::Note { .. } | ObjKind::Room { .. } | ObjKind::QrCode { .. }) {
                        self.focus_text = true;
                    }
                }
            }
        }
    }

    fn context_menu(&mut self, ui: &mut egui::Ui) {
        let has_sel = !self.tab().sel.is_empty();
        if ui.add_enabled(has_sel, egui::Button::new("Ausschneiden  (Strg+X)")).clicked() {
            self.cut();
            ui.close();
        }
        if ui.add_enabled(has_sel, egui::Button::new("Kopieren  (Strg+C)")).clicked() {
            self.copy();
            ui.close();
        }
        if ui.add_enabled(!self.clipboard.is_empty(), egui::Button::new("Einfügen  (Strg+V)")).clicked() {
            self.paste(Pos::new(5.0, 5.0));
            ui.close();
        }
        if ui.add_enabled(has_sel, egui::Button::new("Duplizieren  (Strg+D)")).clicked() {
            self.duplicate();
            ui.close();
        }
        if ui.add_enabled(has_sel, egui::Button::new("Löschen  (Entf)")).clicked() {
            self.delete_selection();
            ui.close();
        }
        ui.separator();
        if ui.add_enabled(has_sel, egui::Button::new("Drehen 90°  (R)")).clicked() {
            self.transform_sel(|k, c| ops::rotate(k, c, 90.0));
            ui.close();
        }
        if ui.add_enabled(has_sel, egui::Button::new("Horizontal spiegeln")).clicked() {
            self.transform_sel(|k, c| ops::mirror(k, c, true));
            ui.close();
        }
        if ui.add_enabled(has_sel, egui::Button::new("Gruppieren  (Strg+G)")).clicked() {
            self.group();
            ui.close();
        }
        if ui.add_enabled(has_sel, egui::Button::new("Gruppierung aufheben")).clicked() {
            self.ungroup();
            ui.close();
        }
        ui.separator();
        if ui.add_enabled(has_sel, egui::Button::new("Ganz nach vorne")).clicked() {
            self.order(true, false);
            ui.close();
        }
        if ui.add_enabled(has_sel, egui::Button::new("Ganz nach hinten")).clicked() {
            self.order(false, false);
            ui.close();
        }
        if ui.add_enabled(has_sel, egui::Button::new("Gleiche auswählen")).clicked() {
            self.select_similar();
            ui.close();
        }
    }
}

fn draw_rulers(painter: &egui::Painter, rect: Rect, view: View, mouse: Option<Pos>) {
    let t = 18.0;
    let bg = Color32::from_rgb(0xf3, 0xf3, 0xf3);
    let fg = Color32::from_rgb(0x55, 0x55, 0x55);
    let top = Rect::from_min_max(rect.min, Pos2::new(rect.max.x, rect.min.y + t));
    let left = Rect::from_min_max(rect.min, Pos2::new(rect.min.x + t, rect.max.y));
    painter.rect_filled(top, 0.0, bg);
    painter.rect_filled(left, 0.0, bg);
    let z = view.zoom;
    let step = [1.0f32, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0].into_iter().find(|s| s * z >= 6.0).unwrap_or(500.0);
    let label_every = [10.0f32, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0].into_iter().find(|s| s * z >= 45.0).unwrap_or(2000.0);
    let x0 = view.to_mm(Pos2::new(rect.min.x + t, 0.0)).x;
    let x1 = view.to_mm(Pos2::new(rect.max.x, 0.0)).x;
    let mut x = (x0 / step).floor() * step;
    while x <= x1 {
        let sx = view.to_screen(Pos::new(x, 0.0)).x;
        let major = (x / label_every).round() * label_every == x;
        let h = if major { t * 0.7 } else { t * 0.3 };
        painter.line_segment([Pos2::new(sx, top.max.y - h), Pos2::new(sx, top.max.y)], Stroke::new(1.0, fg));
        if major {
            painter.text(Pos2::new(sx + 2.0, top.min.y + 1.0), Align2::LEFT_TOP, format!("{x:.0}"), FontId::proportional(9.0), fg);
        }
        x += step;
    }
    let y0 = view.to_mm(Pos2::new(0.0, rect.min.y + t)).y;
    let y1 = view.to_mm(Pos2::new(0.0, rect.max.y)).y;
    let mut y = (y0 / step).floor() * step;
    while y <= y1 {
        let sy = view.to_screen(Pos::new(0.0, y)).y;
        let major = (y / label_every).round() * label_every == y;
        let w = if major { t * 0.7 } else { t * 0.3 };
        painter.line_segment([Pos2::new(left.max.x - w, sy), Pos2::new(left.max.x, sy)], Stroke::new(1.0, fg));
        if major {
            painter.text(Pos2::new(left.min.x + 1.0, sy + 2.0), Align2::LEFT_TOP, format!("{y:.0}"), FontId::proportional(9.0), fg);
        }
        y += step;
    }
    if let Some(m) = mouse {
        let s = view.to_screen(m);
        painter.line_segment([Pos2::new(s.x, top.min.y), Pos2::new(s.x, top.max.y)], Stroke::new(1.0, Color32::RED));
        painter.line_segment([Pos2::new(left.min.x, s.y), Pos2::new(left.max.x, s.y)], Stroke::new(1.0, Color32::RED));
    }
    painter.rect_filled(Rect::from_min_size(rect.min, Vec2::splat(t)), 0.0, bg);
}
