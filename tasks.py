import re
import shutil
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

    return "0.0.0"


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
    ctx.run(
        f'python "{generator}" "{ROOT}/Cargo.lock" -o "{DIST}/generated-sources.json"', hide=True)


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
        output = ctx.run(
            f"cargo run -- schema --format yaml {command}", hide=True)

        with doc.open("w") as f:
            f.write(output.stdout.strip() + "\n")


@task
def prerelease(ctx, update_lang=True):
    clean(ctx)
    legal(ctx)
    flatpak(ctx)
    docs(ctx)
    if update_lang:
        lang(ctx)


@task
def release(ctx):
    version = get_version()
    ctx.run(f'git commit -m "Release v{version}"')
    ctx.run(f'git tag v{version} -m "Release"')
    ctx.run("git push")
    ctx.run("git push --tags")


@task
def release_flatpak(ctx, target="/git/com.github.mtkennerly.ludusavi"):
    target = Path(target)
    spec = target / "com.github.mtkennerly.ludusavi.yaml"
    version = get_version()

    with ctx.cd(target):
        ctx.run("git checkout master")
        ctx.run("git pull")
        ctx.run(f"git checkout -b release/v{version}")

        shutil.copy(DIST / "generated-sources.json",
                    target / "generated-sources.json")
        spec_content = spec.read_bytes().decode("utf-8")
        spec_content = re.sub(r"(        tag:) (.*)",
                              fr"\1 v{version}", spec_content)
        spec.write_bytes(spec_content.encode("utf-8"))

        ctx.run("git add .")
        ctx.run(f'git commit -m "Update for v{version}"')
        ctx.run("git push origin HEAD")


@task
def release_winget(ctx, target="/git/_forks/winget-pkgs"):
    target = Path(target)
    version = get_version()

    with ctx.cd(target):
        ctx.run("git checkout master")
        ctx.run("git pull upstream master")
        ctx.run(f"git checkout -b mtkennerly.ludusavi-{version}")
        ctx.run(
            f"wingetcreate update mtkennerly.ludusavi --version {version} --urls https://github.com/mtkennerly/ludusavi/releases/download/v{version}/ludusavi-v{version}-win64.zip https://github.com/mtkennerly/ludusavi/releases/download/v{version}/ludusavi-v{version}-win32.zip")
        ctx.run(
            f"code --wait manifests/m/mtkennerly/ludusavi/{version}/mtkennerly.ludusavi.locale.en-US.yaml")
        ctx.run(
            f"winget validate --manifest manifests/m/mtkennerly/ludusavi/{version}")
        ctx.run("git add .")
        ctx.run(f'git commit -m "mtkennerly.ludusavi version {version}"')
        ctx.run("git push origin HEAD")
