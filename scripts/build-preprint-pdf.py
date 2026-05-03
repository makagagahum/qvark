#!/usr/bin/env python3
"""Build a simple PDF manuscript from a Qorx Markdown paper.

This script uses only the Python standard library so the preprint package can be
rebuilt on a clean machine without Pandoc, LaTeX, or browser tooling.
"""

from __future__ import annotations

import argparse
import re
import textwrap
import unicodedata
from pathlib import Path


PAGE_WIDTH = 612.0
PAGE_HEIGHT = 792.0
MARGIN = 54.0
TOP = PAGE_HEIGHT - MARGIN
BOTTOM = MARGIN


def normalize_text(value: str) -> str:
    replacements = {
        "\u2013": "-",
        "\u2014": "-",
        "\u2018": "'",
        "\u2019": "'",
        "\u201c": '"',
        "\u201d": '"',
        "\u00a0": " ",
    }
    for old, new in replacements.items():
        value = value.replace(old, new)
    value = unicodedata.normalize("NFKC", value)
    return value.encode("latin-1", "replace").decode("latin-1")


def clean_inline(value: str) -> str:
    value = normalize_text(value.strip())
    value = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", value)
    value = value.replace("**", "")
    value = value.replace("__", "")
    value = value.replace("`", "")
    return value


def pdf_literal(value: str) -> str:
    value = normalize_text(value)
    value = value.replace("\\", "\\\\")
    value = value.replace("(", "\\(")
    value = value.replace(")", "\\)")
    return f"({value})"


def markdown_lines(markdown: str) -> list[tuple[str, str]]:
    lines: list[tuple[str, str]] = []
    in_code = False

    for raw in markdown.splitlines():
        line = raw.rstrip()
        if line.strip().startswith("```"):
            in_code = not in_code
            if not in_code:
                lines.append(("blank", ""))
            continue

        if in_code:
            lines.append(("code", normalize_text(line)))
            continue

        if not line.strip():
            lines.append(("blank", ""))
            continue

        if line.startswith("#"):
            level = min(len(line) - len(line.lstrip("#")), 3)
            lines.append((f"h{level}", clean_inline(line[level:].strip())))
            continue

        if line.startswith("|") and line.endswith("|"):
            parts = [part.strip() for part in line.strip("|").split("|")]
            if all(re.fullmatch(r":?-{3,}:?", part) for part in parts):
                continue
            lines.append(("body", " | ".join(clean_inline(part) for part in parts)))
            continue

        stripped = line.lstrip()
        if stripped.startswith("- "):
            lines.append(("body", "- " + clean_inline(stripped[2:])))
        elif re.match(r"^\d+\.\s+", stripped):
            lines.append(("body", clean_inline(stripped)))
        elif stripped.startswith(">"):
            lines.append(("body", clean_inline(stripped.lstrip("> "))))
        else:
            lines.append(("body", clean_inline(line)))

    return lines


def style_for(kind: str) -> tuple[str, float, float, float]:
    if kind == "h1":
        return "F2", 17.0, 23.0, 0.46
    if kind == "h2":
        return "F2", 14.0, 20.0, 0.48
    if kind == "h3":
        return "F2", 12.0, 17.0, 0.50
    if kind == "code":
        return "F3", 8.5, 12.0, 0.60
    return "F1", 10.5, 15.0, 0.52


def wrap_for_style(text: str, kind: str, font_size: float, width_factor: float) -> list[str]:
    available = PAGE_WIDTH - (2 * MARGIN)
    if kind == "code":
        available -= 12
    max_chars = max(35, int(available / (font_size * width_factor)))
    return textwrap.wrap(
        text,
        width=max_chars,
        break_long_words=True,
        break_on_hyphens=False,
        replace_whitespace=False,
    ) or [""]


