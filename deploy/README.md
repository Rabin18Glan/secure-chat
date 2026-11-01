# EC2 deployment

The workflow in [deploy.yml](../.github/workflows/deploy.yml) builds the React frontend and Rust backend on GitHub Actions, publishes images to GHCR, and deploys them over SSH to:

```text
ec2-user@ec2-32-236-18-31.ap-southeast-2.compute.amazonaws.com
```

Pushes to `main` deploy automatically. You can also run **Actions → Deploy frontend and backend to EC2 → Run workflow** on `main`. No test jobs are included.

Caddy serves the frontend and forwards `/api/*`, `/ws`, `/ws/*`, `/socket`, and `/ping` to the backend. Frontend and backend share one origin. MongoDB and the backend have no published host ports. Database data and HTTPS certificates live in named Docker volumes.

## 1. Prepare EC2 once

This bootstrap supports Amazon Linux and requires `ec2-user` to have sudo access. From your local repository:

```bash
chmod 600 myserver.pem
scp -i myserver.pem deploy/bootstrap-ec2.sh ec2-user@ec2-32-236-18-31.ap-southeast-2.compute.amazonaws.com:~/bootstrap-secure-chat.sh
ssh -i myserver.pem ec2-user@ec2-32-236-18-31.ap-southeast-2.compute.amazonaws.com 'bash ~/bootstrap-secure-chat.sh'
```

Disconnect and reconnect after bootstrap so Docker group membership takes effect. Confirm `docker info` and `docker compose version` work without sudo.

The EC2 security group must allow TCP ports **80** and **443** publicly, and TCP **22** from your deployment runner and your own SSH client. GitHub-hosted runners use changing source IPs; use a runner with fixed outbound IPs if your SSH rule requires a fixed source. Ports **5000**, **8000**, and **27017** do not need inbound rules. EC2 also needs outbound access to GHCR, Docker Hub, and certificate authorities. Existing services must not occupy ports 80/443.

The default image architecture is `linux/amd64`. Run `uname -m` on EC2; if it reports `aarch64`, set repository variable `EC2_PLATFORM=linux/arm64`. ARM builds use QEMU and take longer.

## 2. Add GitHub secrets

In **Settings → Secrets and variables → Actions**, add repository secrets (or secrets in the `production` environment):

| Secret | Value |
| --- | --- |
| `EC2_SSH_PRIVATE_KEY` | Entire contents of `myserver.pem`, including the BEGIN/END lines. The key must work non-interactively. |
| `EC2_KNOWN_HOSTS` | Verified OpenSSH known-hosts entry for the EC2 hostname. |
| `BACKEND_ENV` | Optional multiline backend environment file, primarily for Google OAuth; see below. |

For `EC2_KNOWN_HOSTS`, collect the public host key:

```bash
ssh-keyscan -t ed25519 ec2-32-236-18-31.ap-southeast-2.compute.amazonaws.com > /tmp/secure-chat-known-hosts
ssh-keygen -lf /tmp/secure-chat-known-hosts
```

Compare that fingerprint with `ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub` on EC2 through a trusted connection before saving the known-hosts entry. The workflow uses strict SSH host-key verification.

No AWS access keys or long-lived GHCR password are needed. The workflow uses GitHub's automatic `GITHUB_TOKEN` to push and pull images. If an existing GHCR package does not inherit repository access, grant this repository Actions access in that package's settings.

Example `BACKEND_ENV` when Google sign-in is configured:

```dotenv
GOOGLE_CLIENT_ID=your-google-client-id
GOOGLE_CLIENT_SECRET=your-google-client-secret
GOOGLE_CALLBACK_URL=https://chat.example.com/
```

Use the same origin and callback URL in the Google OAuth console. Leave `BACKEND_ENV` empty if Google sign-in is not configured. Values follow Docker Compose env-file syntax; single-quote values containing a literal `$`. The deployment controls the database URL/name, listen port, allowed origin, and secure-cookie setting separately.

## 3. Configure the public address

Set repository variable **`APP_DOMAIN`** to your hostname, for example `chat.example.com`, and point its DNS A record to the EC2 public IP. Use an Elastic IP so the address survives instance stops/starts. Caddy obtains and renews the HTTPS certificate automatically. Set only the hostname, without `https://`, a port, or a path.

If `APP_DOMAIN` is empty, the workflow serves HTTP on the supplied EC2 hostname. That is useful for checking deployment, but browser camera/microphone access and Web Crypto require a secure context, so full meeting functionality needs HTTPS.

## 4. Deploy

Commit the application changes and deployment files, then push to `main`. The workflow:

1. Installs locked frontend dependencies and builds the frontend.
2. Builds and publishes both production images.
3. Uploads release configuration over SSH.
4. Pulls images by digest and starts the Compose stack.
5. Waits for healthy containers and checks both `/ping` and the site through Caddy.
6. Restores the prior release if startup or routing checks fail.

Release files live under `~/secure-chat/releases/<commit>-<run>-<attempt>`. `~/secure-chat/current` points to the last successful release. Concurrent deployments are serialized. GHCR credentials are stored temporarily and removed when deployment exits; backend environment files are private to the SSH user.

The stable Compose project name is `secure-chat`, so releases reuse the same `secure-chat_mongo_data` volume. This is a new database named `secure_chat`; existing data from a different Compose project is not imported automatically. Do not use `docker compose down --volumes` unless you intend to erase application data. Back up MongoDB separately.

## Operations

On EC2, inspect the deployed stack without printing environment values:

```bash
cd ~/secure-chat/current
docker compose --env-file release.env -f compose.yml ps
docker compose --env-file release.env -f compose.yml logs --tail=100 backend frontend
```

Deployments restart the backend, which disconnects active calls and clears its in-memory login sessions. Users will need to sign in again. Rollback restores containers and configuration; it does not revert database changes. If the first deployment fails, there is no earlier release to restore.
