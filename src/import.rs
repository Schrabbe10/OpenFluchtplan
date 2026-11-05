//! Import: Bilder, PDF-Seiten (gerastert), DXF (Vektor), DWG (über ODA File Converter).

use std::path::{Path, PathBuf};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;

use crate::geom::*;
use crate::model::*;

pub fn image_data(bytes: &[u8], mime: &str) -> Result<ImageData, String> {
    let img = image::load_from_memory(bytes).map_err(|e| format!("Bild nicht lesbar: {e}"))?;
    Ok(ImageData { mime: mime.into(), b64: B64.encode(bytes), w: img.width(), h: img.height() })
}

pub fn load_image_file(path: &Path) -> Result<ImageData, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    match ext.as_str() {
        "png" => image_data(&bytes, "image/png"),
        "jpg" | "jpeg" => image_data(&bytes, "image/jpeg"),
        _ => {
            // andere Formate (bmp, gif, tif) nach PNG umwandeln
            let img = image::load_from_memory(&bytes).map_err(|e| format!("Bild nicht lesbar: {e}"))?;
            let mut out = Vec::new();
            img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).map_err(|e| e.to_string())?;
            image_data(&out, "image/png")
        }
    }
}

/// Anzahl Seiten einer PDF-Datei.
pub fn pdf_page_count(path: &Path) -> Result<usize, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let pdf = hayro::hayro_syntax::Pdf::new(bytes).map_err(|e| format!("PDF nicht lesbar: {e:?}"))?;
    Ok(pdf.pages().len())
}

/// PDF-Seite als Bild (für Grundrisse aus PDF).
pub fn pdf_page_image(path: &Path, page: usize, dpi: f32) -> Result<ImageData, String> {
    use hayro::hayro_interpret::InterpreterSettings;
    use hayro::{PixmapSettings, RenderCache, RenderSettings, render};
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let pdf = hayro::hayro_syntax::Pdf::new(bytes).map_err(|e| format!("PDF nicht lesbar: {e:?}"))?;
    let pages = pdf.pages();
    let pg = pages.get(page).ok_or("Seite nicht vorhanden")?;
    let (w, h) = pg.render_dimensions();
    let mut scale = dpi / 72.0;
    let max = w.max(h) * scale;
    if max > 8000.0 {
        scale *= 8000.0 / max;
    }
    let cache = RenderCache::new();
    let settings = InterpreterSettings::default();
    let pm = render(
        pg,
        &cache,
        &settings,
        &RenderSettings::default(),
        &PixmapSettings { x_scale: scale, y_scale: scale, bg_color: hayro::vello_cpu::color::palette::css::WHITE },
    );
    let png = pm.into_png().map_err(|e| format!("PNG-Fehler: {e:?}"))?;
    image_data(&png, "image/png")
}

/// Bild seitentreu in einen Bereich einpassen.
pub fn fit_rect(area: BBox, w: u32, h: u32) -> (Pos, Pos) {
    let (iw, ih) = (w.max(1) as f32, h.max(1) as f32);
    let f = (area.w() / iw).min(area.h() / ih);
    let (rw, rh) = (iw * f, ih * f);
    let a = Pos::new(area.min.x + (area.w() - rw) / 2.0, area.min.y + (area.h() - rh) / 2.0);
    (a, a + Pos::new(rw, rh))
}

// ---------- DXF ----------

#[derive(Default)]
struct Collected {
    lines: Vec<Vec<Pos>>,
    closed: Vec<bool>,
    texts: Vec<(Pos, String, f32, f32)>,
}

fn arc_poly(cx: f64, cy: f64, r: f64, a0: f64, a1: f64) -> Vec<Pos> {
    let mut a1 = a1;
    if a1 < a0 {
        a1 += 360.0;
    }
    let n = (((a1 - a0) / 6.0).ceil() as usize).clamp(4, 120);
    (0..=n)
        .map(|i| {
            let a = (a0 + (a1 - a0) * i as f64 / n as f64).to_radians();
            Pos::new((cx + r * a.cos()) as f32, (cy + r * a.sin()) as f32)
        })
        .collect()
}

fn collect_entities<'a>(
    drawing: &'a dxf::Drawing,
    entities: impl Iterator<Item = &'a dxf::entities::Entity>,
    tf: &dyn Fn(Pos) -> Pos,
    out: &mut Collected,
    depth: u32,
) {
    use dxf::entities::EntityType as E;
    for e in entities {
        match &e.specific {
            E::Line(l) => {
                out.lines.push(vec![tf(Pos::new(l.p1.x as f32, l.p1.y as f32)), tf(Pos::new(l.p2.x as f32, l.p2.y as f32))]);
                out.closed.push(false);
            }
            E::LwPolyline(p) => {
                out.lines.push(p.vertices.iter().map(|v| tf(Pos::new(v.x as f32, v.y as f32))).collect());
                out.closed.push(p.is_closed());
            }
            E::Polyline(p) => {
                out.lines.push(p.vertices().map(|v| tf(Pos::new(v.location.x as f32, v.location.y as f32))).collect());
                out.closed.push(p.is_closed());
            }
            E::Circle(c) => {
                out.lines.push(arc_poly(c.center.x, c.center.y, c.radius, 0.0, 360.0).into_iter().map(tf).collect());
                out.closed.push(true);
            }
            E::Arc(a) => {
                out.lines.push(arc_poly(a.center.x, a.center.y, a.radius, a.start_angle, a.end_angle).into_iter().map(tf).collect());
                out.closed.push(false);
            }
            E::Text(t) => {
                out.texts.push((tf(Pos::new(t.location.x as f32, t.location.y as f32)), t.value.clone(), t.text_height as f32, t.rotation as f32));
            }
            E::MText(t) => {
                let s = t.text.replace("\\P", "\n");
                out.texts.push((tf(Pos::new(t.insertion_point.x as f32, t.insertion_point.y as f32)), s, t.initial_text_height as f32, t.rotation_angle as f32));
            }
            E::Insert(ins) => {
                if depth > 6 {
                    continue;
                }
                if let Some(block) = drawing.blocks().find(|b| b.name == ins.name) {
                    let (bx, by) = (block.base_point.x as f32, block.base_point.y as f32);
                    let (sx, sy) = (ins.x_scale_factor as f32, ins.y_scale_factor as f32);
                    let rot = ins.rotation as f32;
                    let (ix, iy) = (ins.location.x as f32, ins.location.y as f32);
                    let inner = move |p: Pos| {
                        let q = Pos::new((p.x - bx) * sx, (p.y - by) * sy);
                        // DXF dreht gegen den Uhrzeigersinn (y nach oben)
                        let (s, c) = rot.to_radians().sin_cos();
                        Pos::new(ix + q.x * c - q.y * s, iy + q.x * s + q.y * c)
                    };
                    let composed = |p: Pos| tf(inner(p));
                    collect_entities(drawing, block.entities.iter(), &composed, out, depth + 1);
                }
            }
            _ => {}
        }
    }
}

