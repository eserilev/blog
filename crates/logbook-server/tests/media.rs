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
