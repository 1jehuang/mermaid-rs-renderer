#!/usr/bin/env python3
import argparse
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Iterable

try:
    from PIL import Image, ImageChops, ImageStat
except ImportError:  # pragma: no cover
    print("Pillow is required: pip install pillow", file=sys.stderr)
    sys.exit(2)

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_FIXTURES = ROOT / "tests" / "fixtures"
DEFAULT_CONFIG = DEFAULT_FIXTURES / "modern-config.json"


def run(cmd):
    return subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)


def relative_path(file: Path, base_dir: Path) -> Path:
    try:
        return file.relative_to(base_dir)
    except ValueError:
        return Path(file.name)


def file_key(rel: Path) -> str:
    rel_no_ext = rel.with_suffix("")
    parts = [part.replace(" ", "_") for part in rel_no_ext.parts]
    return "__".join(parts)


def pick_rust_binary() -> Path:
    primary = ROOT / "target" / "release" / "mmdr"
    fallback = ROOT / "target" / "release" / "mermaid-rs-renderer"
    if primary.exists():
        return primary
    if fallback.exists():
        return fallback
    return primary


def render_rust(
    input_path: Path,
    output_path: Path,
    config_path: Path | None,
    layout_out: Path | None = None,
):
    bin_path = pick_rust_binary()
    if not bin_path.exists():
        print("Building release binary...", file=sys.stderr)
        if bin_path.name == "mmdr":
            res = run(["cargo", "build", "--release", "--bin", "mmdr"])
        else:
            res = run(["cargo", "build", "--release"])
        if res.returncode != 0:
            print(res.stderr, file=sys.stderr)
            raise RuntimeError("cargo build failed")
        bin_path = pick_rust_binary()
    cmd = [str(bin_path), "-i", str(input_path), "-o", str(output_path), "-e", "png"]
    if config_path and config_path.exists():
        cmd.extend(["-c", str(config_path)])
    if layout_out is not None:
        cmd.extend(["--dumpLayout", str(layout_out)])
    res = run(cmd)
    if res.returncode != 0:
        print(res.stderr, file=sys.stderr)
        raise RuntimeError("Rust render failed")


def render_mmdc(input_path: Path, output_path: Path, config_path: Path | None):
    cmd = ["npx", "-y", "@mermaid-js/mermaid-cli", "-i", str(input_path), "-o", str(output_path)]
    if config_path and config_path.exists():
        cmd.extend(["-c", str(config_path)])

    puppeteer_cfg = os.environ.get("MMD_PUPPETEER_CONFIG")
    if puppeteer_cfg:
        cmd.extend(["-p", puppeteer_cfg])

    res = run(cmd)
    if res.returncode != 0:
        print(res.stderr, file=sys.stderr)
        raise RuntimeError("mmdc render failed")


def short_error(err: str, limit: int = 180) -> str:
    cleaned = " ".join(err.split())
    if len(cleaned) <= limit:
        return cleaned
    return cleaned[: limit - 1] + "…"


def crop_to_content(img: Image.Image, padding: int = 12) -> Image.Image:
    if img.mode != "RGBA":
        img = img.convert("RGBA")
    bg_color = img.getpixel((0, 0))
    background = Image.new("RGBA", img.size, bg_color)
    diff = ImageChops.difference(img, background)
    bbox = diff.getbbox()
    if not bbox:
        return img
    left, top, right, bottom = bbox
    left = max(0, left - padding)
    top = max(0, top - padding)
    right = min(img.width, right + padding)
    bottom = min(img.height, bottom + padding)
    return img.crop((left, top, right, bottom))


def normalize_for_diff(path: Path) -> Image.Image:
    img = Image.open(path)
    return crop_to_content(img)


def pad_to_max(img_a: Image.Image, img_b: Image.Image):
    max_w = max(img_a.width, img_b.width)
    max_h = max(img_a.height, img_b.height)
    bg = (255, 255, 255, 255)

    def pad(img):
        if img.mode != "RGBA":
            img = img.convert("RGBA")
        canvas = Image.new("RGBA", (max_w, max_h), bg)
        canvas.paste(img, (0, 0))
        return canvas

    return pad(img_a), pad(img_b)


def diff_images(path_a: Path, path_b: Path):
    img_a = normalize_for_diff(path_a)
    img_b = normalize_for_diff(path_b)
    img_a, img_b = pad_to_max(img_a, img_b)
    diff = ImageChops.difference(img_a, img_b)
    stat = ImageStat.Stat(diff)
    mean = sum(stat.mean) / len(stat.mean)
    rms = sum(stat.rms) / len(stat.rms)
    return mean, rms