/// DXF als Gruppe von Linien/Texten, eingepasst in `area` (oder im Planmaßstab, wenn `use_scale`).
pub fn load_dxf(path: &Path, area: BBox, scale_n: f32, use_scale: bool) -> Result<Vec<Obj>, String> {
    let drawing = dxf::Drawing::load_file(path).map_err(|e| format!("DXF nicht lesbar: {e}"))?;
    let mut col = Collected::default();
    let ident = |p: Pos| p;
    collect_entities(&drawing, drawing.entities(), &ident, &mut col, 0);
    if col.lines.is_empty() && col.texts.is_empty() {
        return Err("Die DXF-Datei enthält keine unterstützten Elemente.".into());
    }
    let mut bb = BBox::EMPTY;
    for l in &col.lines {
        for p in l {
            bb.add(*p);
        }
    }
    for t in &col.texts {
        bb.add(t.0);
    }
    // DXF: y nach oben -> spiegeln; Einheiten: mm angenommen
    let f = if use_scale { 1.0 / scale_n } else { (area.w() / bb.w().max(1e-6)).min(area.h() / bb.h().max(1e-6)) };
    let off = Pos::new(area.center().x - bb.center().x * f, area.center().y + bb.center().y * f);
    let map = |p: Pos| Pos::new(off.x + p.x * f, off.y - p.y * f);
    let mut objs = Vec::new();
    for (l, closed) in col.lines.iter().zip(col.closed.iter()) {
        if l.len() < 2 {
            continue;
        }
        objs.push(Obj {
            id: 0,
            layer: 0,
            kind: ObjKind::Polyline { pts: l.iter().map(|p| map(*p)).collect(), closed: *closed },
            style: Style::line(farbe::SCHWARZ, 0.25),
            print: true,
            locked: false,
        });
    }
    for (p, s, hgt, rot) in &col.texts {
        if s.trim().is_empty() {
            continue;
        }
        let size = (hgt * f / 0.72).max(1.0);
        objs.push(Obj {
            id: 0,
            layer: 0,
            kind: ObjKind::Text { pos: map(*p) - Pos::new(0.0, size * 0.8), text: s.clone(), size, bold: false, italic: false, align: TAlign::Links, rot: -rot },
            style: Style::line(farbe::SCHWARZ, 0.2),
            print: true,
            locked: false,
        });
    }
    Ok(objs)
}

/// ODA File Converter (kostenlos von der Open Design Alliance) für DWG -> DXF.
pub fn find_oda_converter() -> Option<PathBuf> {
    let roots = [std::env::var("ProgramFiles").unwrap_or_default(), std::env::var("ProgramFiles(x86)").unwrap_or_default()];
    for r in roots.iter().filter(|r| !r.is_empty()) {
        let base = Path::new(r).join("ODA");
        if let Ok(entries) = std::fs::read_dir(&base) {
            for e in entries.flatten() {
                let exe = e.path().join("ODAFileConverter.exe");
                if exe.exists() {
                    return Some(exe);
                }
            }
        }
    }
    None
}

/// DWG -> DXF umwandeln, liefert Pfad der DXF-Datei.
pub fn dwg_to_dxf(dwg: &Path, out_version: &str, to_dwg: bool) -> Result<PathBuf, String> {
    let exe = find_oda_converter().ok_or(
        "Für DWG wird der kostenlose „ODA File Converter“ benötigt (opendesign.com). Nach der Installation funktioniert der DWG-Import/-Export automatisch.",
    )?;
    let in_dir = dwg.parent().ok_or("Ungültiger Pfad")?;
    let out_dir = std::env::temp_dir().join(format!("fluchtplan_oda_{}", std::process::id()));
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let file = dwg.file_name().and_then(|f| f.to_str()).ok_or("Ungültiger Dateiname")?;
    let fmt = if to_dwg { "DWG" } else { "DXF" };
    let status = std::process::Command::new(exe)
        .arg(in_dir)
        .arg(&out_dir)
        .arg(out_version)
        .arg(fmt)
        .arg("0")
        .arg("1")
        .arg(file)
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("ODA File Converter meldet einen Fehler.".into());
    }
    let ext = if to_dwg { "dwg" } else { "dxf" };
    let out = out_dir.join(Path::new(file).with_extension(ext).file_name().unwrap());
    if out.exists() { Ok(out) } else { Err("Umwandlung fehlgeschlagen.".into()) }
}
