#!/usr/bin/env python3
"""Builds the landing page from template.html.

  python3 site/build.py

Writes two files next to this script:
  index.html     the site page; it loads its images from img/ (publish both together)
  artifact.html  the same page as a fragment, for publishing as a Claude artifact
Change VERSION / SIZE below when a new release goes out.
"""
import base64, mimetypes, pathlib, re

HERE = pathlib.Path(__file__).parent
VERSION = "0.14.0"
SIZE = "86 MB"
REPO = "https://github.com/cesarmour/angles"
DOWNLOAD = f"{REPO}/releases/download/v{VERSION}/Angles-mac.zip"
DESCRIPTION = ("Angles is a free CQB teaching simulator for Mac: a shoot house with armed threats, hostages "
               "and a tactical view that measures slicing the pie in degrees and centimetres.")


def data_uri(name: str) -> str:
    raw = (HERE / "img" / name).read_bytes()
    mime = mimetypes.guess_type(name)[0] or "application/octet-stream"
    return f"data:{mime};base64,{base64.b64encode(raw).decode()}"


def main() -> None:
    raw = (HERE / "template.html").read_text()

    def fill(images) -> str:
        t = re.sub(r"\{\{img:([\w.\-]+)\}\}", lambda m: images(m.group(1)), raw)
        for key, val in {"version": VERSION, "size": SIZE, "repo": REPO, "download": DOWNLOAD}.items():
            t = t.replace("{{" + key + "}}", val)
        left = re.findall(r"\{\{[^}]*\}\}", t)
        assert not left, f"unfilled placeholders: {left}"
        return t

    # the artifact is a single file, so its images are embedded; the site loads them as separate files from img/
    # (the page itself stays small and the opening image can load while the rest is still coming)
    (HERE / "artifact.html").write_text("".join(fill(data_uri).split("<!--BODY-->")))
    head, body = fill(lambda name: f"img/{name}").split("<!--BODY-->")

    icon = data_uri("icon.png")
    page = f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<meta name="description" content="{DESCRIPTION}">
<meta property="og:title" content="Angles">
<meta property="og:description" content="{DESCRIPTION}">
<meta property="og:type" content="website">
<link rel="icon" type="image/png" href="{icon}">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<style>body{{margin:0}}img{{max-width:100%}}[hidden]{{display:none!important}}</style>
{head.strip()}
</head>
<body>
{body.strip()}
</body>
</html>
"""
    (HERE / "index.html").write_text(page)
    for f in ("index.html", "artifact.html"):
        print(f, f"{(HERE / f).stat().st_size / 1024:.0f} KB")


if __name__ == "__main__":
    main()
