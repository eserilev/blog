#!/usr/bin/env bash
# Restore test (spec 7.7). Proves the recovery path on every PR:
#   1. Start empty (ALLOW_EMPTY_START=1) and write posts in all states, a passkey, and an image.
#   2. Run `logbook check-backup` (restore to a temp file, integrity, heartbeat age).
#   3. Delete the container and its volume. The bucket is all that is left.
#   4. Start a new container without the flag. Everything must come back.
#   5. An empty bucket without the flag must make the container refuse to start.
set -euo pipefail

cd "$(dirname "$0")"
export COMPOSE_FILE=compose.test.yaml
URL=http://127.0.0.1:18090

fail() { echo "restore-test: FAIL: $*" >&2; docker compose logs --no-color app | tail -40 >&2; exit 1; }
cleanup() { docker compose down -v --remove-orphans >/dev/null 2>&1 || true; }
trap cleanup EXIT

wait_healthy() {
  for _ in $(seq 1 60); do
    curl -fs "$URL/healthz" >/dev/null 2>&1 && return 0
    sleep 1
  done
  fail "the app did not become healthy"
}

db_query() { docker compose exec -T app sqlite3 /data/logbook.db "$1"; }

cleanup
echo "restore-test: build"
docker compose build -q app

echo "restore-test: start S3 and create buckets"
docker compose up -d --wait s3
for b in logbook empty; do
  echo "s3.bucket.create -name $b" | docker compose exec -T s3 weed shell >/dev/null
done

echo "restore-test: first start, empty"
ALLOW_EMPTY_START=1 docker compose up -d app
wait_healthy
docker compose exec -T app logbook seed-sample >/dev/null
# A passkey row, so the test proves that sign-in data survives a restore too.
db_query "INSERT INTO passkeys (id, passkey, label, created_at) VALUES (x'0102', '{\"test\":true}', 'Restore test', '2026-01-01T00:00:00Z')"
before_keys=$(db_query "SELECT hex(id), passkey, label FROM passkeys ORDER BY id")

# An owner session (token hash in the DB) and an image upload to the bucket.
token=restore-test-token-$RANDOM$RANDOM
token_hash=$(printf '%s' "$token" | sha256sum | cut -c1-64)
db_query "INSERT INTO sessions (token_hash, created_at, expires_at) VALUES (x'$token_hash', '2026-01-01T00:00:00Z', '2999-01-01T00:00:00Z')"
echo 'iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAIAAAD91JpzAAAAFklEQVR4nGP8z8DAwMDAxMDAwMDAAAANHQEDasKb6QAAAABJRU5ErkJggg==' | base64 -d > /tmp/restore-test.png
upload=$(curl -fs -X POST "$URL/api/owner/uploads" -H "Origin: http://localhost:18090" -H "Cookie: logbook_session=$token" -F "file=@/tmp/restore-test.png;type=image/png") || fail "image upload failed"
media_key=$(echo "$upload" | sed -n 's/.*"key":"\([0-9a-f]*\.png\)".*/\1/p')
[ -n "$media_key" ] || fail "no media key in: $upload"
media_before=$(curl -fs "$URL/media/$media_key" | sha256sum)
before_api=$(curl -fs "$URL/api/posts")
before_rows=$(db_query "SELECT id, slug, state, version, body_md FROM posts ORDER BY id")
[ "$(db_query "SELECT COUNT(*) FROM posts WHERE state != 'public'")" -ge 2 ] || fail "seed has no hidden posts"

echo "restore-test: wait for replication"
sleep 5

echo "restore-test: backup check"
docker compose exec -T app logbook check-backup | grep -q "^backup ok" || fail "the backup check failed"

echo "restore-test: delete the container and its volume"
docker compose rm -sf app >/dev/null
docker volume rm logbook-test_data >/dev/null

echo "restore-test: second start, restore from the bucket"
docker compose up -d app
wait_healthy
after_api=$(curl -fs "$URL/api/posts")
after_rows=$(db_query "SELECT id, slug, state, version, body_md FROM posts ORDER BY id")
[ "$before_api" = "$after_api" ] || fail "the public API differs after restore"
[ "$before_rows" = "$after_rows" ] || fail "the posts table differs after restore (drafts and private posts included)"
[ "$before_keys" = "$(db_query "SELECT hex(id), passkey, label FROM passkeys ORDER BY id")" ] || fail "passkeys differ after restore"
[ "$(db_query "PRAGMA integrity_check")" = "ok" ] || fail "integrity check"
[ "$(curl -fs "$URL/media/$media_key" | sha256sum)" = "$media_before" ] || fail "the image did not come back from the bucket"
code=$(curl -s -o /dev/null -w '%{http_code}' "$URL/api/posts/epbs-from-a-clients-perspective")
[ "$code" = "404" ] || fail "a draft is visible after restore (HTTP $code)"
docker compose logs --no-color app | grep -q "starting with a new database" && fail "second start created a new database"

echo "restore-test: empty bucket without ALLOW_EMPTY_START must refuse"
docker compose rm -sf app >/dev/null
docker volume rm logbook-test_data >/dev/null
set +e
out=$(S3_BUCKET=empty docker compose run --rm --no-deps -T app 2>&1)
status=$?
set -e
[ "$status" -ne 0 ] || fail "the app started on an empty bucket"
echo "$out" | grep -q "refusing to start empty" || fail "missing refusal message: $out"

echo "restore-test: PASS"
