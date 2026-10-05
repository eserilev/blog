# Deploy runbook

The blog runs on the sandcastle VPS, in `/srv/logbook`. The sandcastle Caddy serves it. The bucket holds all data. The server disk holds nothing that cannot be rebuilt. Spec: 6.11 to 6.13.

| File | Use |
|---|---|
| `Dockerfile` | The image: the server, the WASM preview, litestream. |
| `entrypoint.sh` | Restore, then replicate (spec 6.12). |
| `litestream.yml` | The replica in `s3://<bucket>/db`. |
| `compose.yaml` | Production. CI copies it to `/srv/logbook`. |
| `logbook.caddy` | The site for the sandcastle Caddy. |
| `.env.example`, `prod.env.example` | The text of the `STACK_ENV` and `PROD_ENV` secrets. |
| `compose.test.yaml`, `restore-test.sh` | The restore test (spec 7.7). |

## One-time setup

Do these steps in this order.

1. Sandcastle: merge the three edits of spec 6.11. Deploy sandcastle. Its Caddy is then on the `edge` network.
2. Hetzner: make a new project for the blog. Do not use the sandcastle project.
3. In that project, make the bucket `logbook`. Turn on versioning and object lock (governance, 35 days) at creation. Object lock cannot be added later.
4. Add a lifecycle rule: expire noncurrent versions after 40 days.
5. Make an S3 key pair in the blog project.
6. Point `unclebill.blog` (A record) at the VPS IP.
7. Fill `.env.example` and `prod.env.example` with real values.
8. Store both texts in a password manager. `PROD_ENV` plus the bucket is the whole blog.
9. GitHub, blog repo: make the Environment `production`. Allow only the `master` branch. (Done.)
10. In that Environment, add the secrets `STACK_ENV`, `PROD_ENV`, `VPS_HOST`, `VPS_USER`, `VPS_SSH_KEY`, and `VPS_FINGERPRINT`. The `VPS_*` values are the same as in sandcastle.
11. Optional: make a check at healthchecks.io with a 1-day period. Put its URL in `HEALTHCHECK_URL`.
12. After the first deploy: add the repo variable `DEPLOY_ENABLED=true`. Without it, a push to `master` does not deploy.

## First deploy

1. Actions → **deploy** → Run workflow. Leave the SHA empty. Turn on `allow_empty_start`.
2. When the job is green, open `https://unclebill.blog`.
3. On the VPS, make a setup link:

   ```sh
   cd /srv/logbook && docker compose exec app logbook setup-link
   ```

4. Open the link within 15 minutes. Register a passkey.
5. Wait 5 minutes. Then run the backup check (below). It must print `backup ok`.

Later deploys never use `allow_empty_start`. Without it, an empty or wrong bucket stops the container. It never starts an empty site.

## Normal deploys

A merge to `master` starts **deploy** at once. It builds the image, tags it `sha-<commit>` and `prod`, and swaps the app. Then it waits for `https://unclebill.blog/healthz`. The PR passed CI before the merge, so the deploy does not wait for CI on `master`.

## Rollback

1. Find the last good commit SHA on `master`.
2. Actions → **deploy** → Run workflow. Put the full SHA in the input.

CAUTION: Do not roll back over a migration with this procedure. The old image refuses the newer schema. Do a point-in-time restore to before the migration instead (spec 6.12).

## Checks

- `/healthz` fails (503) if the database does not answer, or if the newest replica object is more than 1 hour old.
- Every night the app restores the replica to a temp file and checks it. Then it pings `HEALTHCHECK_URL`.
- Run the same check by hand:

  ```sh
  cd /srv/logbook && docker compose exec app logbook check-backup
  ```

## Lost passkey

Make a new setup link on the VPS (First deploy, step 3). Then remove the old passkey on the site.

## The VPS is dead

Follow "Recovery" in spec 6.12. The blog needs no file from the old server: the secrets and the bucket are enough.

## Logs

```sh
cd /srv/logbook && docker compose logs --tail 100 app
```
