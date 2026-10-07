//! Upload and `/media/{key}` tests (spec 6.8, 7.4).

mod common;

use std::io::Cursor;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use common::*;
use http_body_util::BodyExt;
use tower::ServiceExt;

fn png() -> Vec<u8> {
    let img = image::RgbImage::from_pixel(8, 6, image::Rgb([10, 120, 200]));
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

/// A JPEG with an EXIF segment that holds a fake GPS tag.
fn jpeg_with_exif() -> Vec<u8> {
    let img = image::RgbImage::from_pixel(8, 6, image::Rgb([200, 120, 10]));
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Jpeg).unwrap();
    let jpg = out.into_inner();
    let payload = b"Exif\0\0GPSLatitude=33.8467N;GPSLongitude=118.398W";
    let len = u16::try_from(payload.len() + 2).unwrap().to_be_bytes();
    let mut with = vec![0xFF, 0xD8, 0xFF, 0xE1, len[0], len[1]];
    with.extend_from_slice(payload);
    with.extend_from_slice(&jpg[2..]);
    with
}

async fn upload(
    f: &Fixture,
    session: Option<&str>,
    name: &str,
    bytes: Vec<u8>,
) -> (StatusCode, String) {
    let boundary = "logbookboundary";
    let mut body = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: application/octet-stream\r\n\r\n").into_bytes();
    body.extend_from_slice(&bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let mut req = Request::post("/api/owner/uploads")
        .header(header::ORIGIN, ORIGIN)
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .header(header::CONTENT_LENGTH, body.len());
    if let Some(s) = session {
        req = req.header(header::COOKIE, format!("logbook_session={s}"));
    }
    let res = f
        .app
        .clone()
        .oneshot(req.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

async fn get_bytes(f: &Fixture, path: &str) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let res = f
        .app
        .clone()
        .oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let (status, headers) = (res.status(), res.headers().clone());
    (
        status,
        headers,
        res.into_body().collect().await.unwrap().to_bytes().to_vec(),
    )
}

#[tokio::test]
async fn upload_then_serve() {
    let f = fixture().await;
    let s = f.session(30).await;
    let (status, body) = upload(&f, Some(&s), "pic.png", png()).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    let key = v["key"].as_str().unwrap();
    assert!(logbook_core::media_key_ok(key.as_bytes()));
    assert_eq!(v["markdown"], format!("![](/media/{key})"));

    let (status, headers, bytes) = get_bytes(&f, &format!("/media/{key}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CONTENT_TYPE], "image/png");
    assert_eq!(headers["x-content-type-options"], "nosniff");
    assert!(
        headers[header::CACHE_CONTROL]
            .to_str()
            .unwrap()
            .contains("immutable")
    );
    assert_eq!(&bytes[..4], b"\x89PNG");
}

#[tokio::test]
async fn exif_is_removed() {
    let f = fixture().await;
    let s = f.session(30).await;
    let original = jpeg_with_exif();
    assert!(
        original.windows(4).any(|w| w == b"Exif"),
        "the test image has EXIF"
    );
    let (status, body) = upload(&f, Some(&s), "pic.jpg", original).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let key = serde_json::from_str::<serde_json::Value>(&body).unwrap()["key"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        std::path::Path::new(&key)
            .extension()
            .and_then(|e| e.to_str()),
        Some("jpg")
    );
    let (_, _, bytes) = get_bytes(&f, &format!("/media/{key}")).await;
    assert!(!bytes.windows(4).any(|w| w == b"Exif"), "EXIF survived");
    assert!(
        !bytes.windows(11).any(|w| w == b"GPSLatitude"),
        "GPS survived"
    );
}

#[tokio::test]
async fn bad_files_are_refused() {
    let f = fixture().await;
    let s = f.session(30).await;
    for (name, bytes) in [
        (
            "x.svg",
            b"<svg xmlns='http://www.w3.org/2000/svg' onload='alert(1)'/>".to_vec(),
        ),
        ("x.png", b"not really a png".to_vec()),
        ("x.html", b"<html><script>x</script></html>".to_vec()),
    ] {
        let (status, body) = upload(&f, Some(&s), name, bytes).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{name}: {body}");
    }
    let big = vec![0u8; 11 * 1024 * 1024];
    let (status, _) = upload(&f, Some(&s), "big.png", big).await;
    assert!(
        status == StatusCode::PAYLOAD_TOO_LARGE || status == StatusCode::BAD_REQUEST,
        "{status}"
    );
}

#[tokio::test]
async fn uploads_need_a_session() {
    let f = fixture().await;
    let (status, _) = upload(&f, None, "pic.png", png()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn media_keys_are_checked() {
    let f = fixture().await;
    let hex = "a".repeat(64);
    for path in [
        "/media/..%2fCargo.toml".to_string(),
        format!("/media/{hex}.svg"),
        format!("/media/{}.png", "A".repeat(64)),
        "/media/x.png".to_string(),
        format!("/media/{hex}.png"), // valid shape, not uploaded
    ] {
        let (status, _, _) = get_bytes(&f, &path).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
    }
}

#[tokio::test]
async fn exports_carry_images() {
    let f = fixture().await;
    let s = f.session(30).await;
    let (_, body) = upload(&f, Some(&s), "pic.png", png()).await;
    let key = serde_json::from_str::<serde_json::Value>(&body).unwrap()["key"]
        .as_str()
        .unwrap()
        .to_string();
    // Used by a public post.
    sqlx::query("UPDATE posts SET body_md = body_md || ? WHERE slug = ?")
        .bind(format!("\n\n![](/media/{key})"))
        .bind(PUBLIC_SLUG)
        .execute(&f.pool)
        .await
        .unwrap();
    assert_eq!(
        logbook_server::export::media_keys(&format!("x ![](/media/{key}) y /media/{key}")),
        vec![key.clone()]
    );

    let tmp = tempfile::tempdir().unwrap();
    let remote = tmp.path().join("remote.git");
    std::fs::create_dir(&remote).unwrap();
    assert!(
        std::process::Command::new("git")
            .args(["init", "-q", "--bare", "-b", "main"])
            .current_dir(&remote)
            .status()
            .unwrap()
            .success()
    );
    let cfg = logbook_server::export::GitExport {
        repo: remote.to_string_lossy().into_owned(),
        branch: "main".into(),
        dir: tmp.path().join("work/export"),
        deploy_key: None,
    };
    assert!(
        logbook_server::export::export_once(&f.pool, &cfg, &f.state.media)
            .await
            .unwrap()
    );
    assert!(
        cfg.dir.join("images").join(&key).exists(),
        "the image is in the git export"
    );
}

/// The e2e fixture: a 4 s, 96 x 54 VP9 WebM.
const WEBM: &[u8] = include_bytes!("../../../e2e/fixtures/clip.webm");

fn mp4_box(kind: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut b = u32::try_from(payload.len() + 8)
        .unwrap()
        .to_be_bytes()
        .to_vec();
    b.extend_from_slice(kind);
    b.extend_from_slice(payload);
    b
}

/// An MP4-shaped file with a GPS position in `moov/udta`.
fn mp4_with_gps() -> Vec<u8> {
    let gps = mp4_box(b"\xa9xyz", b"+33.8467-118.3980/");
    let moov = mp4_box(
        b"moov",
        &[mp4_box(b"mvhd", &[0; 12]), mp4_box(b"udta", &gps)].concat(),
    );
    [
        mp4_box(b"ftyp", b"isom\0\0\x02\0isomiso2mp41"),
        moov,
        mp4_box(b"mdat", &[7; 64]),
    ]
    .concat()
}

async fn get_range(
    f: &Fixture,
    path: &str,
    range: &str,
) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let res = f
        .app
        .clone()
        .oneshot(
            Request::get(path)
                .header(header::RANGE, range)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let (status, headers) = (res.status(), res.headers().clone());
    (
        status,
        headers,
        res.into_body().collect().await.unwrap().to_bytes().to_vec(),
    )
}

async fn upload_key(f: &Fixture, s: &str, name: &str, bytes: Vec<u8>) -> serde_json::Value {
    let (status, body) = upload(f, Some(s), name, bytes).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    serde_json::from_str(&body).unwrap()
}

#[tokio::test]
async fn video_upload_then_ranges() {
    let f = fixture().await;
    let s = f.session(30).await;
    let v = upload_key(&f, &s, "clip.webm", WEBM.to_vec()).await;
    let key = v["key"].as_str().unwrap().to_string();
    assert_eq!(&key[64..], ".webm", "{key}");
    assert!(logbook_core::video_key_ok(key.as_bytes()));
    assert_eq!(v["kind"], "video");
    assert_eq!(v["markdown"], format!("```video\n/media/{key}\n```"));
    let path = format!("/media/{key}");

    // The whole file.
    let (status, headers, bytes) = get_bytes(&f, &path).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bytes, WEBM, "WebM files are stored as they are");
    assert_eq!(headers[header::CONTENT_TYPE], "video/webm");
    assert_eq!(headers[header::ACCEPT_RANGES], "bytes");
    assert_eq!(headers[header::CONTENT_LENGTH], WEBM.len().to_string());
    assert_eq!(headers["x-content-type-options"], "nosniff");
    assert_eq!(
        headers[header::CACHE_CONTROL],
        "public, max-age=31536000, immutable"
    );

    // One range: 206 with Content-Range.
    let n = WEBM.len();
    let (status, headers, bytes) = get_range(&f, &path, "bytes=100-199").await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(headers[header::CONTENT_RANGE], format!("bytes 100-199/{n}"));
    assert_eq!(headers[header::CONTENT_LENGTH], "100");
    assert_eq!(headers[header::CONTENT_TYPE], "video/webm");
    assert_eq!(bytes, &WEBM[100..200]);

    let (status, headers, bytes) = get_range(&f, &path, "bytes=-10").await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        headers[header::CONTENT_RANGE],
        format!("bytes {}-{}/{n}", n - 10, n - 1)
    );
    assert_eq!(bytes, &WEBM[n - 10..]);

    let (status, _, bytes) = get_range(&f, &path, "bytes=5000-").await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(bytes, &WEBM[5000..]);

    // Past the end: 416.
    let (status, headers, _) = get_range(&f, &path, &format!("bytes={n}-")).await;
    assert_eq!(status, StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(headers[header::CONTENT_RANGE], format!("bytes */{n}"));

    // Several ranges: the whole file.
    let (status, _, bytes) = get_range(&f, &path, "bytes=0-1,4-5").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bytes.len(), n);

    // A second upload of the same video gives the same key.
    let again = upload_key(&f, &s, "again.webm", WEBM.to_vec()).await;
    assert_eq!(again["key"], key.as_str());
}

#[tokio::test]
async fn mp4_gps_is_blanked() {
    let f = fixture().await;
    let s = f.session(30).await;
    let original = mp4_with_gps();
    let v = upload_key(&f, &s, "phone.mp4", original.clone()).await;
    let key = v["key"].as_str().unwrap();
    assert_eq!(&key[64..], ".mp4", "{key}");
    let (status, headers, bytes) = get_bytes(&f, &format!("/media/{key}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CONTENT_TYPE], "video/mp4");
    assert_eq!(bytes.len(), original.len());
    assert!(!bytes.windows(8).any(|w| w == b"+33.8467"), "GPS survived");
    assert!(bytes.windows(64).any(|w| w == [7; 64]), "media kept");
}

#[tokio::test]
async fn bad_videos_are_refused() {
    let f = fixture().await;
    let s = f.session(30).await;
    for (name, bytes) in [
        (
            "x.mov",
            [mp4_box(b"ftyp", b"qt  \0\0\0\0qt  "), vec![0; 32]].concat(),
        ),
        (
            "x.mkv",
            b"\x1a\x45\xdf\xa3\x9f\x42\x86\x81\x01\x42\x82\x88matroska".to_vec(),
        ),
        (
            "broken.mp4",
            [
                mp4_box(b"ftyp", b"isom\0\0\0\0"),
                vec![0, 0, 0xff, 0xff, b'm'],
            ]
            .concat(),
        ),
    ] {
        let (status, body) = upload(&f, Some(&s), name, bytes).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{name}: {body}");
    }
}

#[tokio::test]
async fn videos_have_a_size_cap() {
    let f = fixture().await;
    let s = f.session(30).await;
    let cap = logbook_server::video::VIDEO_MAX;
    let mut big = WEBM.to_vec();
    big.resize(cap + 1, 0);
    let (status, body) = upload(&f, Some(&s), "big.webm", big).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE, "{body}");
    assert!(body.contains("100 MB"), "{body}");

    // Exactly at the cap: accepted.
    let mut max = WEBM.to_vec();
    max.resize(cap, 0);
    let (status, body) = upload(&f, Some(&s), "max.webm", max).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    // Images keep their 10 MB cap.
    let mut img = png();
    img.resize(logbook_server::media::UPLOAD_MAX + 1, 0);
    let (status, _) = upload(&f, Some(&s), "big.png", img).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn video_keys_are_checked() {
    let f = fixture().await;
    let hex = "a".repeat(64);
    for path in [
        format!("/media/{hex}.mov"),
        format!("/media/{hex}.MP4"),
        format!("/media/{hex}.webm"), // valid shape, not uploaded
        format!("/media/{hex}.mp4"),
    ] {
        let (status, _, _) = get_range(&f, &path, "bytes=0-1").await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
    }
}

#[tokio::test]
async fn exports_skip_videos() {
    let f = fixture().await;
    let s = f.session(30).await;
    let key = upload_key(&f, &s, "clip.webm", WEBM.to_vec()).await["key"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(logbook_server::export::media_keys(&format!("```video\n/media/{key}\n```")).is_empty());
    let res = f
        .app
        .clone()
        .oneshot(
            Request::get("/api/owner/export.zip")
                .header(header::COOKIE, format!("logbook_session={s}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let zip = res.into_body().collect().await.unwrap().to_bytes();
    assert!(
        !zip.windows(key.len()).any(|w| w == key.as_bytes()),
        "the zip holds no video"
    );
}
