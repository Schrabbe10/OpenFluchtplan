//! Flurkarten und amtliche Karten über WMS (OGC Web Map Service 1.3.0, EPSG:3857).

use std::io::Read;

use crate::model::ImageData;

/// Vorbelegte Dienste. Die Nutzungsbedingungen liefert der Dienst selbst (GetCapabilities).
pub const PRESETS: &[(&str, &str, &str)] = &[
    (
        "ALKIS Schleswig-Holstein (LVermGeo SH)",
        "https://service.gdi-sh.de/WMS_SH_ALKIS_OpenGBD",
        "adv_alkis_flurstuecke,adv_alkis_gebaeude,adv_alkis_verkehr",
    ),
    (
        "ALKIS Nordrhein-Westfalen (dl-de/zero)",
        "https://www.wms.nrw.de/geobasis/wms_nw_alkis",
        "adv_alkis_flurstuecke,adv_alkis_gebaeude,adv_alkis_verkehr",
    ),
    ("basemap.de Web Raster (BKG, CC BY 4.0)", "https://sgx.geodatenzentrum.de/wms_basemapde", "de_basemapde_web_raster_grau"),
];

#[derive(Clone, Debug, Default)]
pub struct Layer {
    pub name: String,
    pub title: String,
}

#[derive(Clone, Debug, Default)]
pub struct Caps {
    pub title: String,
    pub layers: Vec<Layer>,
    /// Nutzungsbedingungen (Fees + AccessConstraints)
    pub terms: String,
    /// Quellenvermerk für den Plan
    pub attribution: String,
    pub supports_3857: bool,
    /// Basis-URL für GetMap
    pub map_url: String,
}

fn base(url: &str) -> String {
    let u = url.trim();
    let u = u.split('?').next().unwrap_or(u);
    u.to_string()
}

fn get_bytes(url: &str, query: &[(&str, &str)]) -> Result<(Vec<u8>, String), String> {
    let mut req = crate::osm::agent().get(url);
    for (k, v) in query {
        req = req.query(*k, *v);
    }
    let mut resp = req.call().map_err(|e| format!("Kartendienst nicht erreichbar: {e}"))?;
    let ct = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    let mut bytes = Vec::new();
    resp.body_mut().as_reader().read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    Ok((bytes, ct))
}

/// Dienstbeschreibung abrufen.
pub fn capabilities(url: &str) -> Result<Caps, String> {
    let (bytes, _) = get_bytes(&base(url), &[("SERVICE", "WMS"), ("REQUEST", "GetCapabilities"), ("VERSION", "1.3.0")])?;
    parse_capabilities(&String::from_utf8_lossy(&bytes), &base(url))
}

pub fn parse_capabilities(xml: &str, url: &str) -> Result<Caps, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| format!("Ungültige Dienstbeschreibung: {e}"))?;
    let root = doc.root_element();
    if root.tag_name().name().contains("Exception") {
        return Err(format!("Dienst meldet Fehler: {}", root.text().unwrap_or("").trim()));
    }
    fn child<'a, 'i>(n: roxmltree::Node<'a, 'i>, name: &str) -> Option<roxmltree::Node<'a, 'i>> {
        n.children().find(|c| c.tag_name().name() == name)
    }
    fn text(n: Option<roxmltree::Node>) -> String {
        n.and_then(|n| n.text()).unwrap_or("").trim().to_string()
    }
    let mut caps = Caps { map_url: url.to_string(), ..Default::default() };
    if let Some(service) = child(root, "Service") {
        caps.title = text(child(service, "Title"));
        let fees = text(child(service, "Fees"));
        let ac = text(child(service, "AccessConstraints"));
        caps.terms = [fees, ac].into_iter().filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("none")).collect::<Vec<_>>().join("\n\n");
    }
    for n in root.descendants().filter(|n| n.tag_name().name() == "Layer") {
        let name = text(child(n, "Name"));
        if !name.is_empty() {
            let title = text(child(n, "Title"));
            caps.layers.push(Layer { title: if title.is_empty() { name.clone() } else { title }, name });
        }
        if caps.attribution.is_empty() {
            if let Some(a) = child(n, "Attribution") {
                caps.attribution = text(child(a, "Title"));
            }
        }
    }
    caps.supports_3857 = root.descendants().any(|n| matches!(n.tag_name().name(), "CRS" | "SRS") && n.text().is_some_and(|t| t.trim() == "EPSG:3857"));
    // GetMap-Adresse aus der Beschreibung übernehmen, falls angegeben
    if let Some(gm) = root.descendants().find(|n| n.tag_name().name() == "GetMap") {
        if let Some(href) = gm.descendants().find(|n| n.tag_name().name() == "OnlineResource").and_then(|n| n.attributes().find(|a| a.name() == "href").map(|a| a.value().to_string())) {
            if href.starts_with("http") {
                caps.map_url = base(&href);
            }
        }
    }
    if caps.attribution.is_empty() {
        // Quellenvermerk steht oft in den Nutzungsbedingungen
        if let Some(i) = caps.terms.find('©') {
            caps.attribution = caps.terms[i..].lines().next().unwrap_or("").split("||").next().unwrap_or("").trim().to_string();
        }
    }
    if caps.attribution.is_empty() {
        caps.attribution = format!("Quelle: {}", if caps.title.is_empty() { url } else { &caps.title });
    }
    Ok(caps)
}

