//! The surf report for Redondo Beach (spec 4.5). All data from NOAA, fetched by the
//! server every 30 minutes. The page only calls `/api/surf`.
//!
//! - NDBC buoy 46221 (Santa Monica Bay): swell, waves, water temperature.
//! - NWS station KTOA (Torrance airport): wind. The buoy has no wind sensor.
//! - CO-OPS station 9410738 (King Harbor): tide highs and lows.

use std::time::Duration;

use axum::{
    Json,
    extract::State,
    http::header,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::AppState;

const NDBC_TXT: &str = "https://www.ndbc.noaa.gov/data/realtime2/46221.txt";
const NDBC_SPEC: &str = "https://www.ndbc.noaa.gov/data/realtime2/46221.spec";
const NWS_OBS: &str = "https://api.weather.gov/stations/KTOA/observations/latest";
/// Tide predictions from yesterday for 72 hours. (`range` alone looks back, not ahead.)
fn coops_url(now: time::OffsetDateTime) -> String {
    let d = (now - time::Duration::days(1)).date();
    format!(
        "https://api.tidesandcurrents.noaa.gov/api/prod/datagetter?product=predictions&station=9410738&begin_date={:04}{:02}{:02}&range=72&datum=MLLW&interval=hilo&units=english&time_zone=lst_ldt&format=json&application=logbook",
        d.year(),
        u8::from(d.month()),
        d.day()
    )
}

const M_TO_FT: f64 = 3.280_84;
const KMH_TO_KT: f64 = 0.539_957;

/// Swell from the spectral file.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Swell {
    pub height_ft: f64,
    pub period_s: f64,
    /// Compass point, for example `SSW`.
    pub direction: String,
    pub observed_at: String,
}

/// Waves and water from the standard file.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Waves {
    pub height_ft: Option<f64>,
    pub water_c: Option<f64>,
    pub observed_at: String,
}

/// Wind at the airport.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Wind {
    pub speed_kt: f64,
    pub direction_deg: f64,
    pub direction: String,
    /// Offshore for Redondo: wind from the land side (north-east through south-east).
    pub offshore: bool,
    pub observed_at: String,
}

/// One tide high or low.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Tide {
    /// Local time, `YYYY-MM-DD HH:MM`.
    pub time: String,
    pub height_ft: f64,
    /// `H` or `L`.
    pub kind: String,
}

/// Everything the surf window shows. Each part can be missing on its own.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Surf {
    pub swell: Option<Swell>,
    pub waves: Option<Waves>,
    pub wind: Option<Wind>,
    pub tides: Vec<Tide>,
    pub rating: Option<String>,
    pub fetched_at: String,
}

/// `YYYY MM DD hh mm` columns → RFC 3339 (UTC).
fn ndbc_time(cols: &[&str]) -> Option<String> {
    let n: Vec<u32> = cols
        .iter()
        .take(5)
        .map(|c| c.parse().ok())
        .collect::<Option<_>>()?;
    Some(format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:00Z",
        n[0], n[1], n[2], n[3], n[4]
    ))
}

/// The newest data row of an NDBC file, split into columns. Rows start after `#` lines.
fn newest_row(text: &str) -> Option<Vec<&str>> {
    text.lines()
        .find(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| l.split_whitespace().collect())
}

fn num(s: &str) -> Option<f64> {
    if s == "MM" {
        None
    } else {
        s.parse().ok().filter(|v: &f64| v.is_finite())
    }
}

/// Parses `46221.spec`: `SwH` (m), `SwP` (s), `SwD`.
#[must_use]
pub fn parse_spec(text: &str) -> Option<Swell> {
    let header: Vec<&str> = text
        .lines()
        .next()?
        .trim_start_matches('#')
        .split_whitespace()
        .collect();
    let row = newest_row(text)?;
    let col = |name: &str| {
        header
            .iter()
            .position(|h| *h == name)
            .and_then(|i| row.get(i).copied())
    };
    Some(Swell {
        height_ft: num(col("SwH")?)? * M_TO_FT,
        period_s: num(col("SwP")?)?,
        direction: col("SwD").filter(|d| *d != "MM").unwrap_or("").to_string(),
        observed_at: ndbc_time(&row)?,
    })
}

/// Parses `46221.txt`: `WVHT` (m), `WTMP` (°C).
#[must_use]
pub fn parse_txt(text: &str) -> Option<Waves> {
    let header: Vec<&str> = text
        .lines()
        .next()?
        .trim_start_matches('#')
        .split_whitespace()
        .collect();
    let row = newest_row(text)?;
    let col = |name: &str| {
        header
            .iter()
            .position(|h| *h == name)
            .and_then(|i| row.get(i).copied())
            .and_then(num)
    };
    Some(Waves {
        height_ft: col("WVHT").map(|m| m * M_TO_FT),
        water_c: col("WTMP"),
        observed_at: ndbc_time(&row)?,
    })
}

