"""Install the pinned official Godot build on a disposable GitHub runner."""

import hashlib
import os
import platform
from pathlib import Path
import shutil
import subprocess
import urllib.request
import zipfile

VERSION = "4.7.2-stable"
TARGETS = {
    "Linux": ("linux.x86_64", "linux.x86_64", "9aa00f7a605200940bce3027a567b782f49bd8e940dd06ae9e987bd65aee1b1467edd56ed84fcdcbdd44354bf613bdbb4e5d2913e925850368e150c59ed54c65"),
    "Darwin": ("macos.universal", "Godot.app/Contents/MacOS/Godot", "38aa16e5bba2083941fc5b3e54be0089bd4cc35e32415f5b9fd9a8a6a7b9818255d44532ea8ef94b5aef56c4b407c2d634fa4f657e4ebe681ebbf59b7bac69ca"),
    "Windows": ("win64.exe", "win64_console.exe", "83decd58fdf67b9d657958a1ae6bf1929c20785315a81effe245874cdc57acb709bf868e00778a96984338c1b29dafdb453c6847747694621c6ecf5da2259993"),
}


def main():
    system = platform.system()
    suffix, entry, expected = TARGETS[system]
    stem = f"Godot_v{VERSION}_{suffix}"
    target = Path(os.environ["RUNNER_TEMP"]) / "airline-godot"
    target.mkdir(exist_ok=True)
    archive = target / f"{stem}.zip"
    url = f"https://github.com/godotengine/godot-builds/releases/download/{VERSION}/{archive.name}"
    with urllib.request.urlopen(url, timeout=90) as source, archive.open("wb") as output:
        shutil.copyfileobj(source, output)
    with archive.open("rb") as content:
        if hashlib.file_digest(content, "sha512").hexdigest() != expected:
            raise SystemExit(f"Checksum mismatch: {archive.name}")
    if system == "Darwin":
        subprocess.run(["ditto", "-x", "-k", archive, target], check=True)
    else:
        with zipfile.ZipFile(archive) as bundle:
            bundle.extractall(target)
    binary = target / (entry if system == "Darwin" else f"Godot_v{VERSION}_{entry}")
    if not binary.is_file():
        raise SystemExit(f"Missing executable: {binary}")
    binary.chmod(0o755)
    with open(os.environ["GITHUB_ENV"], "a", encoding="utf-8") as environment:
        environment.write(f"AIRLINE_GODOT={binary}\n")


if __name__ == "__main__":
    main()
