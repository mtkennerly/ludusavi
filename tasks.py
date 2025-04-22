import datetime as dt
import re
import shutil
import sys
import textwrap
import zipfile
from pathlib import Path

from invoke import task

ROOT = Path(__file__).parent
DIST = ROOT / "dist"
LANG = ROOT / "lang"


def get_version() -> str:
    for line in (ROOT / "Cargo.toml").read_text("utf-8").splitlines():
        if line.startswith("version ="):
            return line.replace("version = ", "").strip('"')

    raise RuntimeError("Could not determine version")


def replace_pattern_in_file(file: Path, old: str, new: str, count: int = 1):
    content = file.read_text("utf-8")
    updated = re.sub(old, new, content, count=count)
    file.write_text(updated, "utf-8")


def confirm(prompt: str):
    response = input(f"Confirm by typing '{prompt}': ")
    if response.lower() != prompt.lower():
        sys.exit(1)


@task
def version(ctx):
    print(get_version())


@task
def legal(ctx):
    version = get_version()
    txt_name = f"ludusavi-v{version}-legal.txt"
    txt_path = ROOT / "dist" / txt_name
    try:
        ctx.run(f'cargo lichking bundle --file "{txt_path}"', hide=True)
    except Exception:
        pass
    raw = txt_path.read_text("utf8")
    normalized = re.sub(r"C:\\Users\\[^\\]+", "~", raw)
    txt_path.write_text(normalized, "utf8")

    zip_path = ROOT / "dist" / f"ludusavi-v{version}-legal.zip"
    with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as zip:
        zip.write(txt_path, txt_name)


@task
def flatpak(ctx, generator="/opt/flatpak-cargo-generator.py"):
    ctx.run(f'python "{generator}" "{ROOT}/Cargo.lock" -o "{DIST}/generated-sources.json"', hide=True)


@task
def lang(ctx, jar="/opt/crowdin-cli/crowdin-cli.jar"):
    ctx.run(f'java -jar "{jar}" pull --export-only-approved')

    mapping = {}
    for file in LANG.glob("*.ftl"):
        if "en-US.ftl" in file.name:
            continue
        content = file.read_text("utf8")
        if content not in mapping:
            mapping[content] = set()
        mapping[content].add(file)

    for group in mapping.values():
        if len(group) > 1:
            for file in group:
                file.unlink()


@task
def clean(ctx):
    if DIST.exists():
        shutil.rmtree(DIST, ignore_errors=True)
    DIST.mkdir()


@task
def docs(ctx):
    docs_cli(ctx)
    docs_schema(ctx)


@task
def docs_cli(ctx):
    docs = Path(__file__).parent / "docs"
    if not docs.exists():
        docs.mkdir(parents=True)
    doc = docs / "cli.md"

    commands = [
        "--help",
        "backup --help",
        "restore --help",
        "complete --help",
        "backups --help",
        "find --help",
        "manifest --help",
        "cloud --help",
        "wrap --help",
        "api --help",
        "schema --help",
    ]

    lines = [
        "This is the raw help text for the command line interface.",
    ]
    for command in commands:
        print(f"cli.md: {command}")
        output = ctx.run(f"cargo run -- {command}", hide=True)
        lines.append("")
        lines.append(f"## `{command}`")
        lines.append("```")
        for line in output.stdout.splitlines():
            lines.append(line.rstrip())
        lines.append("```")

    with doc.open("w") as f:
        for line in lines:
            f.write(line + "\n")


@task
def docs_schema(ctx):
    docs = Path(__file__).parent / "docs" / "schema"
    if not docs.exists():
        docs.mkdir(parents=True)

    commands = [
        "api-input",
        "api-output",
        "config",
        "general-output",
    ]

    for command in commands:
        doc = docs / f"{command}.yaml"
        print(f"schema: {command}")
        output = ctx.run(f"cargo run -- schema --format yaml {command}", hide=True)

        with doc.open("w") as f:
            f.write(output.stdout.strip() + "\n")


