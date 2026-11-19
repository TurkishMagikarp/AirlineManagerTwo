"""Mint a one-hour GitHub App token. Run with the App key outside agent mounts."""

import argparse
from base64 import urlsafe_b64encode
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
from urllib.request import Request, urlopen


def b64(content):
    return urlsafe_b64encode(content).rstrip(b"=")


def mint(app, installation, key, repo):
    now = int(time.time())
    header = b64(b'{"alg":"RS256","typ":"JWT"}')
    claims = b64(json.dumps({"iat": now - 60, "exp": now + 540, "iss": app}).encode())
    message = b".".join((header, claims))
    signature = subprocess.run(["openssl", "dgst", "-sha256", "-sign", key],
                               input=message, stdout=subprocess.PIPE, check=True).stdout
    jwt = b".".join((message, b64(signature))).decode()
    body = json.dumps({"repositories": [repo], "permissions": {
        "contents": "write", "pull_requests": "write", "actions": "read",
    }}).encode()
    request = Request(f"https://api.github.com/app/installations/{installation}/access_tokens",
                      data=body, headers={"Authorization": f"Bearer {jwt}",
                                          "Accept": "application/vnd.github+json",
                                          "Content-Type": "application/json",
                                          "X-GitHub-Api-Version": "2022-11-28",
                                          "User-Agent": "airline-delivery"}, method="POST")
    with urlopen(request, timeout=25) as response:
        return json.load(response)["token"]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--config", type=Path, required=True)
    args = p.parse_args()
    cfg = json.loads(args.config.read_text())
    dest = Path(cfg["github_token"])
    # The key is read as root. Only the short-lived token is shared with the agent UID.
    token = mint(cfg["app_id"], cfg["installation_id"], cfg["private_key"], cfg["repo"].split("/")[-1])
    # Use a root-owned staging directory on the same filesystem (outside the agent bind mount).
    with tempfile.NamedTemporaryFile(mode="w", dir=cfg["token_staging"], delete=False) as output:
        output.write(token + "\n")
        temp = Path(output.name)
    try:
        os.chown(temp, cfg["agent_uid"], cfg["agent_gid"])
        os.replace(temp, dest)
    finally:
        temp.unlink(missing_ok=True)


if __name__ == "__main__":
    main()
