//! Kartenausschnitt aus OpenStreetMap (für Übersichtspläne). Kacheln © OpenStreetMap-Mitwirkende (ODbL).

use crate::model::ImageData;

#[cfg(not(target_arch = "wasm32"))]
const UA: &str = concat!("OpenFluchtplan/", env!("CARGO_PKG_VERSION"), " (+https://github.com/Schrabbe10/OpenFluchtplan)");

#[cfg(not(target_arch = "wasm32"))]
pub fn agent() -> ureq::Agent {
    // Windows-Zertifikatspeicher verwenden (Firmen-CA bei TLS-Inspektion im Firmennetz)
    let tls = ureq::tls::TlsConfig::builder()
        .provider(ureq::tls::TlsProvider::NativeTls)
        .root_certs(ureq::tls::RootCerts::PlatformVerifier)
        .build();
    ureq::Agent::config_builder()
        .tls_config(tls)
        .user_agent(UA)
        .timeout_global(Some(std::time::Duration::from_secs(20)))
        .build()
        .into()
}

/// HTTP-GET mit Abfrageparametern; liefert Inhalt und Content-Type. Einziger Netzwerkzugriff des Programms.
#[cfg(not(target_arch = "wasm32"))]
pub fn http_get(url: &str, query: &[(&str, &str)]) -> Result<(Vec<u8>, String), String> {
    use std::io::Read;
    let mut req = agent().get(url);
    for (k, v) in query {
        req = req.query(*k, *v);
    }
    let mut resp = req.call().map_err(|e| format!("Server nicht erreichbar: {e}"))?;
    let ct = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    let mut bytes = Vec::new();
    resp.body_mut().as_reader().read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    Ok((bytes, ct))
}

/// Im Browser sind Kartendienste noch nicht angebunden (Abrufe dort nur asynchron und je nach Dienst durch CORS gesperrt).
#[cfg(target_arch = "wasm32")]
pub fn http_get(_url: &str, _query: &[(&str, &str)]) -> Result<(Vec<u8>, String), String> {
    Err("Kartendienste (OpenStreetMap, WMS) sind in der Browser-Version noch nicht verfügbar – bitte die Desktop-Version verwenden.".into())
}

/// Adresse -> Koordinaten (Nominatim).
pub fn geocode(query: &str) -> Result<(f64, f64), String> {
    let (body, _) = http_get("https://nominatim.openstreetmap.org/search", &[("format", "json"), ("limit", "1"), ("q", query)]).map_err(|e| format!("Adresssuche fehlgeschlagen: {e}"))?;
    let v: serde_json::Value = serde_json::from_slice(&body).map_err(|e| e.to_string())?;
    let first = v.get(0).ok_or("Adresse nicht gefunden")?;
    let lat = first["lat"].as_str().and_then(|s| s.parse().ok()).ok_or("Ungültige Antwort")?;
    let lon = first["lon"].as_str().and_then(|s| s.parse().ok()).ok_or("Ungültige Antwort")?;
    Ok((lat, lon))
}

fn tile_xy(lat: f64, lon: f64, z: u32) -> (f64, f64) {
    let n = 2f64.powi(z as i32);
    let x = (lon + 180.0) / 360.0 * n;
    let lr = lat.to_radians();
    let y = (1.0 - (lr.tan() + 1.0 / lr.cos()).ln() / std::f64::consts::PI) / 2.0 * n;
    (x, y)
}

/// Kartenbild um einen Punkt (Breite x Höhe in Kacheln à 256 px).
pub fn map_image(lat: f64, lon: f64, z: u32, tiles_x: u32, tiles_y: u32) -> Result<ImageData, String> {
    let (cx, cy) = tile_xy(lat, lon, z);
    let (w, h) = (tiles_x * 256, tiles_y * 256);
    let x0 = cx - tiles_x as f64 / 2.0;
    let y0 = cy - tiles_y as f64 / 2.0;
    let mut canvas = image::RgbaImage::from_pixel(w, h, image::Rgba([255, 255, 255, 255]));
    let n = 2i64.pow(z);
    for ty in y0.floor() as i64..=(y0 + tiles_y as f64).floor() as i64 {
        for tx in x0.floor() as i64..=(x0 + tiles_x as f64).floor() as i64 {
            if ty < 0 || ty >= n {
                continue;
            }
            let txw = tx.rem_euclid(n);
            let url = format!("https://tile.openstreetmap.org/{z}/{txw}/{ty}.png");
            let (bytes, _) = http_get(&url, &[]).map_err(|e| format!("Kartenkachel nicht geladen: {e}"))?;
            let tile = image::load_from_memory(&bytes).map_err(|e| e.to_string())?.to_rgba8();
            let px = ((tx as f64 - x0) * 256.0).round() as i64;
            let py = ((ty as f64 - y0) * 256.0).round() as i64;
            image::imageops::overlay(&mut canvas, &tile, px, py);
        }
    }
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(canvas)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    crate::import::image_data(&png, "image/png")
}