@task
def prerelease(ctx, new_version, update_lang=True):
    date = dt.datetime.now().strftime("%Y-%m-%d")

    replace_pattern_in_file(
        ROOT / "Cargo.toml",
        'version = ".+"',
        f'version = "{new_version}"',
    )

    replace_pattern_in_file(
        ROOT / "CHANGELOG.md",
        "## Unreleased",
        f"## v{new_version} ({date})",
    )

    replace_pattern_in_file(
        ROOT / ".github/ISSUE_TEMPLATE/bug.yaml",
        r"(options:)(\n        - v\d+\.\d+\.\d+)",
        fr"\g<1>\n        - v{new_version}\g<2>",
    )

    replace_pattern_in_file(
        ROOT / ".github/ISSUE_TEMPLATE/bug.yaml",
        r"- v\d+\.\d+\.\d+\n        (- Other)",
        fr"\g<1>",
    )

    for metainfo in [ROOT / "assets/linux/com.mtkennerly.ludusavi.metainfo.xml", ROOT / "assets/flatpak/com.github.mtkennerly.ludusavi.metainfo.xml"]:
        replace_pattern_in_file(
            metainfo,
            "(ludusavi/v).+(/docs/sample-gui-linux.png)",
            fr"\g<1>{new_version}\g<2>",
        )

        replace_pattern_in_file(
            metainfo,
            "<releases>",
            f'<releases>\n        <release version="{new_version}" date="{date}"/>',
        )

    # Update version in Cargo.lock
    ctx.run("cargo build")

    clean(ctx)
    legal(ctx)
    flatpak(ctx)
    docs(ctx)
    if update_lang:
        lang(ctx)


@task
def release(ctx):
    version = get_version()

    confirm(f"release {version}")

    ctx.run(f'git commit -m "Release v{version}"')
    ctx.run(f'git tag v{version} -m "Release"')
    ctx.run("git push")
    ctx.run(f"git push origin tag v{version}")


@task
def release_flatpak(ctx, target="/git/com.github.mtkennerly.ludusavi"):
    target = Path(target)
    spec = target / "com.github.mtkennerly.ludusavi.yaml"
    version = get_version()

    with ctx.cd(target):
        ctx.run("git checkout master")
        ctx.run("git pull")
        ctx.run(f"git checkout -b release/v{version}")

        shutil.copy(DIST / "generated-sources.json", target / "generated-sources.json")
        spec_content = spec.read_bytes().decode("utf-8")
        spec_content = re.sub(r"(        tag:) (.*)", fr"\1 v{version}", spec_content)
        spec.write_bytes(spec_content.encode("utf-8"))

        ctx.run("git add .")
        ctx.run(f'git commit -m "Update for v{version}"')
        ctx.run("git push origin HEAD")


@task
def release_winget(ctx, target="/git/_forks/winget-pkgs"):
    target = Path(target)
    version = get_version()
    changelog = textwrap.indent(latest_changelog(), "  ")

    with ctx.cd(target):
        ctx.run("git checkout master")
        ctx.run("git pull upstream master")
        ctx.run(f"git checkout -b mtkennerly.ludusavi-{version}")
        ctx.run(f"wingetcreate update mtkennerly.ludusavi --version {version} --urls https://github.com/mtkennerly/ludusavi/releases/download/v{version}/ludusavi-v{version}-win64.zip https://github.com/mtkennerly/ludusavi/releases/download/v{version}/ludusavi-v{version}-win32.zip")

        spec = target / f"manifests/m/mtkennerly/ludusavi/{version}/mtkennerly.ludusavi.locale.en-US.yaml"
        spec_content = spec.read_bytes().decode("utf-8")
        spec_content = spec_content.replace("Moniker: ludusavi", f"Moniker: ludusavi\nReleaseNotes: |-\n{changelog}\nReleaseNotesUrl: https://github.com/mtkennerly/ludusavi/releases/tag/v{version}")
        spec.write_bytes(spec_content.encode("utf-8"))

        ctx.run(f"winget validate --manifest manifests/m/mtkennerly/ludusavi/{version}")
        ctx.run("git add .")
        ctx.run(f'git commit -m "mtkennerly.ludusavi version {version}"')
        ctx.run("git push origin HEAD")


def latest_changelog() -> str:
    changelog = ROOT / "CHANGELOG.md"
    content = changelog.read_bytes().decode("utf-8")

    lines = []
    header = False
    for line in content.splitlines():
        if line.startswith("#"):
            if header:
                break
            header = True
            continue

        lines.append(line)

    return "\n".join(lines).strip()