/// Web-Mercator-Koordinaten (m) eines Punktes.
pub fn to_3857(lat: f64, lon: f64) -> (f64, f64) {
    const R: f64 = 6378137.0;
    let x = R * lon.to_radians();
    let y = R * ((std::f64::consts::FRAC_PI_4 + lat.to_radians() / 2.0).tan()).ln();
    (x, y)
}

/// Ausschnitt von `w_m` × `h_m` Metern (Natur) um den Punkt als Bild mit `px_w` × `px_h` Pixeln.
pub fn get_map(map_url: &str, layers: &[String], lat: f64, lon: f64, w_m: f64, h_m: f64, px_w: u32, px_h: u32) -> Result<ImageData, String> {
    if layers.is_empty() {
        return Err("Bitte mindestens eine Ebene auswählen.".into());
    }
    let (cx, cy) = to_3857(lat, lon);
    // Mercator-Verzerrung: 1 m Natur = 1/cos(φ) m in EPSG:3857
    let k = 1.0 / lat.to_radians().cos();
    let (hw, hh) = (w_m * k / 2.0, h_m * k / 2.0);
    let bbox = format!("{:.3},{:.3},{:.3},{:.3}", cx - hw, cy - hh, cx + hw, cy + hh);
    let (w, h) = (px_w.to_string(), px_h.to_string());
    let layers = layers.join(",");
    let styles = vec![""; layers.split(',').count()].join(",");
    let (bytes, ct) = get_bytes(
        map_url,
        &[
            ("SERVICE", "WMS"),
            ("VERSION", "1.3.0"),
            ("REQUEST", "GetMap"),
            ("LAYERS", &layers),
            ("STYLES", &styles),
            ("CRS", "EPSG:3857"),
            ("BBOX", &bbox),
            ("WIDTH", &w),
            ("HEIGHT", &h),
            ("FORMAT", "image/png"),
            ("TRANSPARENT", "FALSE"),
            ("BGCOLOR", "0xFFFFFF"),
        ],
    )?;
    if ct.contains("xml") || bytes.starts_with(b"<?xml") || bytes.starts_with(b"<") {
        let msg = String::from_utf8_lossy(&bytes);
        let msg = roxmltree::Document::parse(&msg)
            .ok()
            .and_then(|d| d.descendants().find(|n| n.tag_name().name().contains("Exception")).and_then(|n| n.text().map(|t| t.trim().to_string())))
            .unwrap_or_else(|| msg.chars().take(300).collect());
        return Err(format!("Kartendienst meldet: {msg}"));
    }
    // Auf weißen Hintergrund legen (manche Dienste liefern trotzdem Transparenz)
    let img = image::load_from_memory(&bytes).map_err(|e| format!("Kartenbild unlesbar: {e}"))?.to_rgba8();
    let mut canvas = image::RgbaImage::from_pixel(img.width(), img.height(), image::Rgba([255, 255, 255, 255]));
    image::imageops::overlay(&mut canvas, &img, 0, 0);
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(canvas)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    crate::import::image_data(&png, "image/png")
}