/// Degrees → 16-point compass.
#[must_use]
pub fn compass(deg: f64) -> &'static str {
    const POINTS: [&str; 16] = [
        "N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW", "WSW", "W", "WNW",
        "NW", "NNW",
    ];
    // Bounded to 0..16 by rem_euclid and the modulo.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let i = ((deg.rem_euclid(360.0) / 22.5).round() as usize) % 16;
    POINTS[i]
}

#[derive(Deserialize)]
struct NwsValue {
    value: Option<f64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NwsProps {
    timestamp: String,
    wind_direction: NwsValue,
    wind_speed: NwsValue,
}

#[derive(Deserialize)]
struct NwsObs {
    properties: NwsProps,
}

/// Parses the NWS observation. `None` when the station reports no wind.
#[must_use]
pub fn parse_nws(json: &str) -> Option<Wind> {
    let o: NwsObs = serde_json::from_str(json).ok()?;
    let kmh = o.properties.wind_speed.value?;
    let deg = o.properties.wind_direction.value.unwrap_or(0.0);
    Some(Wind {
        speed_kt: kmh * KMH_TO_KT,
        direction_deg: deg,
        direction: compass(deg).to_string(),
        offshore: (30.0..=150.0).contains(&deg) && kmh > 0.0,
        observed_at: o.properties.timestamp,
    })
}

#[derive(Deserialize)]
struct CoopsPred {
    t: String,
    v: String,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Deserialize)]
struct Coops {
    predictions: Vec<CoopsPred>,
}

/// Parses CO-OPS high/low predictions.
#[must_use]
pub fn parse_tides(json: &str) -> Vec<Tide> {
    serde_json::from_str::<Coops>(json)
        .map(|c| {
            c.predictions
                .into_iter()
                .filter_map(|p| {
                    Some(Tide {
                        time: p.t,
                        height_ft: p.v.parse().ok()?,
                        kind: p.kind,
                    })
                })
                .filter(|t| t.kind == "H" || t.kind == "L")
                .collect()
        })
        .unwrap_or_default()
}

/// A one-line rating from swell and wind (spec 4.5).
#[must_use]
pub fn rating(swell: Option<&Swell>, wind: Option<&Wind>) -> Option<String> {
    let s = swell?;
    let light = wind.is_none_or(|w| w.speed_kt < 6.0);
    let offshore = wind.is_some_and(|w| w.offshore);
    let blown = wind.is_some_and(|w| !w.offshore && w.speed_kt >= 12.0);
    let text = if s.height_ft < 1.0 {
        "Flat."
    } else if blown {
        "Blown out. Onshore wind."
    } else if s.height_ft < 2.0 {
        "Small. A longboard day."
    } else if s.period_s >= 12.0 && (offshore || light) {
        "Good. Long-period swell, clean wind."
    } else {
        "Fair."
    };
    Some(text.to_string())
}

async fn text(client: &reqwest::Client, url: &str) -> Option<String> {
    let res = client.get(url).send().await.ok()?;
    if !res.status().is_success() {
        tracing::warn!("surf: {url} returned {}", res.status());
        return None;
    }
    res.text().await.ok()
}

/// Fetches all sources. A failed source leaves its part empty.
pub async fn fetch(client: &reqwest::Client) -> Surf {
    let coops_url = coops_url(time::OffsetDateTime::now_utc());
    let (spec, txt, nws, coops) = tokio::join!(
        text(client, NDBC_SPEC),
        text(client, NDBC_TXT),
        text(client, NWS_OBS),
        text(client, &coops_url)
    );
    let swell = spec.as_deref().and_then(parse_spec);
    let waves = txt.as_deref().and_then(parse_txt);
    let wind = nws.as_deref().and_then(parse_nws);
    let tides = coops.as_deref().map(parse_tides).unwrap_or_default();
    let rating = rating(swell.as_ref(), wind.as_ref());
    let fetched_at = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();
    Surf {
        swell,
        waves,
        wind,
        tides,
        rating,
        fetched_at,
    }
}

/// Fetches every 30 minutes. Keeps the last good part when a source fails.
pub fn spawn_fetch(state: AppState, user_agent: String) {
    tokio::spawn(async move {
        let client = match reqwest::Client::builder()
            .user_agent(user_agent)
            .timeout(Duration::from_secs(20))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("surf: cannot build the HTTP client: {e}");
                return;
            }
        };
        let mut tick = tokio::time::interval(Duration::from_mins(30));
        loop {
            tick.tick().await;
            let new = fetch(&client).await;
            let mut cur = state.surf.write().await;
            let old = cur.take().unwrap_or_default();
            let merged = Surf {
                swell: new.swell.or(old.swell),
                waves: new.waves.or(old.waves),
                wind: new.wind.or(old.wind),
                tides: if new.tides.is_empty() {
                    old.tides
                } else {
                    new.tides
                },
                rating: new.rating.or(old.rating),
                fetched_at: new.fetched_at,
            };
            *cur = Some(merged);
        }
    });
}