def save_diff_images(path_a: Path, path_b: Path, diff_out: Path, side_out: Path, scale: float):
    img_a = normalize_for_diff(path_a)
    img_b = normalize_for_diff(path_b)
    img_a, img_b = pad_to_max(img_a, img_b)
    diff = ImageChops.difference(img_a, img_b)
    if scale and scale != 1.0:
        diff = diff.point(lambda p: min(255, int(p * scale)))
    diff.save(diff_out)

    gap = 12
    canvas = Image.new(
        "RGBA",
        (img_a.width + img_b.width + gap, max(img_a.height, img_b.height)),
        (255, 255, 255, 255),
    )
    canvas.paste(img_a, (0, 0))
    canvas.paste(img_b, (img_a.width + gap, 0))
    canvas.save(side_out)


def write_html_report(out_dir: Path, rows: Iterable[dict]):
    def fmt_metric(value):
        if isinstance(value, (int, float)):
            return f"{value:.2f}"
        return "n/a"

    def img_cell(name: str):
        if not name:
            return "<span class=\"muted\">n/a</span>"
        return f"<img src=\"{name}\" />"

    rows_html = []
    for row in rows:
        rows_html.append(
            f"""
            <tr>
                <td>{row['name']}</td>
                <td class=\"status\">{row.get('status', '')}</td>
                <td>{fmt_metric(row.get('mean'))}</td>
                <td>{fmt_metric(row.get('rms'))}</td>
                <td>{img_cell(row.get('rust', ''))}</td>
                <td>{img_cell(row.get('mmdc', ''))}</td>
                <td>{img_cell(row.get('diff', ''))}</td>
                <td>{img_cell(row.get('side', ''))}</td>
            </tr>
            """
        )

    html = f"""
<!doctype html>
<html>
<head>
<meta charset=\"utf-8\" />
<title>Mermaid Render Conformance Report</title>
<style>
body {{ font-family: system-ui, -apple-system, sans-serif; margin: 24px; color: #222; }}
img {{ max-width: 280px; height: auto; border: 1px solid #ddd; background: #fff; }}
code {{ background: #f4f4f4; padding: 2px 6px; border-radius: 4px; }}
.table-wrap {{ overflow-x: auto; }}
table {{ border-collapse: collapse; min-width: 1200px; }}
th, td {{ border-bottom: 1px solid #eee; padding: 8px 10px; vertical-align: top; text-align: left; }}
th {{ position: sticky; top: 0; background: #fafafa; }}
.muted {{ color: #777; }}
.status {{ max-width: 320px; }}
</style>
</head>
<body>
<h1>Mermaid Render Conformance Report</h1>
<p>Generated by <code>scripts/conformance_compare.py</code>.</p>
<div class=\"table-wrap\">
<table>
<thead>
<tr>
<th>Fixture</th>
<th>Status</th>
<th>Mean</th>
<th>RMS</th>
<th>mmdr</th>
<th>mmdc</th>
<th>diff</th>
<th>side-by-side</th>
</tr>
</thead>
<tbody>
{''.join(rows_html)}
</tbody>
</table>
</div>
</body>
</html>
"""
    (out_dir / "report.html").write_text(html, encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description="Compare Rust renderer output vs mermaid-cli")
    parser.add_argument(
        "--input",
        type=str,
        default=str(DEFAULT_FIXTURES),
        help=".mmd file or directory",
    )
    parser.add_argument(
        "--config",
        type=str,
        default=str(DEFAULT_CONFIG),
        help="mmdc config file",
    )
    parser.add_argument(
        "--strict", action="store_true", help="Fail if diff exceeds thresholds"
    )
    parser.add_argument(
        "--mean-threshold", type=float, default=5.0, help="Mean diff threshold"
    )
    parser.add_argument(
        "--rms-threshold", type=float, default=8.0, help="RMS diff threshold"
    )
    parser.add_argument(
        "--output-dir",
        type=str,
        default="",
        help="Directory to store render artifacts and report",
    )
    parser.add_argument(
        "--diff-scale", type=float, default=4.0, help="Scale factor for diff visualization"
    )
    parser.add_argument(
        "--layout-diff",
        action="store_true",
        help="Generate layout diff report using SVG + layout JSON",
    )
    args = parser.parse_args()

    input_path = Path(args.input)
    if input_path.is_dir():
        files = sorted(input_path.glob("**/*.mmd"))
        base_dir = input_path
    else:
        files = [input_path]
        base_dir = input_path.parent

    if not files:
        print("No .mmd fixtures found.", file=sys.stderr)
        return 1

    failed = False
    rows = []
    output_dir = Path(args.output_dir) if args.output_dir else None

    if output_dir:
        output_dir.mkdir(parents=True, exist_ok=True)
        work_dir = output_dir
    else:
        work_dir = Path(tempfile.mkdtemp())

    for file in files:
        rel = relative_path(file, base_dir)
        key = file_key(rel)
        display_name = str(rel)
        rust_out = work_dir / f"{key}-rust.png"
        mmdc_out = work_dir / f"{key}-mmdc.png"
        diff_out = work_dir / f"{key}-diff.png"
        side_out = work_dir / f"{key}-side.png"
        layout_out = work_dir / f"{key}-layout.json"
        mmdc_svg = work_dir / f"{key}-mmdc.svg"
        layout_report = work_dir / f"{key}-layout-report.json"
        print(f"Comparing {display_name}...")
        for path in (rust_out, mmdc_out, diff_out, side_out, layout_out, mmdc_svg, layout_report):
            if path.exists():
                path.unlink()

        rust_error = None
        mmdc_error = None
        layout_error = None
        rust_ok = False
        mmdc_ok = False
        mean = None
        rms = None

        try:
            render_rust(file, rust_out, Path(args.config), layout_out if args.layout_diff else None)
            rust_ok = True
        except Exception as err:  # pragma: no cover - exercised via script runs
            rust_error = short_error(str(err))
            print(f"  mmdr failed: {rust_error}", file=sys.stderr)

        try:
            render_mmdc(file, mmdc_out, Path(args.config))
            mmdc_ok = True
        except Exception as err:  # pragma: no cover - exercised via script runs
            mmdc_error = short_error(str(err))
            print(f"  mmdc failed: {mmdc_error}", file=sys.stderr)

        if args.layout_diff and rust_ok and mmdc_ok:
            try:
                render_mmdc(file, mmdc_svg, Path(args.config))
                diff_cmd = [
                    sys.executable,
                    str(ROOT / "scripts" / "layout_diff.py"),
                    "--mmdr-layout",
                    str(layout_out),
                    "--mermaid-svg",
                    str(mmdc_svg),
                    "--output",
                    str(layout_report),
                ]
                diff_res = run(diff_cmd)
                if diff_res.returncode != 0:
                    layout_error = short_error(diff_res.stderr)
                    print(diff_res.stderr, file=sys.stderr)
                else:
                    print(diff_res.stdout)
            except Exception as err:  # pragma: no cover - exercised via script runs
                layout_error = short_error(str(err))
                print(f"  layout diff failed: {layout_error}", file=sys.stderr)

        diff_generated = False
        if rust_ok and mmdc_ok:
            mean, rms = diff_images(rust_out, mmdc_out)
            print(f"  mean diff: {mean:.2f}, rms diff: {rms:.2f}")
            save_diff_images(rust_out, mmdc_out, diff_out, side_out, args.diff_scale)
            diff_generated = True
        else:
            reason = "both renders failed"
            if rust_ok and not mmdc_ok:
                reason = "mmdc failed"
            elif mmdc_ok and not rust_ok:
                reason = "mmdr failed"
            print(f"  diff skipped: {reason}", file=sys.stderr)
            for stale in (diff_out, side_out):
                if stale.exists():
                    stale.unlink()

        if rust_error:
            status = f"mmdr error: {rust_error}"
        elif mmdc_error:
            status = f"mmdc error: {mmdc_error}"
        elif layout_error:
            status = f"layout diff error: {layout_error}"
        else:
            status = "ok"

        rows.append(
            {
                "name": display_name,
                "status": status,
                "mean": mean,
                "rms": rms,
                "rust": rust_out.name if rust_ok else "",
                "mmdc": mmdc_out.name if mmdc_ok else "",
                "diff": diff_out.name if diff_generated else "",
                "side": side_out.name if diff_generated else "",
            }
        )

        if args.strict:
            if rust_error:
                failed = True
            elif mean is not None and (mean > args.mean_threshold or rms > args.rms_threshold):
                failed = True

    if output_dir:
        write_html_report(output_dir, rows)
    else:
        if work_dir.exists():
            for child in work_dir.iterdir():
                child.unlink()
            work_dir.rmdir()

    if failed:
        print("One or more comparisons exceeded thresholds.", file=sys.stderr)
        return 2

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