def render_pages(lines: list[tuple[str, str]]) -> list[bytes]:
    pages: list[list[str]] = [[]]
    y = TOP

    def new_page() -> None:
        nonlocal y
        pages.append([])
        y = TOP

    def add_command(kind: str, text: str) -> None:
        nonlocal y
        font, size, leading, width_factor = style_for(kind)
        if kind.startswith("h") and y < TOP - 8:
            y -= 8
        if y - leading < BOTTOM:
            new_page()

        x = MARGIN + (12 if kind == "code" else 0)
        for part in wrap_for_style(text, kind, size, width_factor):
            if y - leading < BOTTOM:
                new_page()
            pages[-1].append(
                f"BT /{font} {size:.1f} Tf 1 0 0 1 {x:.1f} {y:.1f} Tm {pdf_literal(part)} Tj ET\n"
            )
            y -= leading

        if kind.startswith("h"):
            y -= 2

    for kind, text in lines:
        if kind == "blank":
            y -= 8
            if y < BOTTOM:
                new_page()
            continue
        add_command(kind, text)

    return ["".join(page).encode("latin-1", "replace") for page in pages if page]


def make_pdf(page_streams: list[bytes], title: str, author: str) -> bytes:
    objects: dict[int, bytes] = {}
    page_ids: list[int] = []
    next_id = 7

    objects[1] = b"<< /Type /Catalog /Pages 2 0 R >>"
    objects[3] = b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"
    objects[4] = b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>"
    objects[5] = b"<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>"
    objects[6] = (
        "<< /Title {title} /Author {author} /Subject {subject} "
        "/Keywords {keywords} /Creator {creator} /Producer {producer} >>"
    ).format(
        title=pdf_literal(title),
        author=pdf_literal(author),
        subject=pdf_literal("Qorx Local Context Resolution preprint"),
        keywords=pdf_literal("Qorx, local context resolution, AI context, Rust"),
        creator=pdf_literal("scripts/build-preprint-pdf.py"),
        producer=pdf_literal("Qorx dependency-free PDF builder"),
    ).encode("latin-1", "replace")

    for stream in page_streams:
        content_id = next_id
        page_id = next_id + 1
        next_id += 2

        objects[content_id] = (
            f"<< /Length {len(stream)} >>\nstream\n".encode("latin-1")
            + stream
            + b"\nendstream"
        )
        objects[page_id] = (
            f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_WIDTH:.0f} {PAGE_HEIGHT:.0f}] "
            f"/Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R >> >> "
            f"/Contents {content_id} 0 R >>"
        ).encode("latin-1")
        page_ids.append(page_id)

    kids = " ".join(f"{page_id} 0 R" for page_id in page_ids)
    objects[2] = f"<< /Type /Pages /Kids [{kids}] /Count {len(page_ids)} >>".encode("latin-1")

    output = bytearray(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")
    offsets = [0]
    for object_id in sorted(objects):
        offsets.append(len(output))
        output.extend(f"{object_id} 0 obj\n".encode("latin-1"))
        output.extend(objects[object_id])
        output.extend(b"\nendobj\n")

    xref_at = len(output)
    max_id = max(objects)
    output.extend(f"xref\n0 {max_id + 1}\n".encode("latin-1"))
    output.extend(b"0000000000 65535 f \n")
    offset_by_id = {object_id: offset for object_id, offset in zip(sorted(objects), offsets[1:])}
    for object_id in range(1, max_id + 1):
        output.extend(f"{offset_by_id.get(object_id, 0):010d} 00000 n \n".encode("latin-1"))
    output.extend(
        f"trailer\n<< /Size {max_id + 1} /Root 1 0 R /Info 6 0 R >>\nstartxref\n{xref_at}\n%%EOF\n".encode(
            "latin-1"
        )
    )
    return bytes(output)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--input",
        default="docs/papers/qorx-local-context-resolution-preprint.md",
        help="Markdown manuscript path.",
    )
    parser.add_argument(
        "--output",
        default="docs/papers/dist/qorx-local-context-resolution-preprint.pdf",
        help="Output PDF path.",
    )
    parser.add_argument(
        "--title",
        default="Qorx Local Context Resolution: A Handle-Resolved Runtime Model for Local AI Context",
    )
    parser.add_argument("--author", default="Marvin Sarreal Villanueva")
    args = parser.parse_args()

    source = Path(args.input)
    output = Path(args.output)
    markdown = source.read_text(encoding="utf-8")
    pages = render_pages(markdown_lines(markdown))
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(make_pdf(pages, args.title, args.author))
    print(f"Wrote {output} ({output.stat().st_size} bytes, {len(pages)} pages)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