/// `GET /api/surf`: the cached report, or `{ "available": false }`.
pub async fn api_surf(State(s): State<AppState>) -> Response {
    let cur = s.surf.read().await.clone();
    let body = match cur {
        Some(surf) => serde_json::json!({ "available": true, "surf": surf }),
        None => serde_json::json!({ "available": false }),
    };
    ([(header::CACHE_CONTROL, "public, max-age=300")], Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(format!(
            "{}/tests/fixtures/noaa/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

    #[test]
    fn parses_the_real_spectral_file() {
        let s = parse_spec(&fixture("46221.spec")).unwrap();
        assert!((s.height_ft - 0.6 * M_TO_FT).abs() < 1e-9);
        assert!((s.period_s - 14.3).abs() < 1e-9);
        assert_eq!(s.direction, "SSW");
        assert_eq!(s.observed_at, "2026-10-05T00:26:00Z");
    }

    #[test]
    fn parses_the_real_standard_file() {
        let w = parse_txt(&fixture("46221.txt")).unwrap();
        assert!((w.height_ft.unwrap() - 0.8 * M_TO_FT).abs() < 1e-9);
        assert_eq!(w.water_c, Some(24.6));
    }

    #[test]
    fn missing_values_are_none_not_zero() {
        let text = "#YY  MM DD hh mm WVHT  SwH  SwP  WWH  WWP SwD WWD  STEEPNESS  APD MWD\n#yr\n2026 10 05 00 26  MM   MM   MM   MM   MM  MM  MM N/A MM MM\n";
        assert_eq!(parse_spec(text), None);
        let standard = "#YY  MM DD hh mm WDIR WSPD GST  WVHT   DPD   APD MWD   PRES  ATMP  WTMP\n#yr\n2026 10 05 00 26  MM MM MM MM MM MM MM MM MM MM\n";
        let w = parse_txt(standard).unwrap();
        assert_eq!((w.height_ft, w.water_c), (None, None));
    }

    #[test]
    fn parses_nws_wind_and_handles_null() {
        assert_eq!(
            parse_nws(&fixture("ktoa.json")),
            None,
            "the real file has null wind"
        );
        let w = parse_nws(&fixture("ktoa_wind.json")).unwrap();
        assert!((w.speed_kt - 18.36 * KMH_TO_KT).abs() < 1e-9);
        assert_eq!(w.direction, "S");
        assert!(!w.offshore);
        assert_eq!(parse_nws("not json"), None);
    }

    #[test]
    fn parses_tides() {
        let t = parse_tides(&fixture("tides.json"));
        assert_eq!(t.len(), 7);
        assert_eq!(
            t[0],
            Tide {
                time: "2026-10-04 06:52".into(),
                height_ft: 3.829,
                kind: "H".into()
            }
        );
        assert!(t[3].height_ft < 0.0, "negative tides parse");
        assert!(parse_tides("{\"error\":{}}").is_empty());
    }
    #[test]
    fn tide_url_starts_yesterday() {
        let now = time::macros::datetime!(2026-03-01 08:00 UTC);
        assert!(coops_url(now).contains("begin_date=20260228&range=72"));
    }

    #[test]
    fn compass_points() {
        assert_eq!(compass(0.0), "N");
        assert_eq!(compass(359.0), "N");
        assert_eq!(compass(90.0), "E");
        assert_eq!(compass(202.5), "SSW");
        assert_eq!(compass(-90.0), "W");
    }

    #[test]
    fn ratings() {
        let swell = |h: f64, p: f64| Swell {
            height_ft: h,
            period_s: p,
            direction: "SSW".into(),
            observed_at: String::new(),
        };
        let wind = |kt: f64, deg: f64| Wind {
            speed_kt: kt,
            direction_deg: deg,
            direction: compass(deg).into(),
            offshore: (30.0..=150.0).contains(&deg),
            observed_at: String::new(),
        };
        assert_eq!(rating(None, None), None);
        assert_eq!(rating(Some(&swell(0.5, 14.0)), None).unwrap(), "Flat.");
        assert_eq!(
            rating(Some(&swell(3.0, 14.0)), Some(&wind(15.0, 250.0))).unwrap(),
            "Blown out. Onshore wind."
        );
        assert_eq!(
            rating(Some(&swell(1.5, 14.0)), None).unwrap(),
            "Small. A longboard day."
        );
        assert_eq!(
            rating(Some(&swell(3.0, 14.0)), Some(&wind(8.0, 70.0))).unwrap(),
            "Good. Long-period swell, clean wind."
        );
        assert_eq!(
            rating(Some(&swell(3.0, 8.0)), Some(&wind(8.0, 250.0))).unwrap(),
            "Fair."
        );
    }
}
