//! Renderer-neutral vector output, enabled by the `scene` feature.
//!
//! The current implementation reuses the existing layout and SVG emitter, then
//! normalizes that SVG with usvg. This is an SVG intermediary, **not** a direct
//! layout backend. No image buffers, rasterization, or renderer handles are used.
//! Text is shaped by usvg using installed system fonts and converted to outlines.
//! All coordinates are canvas-local logical pixels, including viewBox transforms.

use anyhow::{Context, Result, bail, ensure};
use std::sync::{Arc, LazyLock};
use usvg::tiny_skia_path::{Path, PathSegment, PathStroker, Point, Transform};

use crate::RenderOptions;

/// A vector display list in paint order. Consumers should clip to its canvas.
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    pub width: f32,
    pub height: f32,
    pub commands: Vec<SceneCommand>,
}

/// Drawing and balanced graphics-state commands. Paths never require stroking.
#[derive(Debug, Clone, PartialEq)]
pub enum SceneCommand {
    FillPath {
        path: Vec<PathCommand>,
        paint: Paint,
        fill_rule: FillRule,
    },
    /// Intersect the current clip with this filled path until `PopClip`.
    PushClip {
        path: Vec<PathCommand>,
        fill_rule: FillRule,
    },
    PopClip,
    /// Composite the enclosed commands as a group, then apply opacity/blending.
    /// Multiplying each enclosed paint's alpha is NOT equivalent.
    PushLayer {
        opacity: f32,
        blend_mode: BlendMode,
    },
    PopLayer,
}

/// Canvas-local path geometry, with quadratic/cubic control points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathCommand {
    MoveTo {
        x: f32,
        y: f32,
    },
    LineTo {
        x: f32,
        y: f32,
    },
    QuadTo {
        x1: f32,
        y1: f32,
        x: f32,
        y: f32,
    },
    CubicTo {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        x: f32,
        y: f32,
    },
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

/// Unpremultiplied sRGB color. Alpha is in `0..=1`, not `0..=255`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Normal,
    Multiply,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GradientStop {
    pub offset: f32,
    pub color: Color,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Paint {
    Solid(Color),
    /// Canvas-local gradient axis. Stops interpolate in sRGB and extend with
    /// endpoint colors (pad). Do not substitute the path's bounding box.
    LinearGradient {
        start: (f32, f32),
        end: (f32, f32),
        stops: Vec<GradientStop>,
    },
}

static FONTS: LazyLock<Arc<usvg::fontdb::Database>> = LazyLock::new(|| {
    let mut db = usvg::fontdb::Database::new();
    db.load_system_fonts();
    // Minimal Linux installations can have fonts but no fontconfig aliases,
    // leaving fontdb's default Arial/Times New Roman families unresolved.
    let fallback = [
        "Arial",
        "Helvetica",
        "Noto Sans",
        "DejaVu Sans",
        "Liberation Sans",
        "Adwaita Sans",
    ]
    .into_iter()
    .find(|name| {
        db.query(&usvg::fontdb::Query {
            families: &[usvg::fontdb::Family::Name(name)],
            ..Default::default()
        })
        .is_some()
    })
    .map(str::to_owned)
    .or_else(|| {
        db.faces()
            .find(|f| !f.monospaced)
            .or_else(|| db.faces().next())
            .map(|f| f.families[0].0.clone())
    });
    if let Some(fallback) = fallback {
        for family in [usvg::fontdb::Family::SansSerif, usvg::fontdb::Family::Serif] {
            if db
                .query(&usvg::fontdb::Query {
                    families: &[family],
                    ..Default::default()
                })
                .is_none()
            {
                match family {
                    usvg::fontdb::Family::SansSerif => db.set_sans_serif_family(fallback.clone()),
                    _ => db.set_serif_family(fallback.clone()),
                }
            }
        }
    }
    Arc::new(db)
});

/// Render any supported Mermaid diagram to native vector commands.
///
/// Uses the same strict parsing, init directives, layout and options as
/// [`crate::render_with_options`]. The existing SVG output is normalized by
/// usvg internally. Text becomes shaped glyph outlines, strokes become filled
/// paths (including dashes, caps and joins), and transforms are baked in.
/// System fonts must be installed for labels to resolve.
///
/// Returns an error for invalid Mermaid or unsupported vector effects rather
/// than silently rasterizing. Embedded images (including bitmap-only glyphs),
/// radial gradients, patterns, masks, filters and complex clip unions are not
/// supported. None are required by the standard diagram fixtures.
///
/// ```
/// use mermaid_rs_renderer::{render_scene, RenderOptions};
/// let scene = render_scene("flowchart TD; A[Start] --> B[Done]", RenderOptions::default())?;
/// assert!(scene.width > 0.0 && scene.height > 0.0);
/// assert!(!scene.commands.is_empty());
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn render_scene(input: &str, options: RenderOptions) -> Result<Scene> {
    let svg = crate::render_with_options(input, options)?;
    normalize_svg(&svg)
}

fn normalize_svg(svg: &str) -> Result<Scene> {
    // usvg may discard unavailable images. Reject them before parsing so an
    // unsupported image node cannot quietly become a successful empty scene.
    ensure!(
        !svg.contains("<image"),
        "vector scene does not support embedded images"
    );
    ensure!(
        !svg.contains("<text") || FONTS.faces().next().is_some(),
        "vector scene labels require installed system fonts"
    );
    let options = usvg::Options {
        fontdb: Arc::clone(&FONTS),
        ..Default::default()
    };
    let tree = usvg::Tree::from_str(svg, &options).context("normalizing vector scene SVG")?;
    let mut scene = Scene {
        width: tree.size().width(),
        height: tree.size().height(),
        commands: Vec::new(),
    };
    group(tree.root(), Transform::identity(), &mut scene.commands)?;
    Ok(scene)
}

fn group(g: &usvg::Group, parent: Transform, out: &mut Vec<SceneCommand>) -> Result<()> {
    ensure!(
        g.mask().is_none() && g.filters().is_empty(),
        "vector scene does not support masks or filters"
    );
    let blend_mode = match g.blend_mode() {
        usvg::BlendMode::Normal => BlendMode::Normal,
        usvg::BlendMode::Multiply => BlendMode::Multiply,
        other => bail!("unsupported vector scene blend mode: {other:?}"),
    };
    let ts = parent.pre_concat(g.transform());
    let layer = g.opacity().get() != 1.0 || blend_mode != BlendMode::Normal || g.isolate();
    if layer {
        out.push(SceneCommand::PushLayer {
            opacity: g.opacity().get(),
            blend_mode,
        });
    }
    let clips = if let Some(clip) = g.clip_path() {
        push_clip(clip, ts, out)?
    } else {
        0
    };
    for node in g.children() {
        match node {
            usvg::Node::Group(g) => group(g, ts, out)?,
            usvg::Node::Text(t) => group(t.flattened(), ts, out)?,
            usvg::Node::Path(p) if p.is_visible() => path(p, ts, out)?,
            usvg::Node::Path(_) => {}
            usvg::Node::Image(_) => bail!("vector scene does not support images or bitmap glyphs"),
        }
    }
    for _ in 0..clips {
        out.push(SceneCommand::PopClip);
    }
    if layer {
        out.push(SceneCommand::PopLayer);
    }
    Ok(())
}

fn push_clip(clip: &usvg::ClipPath, ts: Transform, out: &mut Vec<SceneCommand>) -> Result<usize> {
    let mut count = 0;
    if let Some(other) = clip.clip_path() {
        count += push_clip(other, ts, out)?;
    }
    // mmdr emits one rectangular clip for XY charts. Restrict arbitrary clip
    // unions instead of confusing SVG union semantics with winding fill.
    let children = clip.root().children();
    ensure!(
        children.len() == 1,
        "vector scene requires a single-path clip"
    );
    let usvg::Node::Path(p) = &children[0] else {
        bail!("vector scene requires a path clip")
    };
    ensure!(p.is_visible(), "vector scene requires a visible clip path");
    out.push(SceneCommand::PushClip {
        path: segments(p.data(), ts.pre_concat(clip.transform()))?,
        fill_rule: p
            .fill()
            .map(|f| rule(f.rule()))
            .unwrap_or(FillRule::NonZero),
    });
    Ok(count + 1)
}

fn path(p: &usvg::Path, ts: Transform, out: &mut Vec<SceneCommand>) -> Result<()> {
    let fill = |out: &mut Vec<SceneCommand>| -> Result<()> {
        if let Some(f) = p.fill() {
            out.push(SceneCommand::FillPath {
                path: segments(p.data(), ts)?,
                paint: paint(f.paint(), f.opacity().get(), ts)?,
                fill_rule: rule(f.rule()),
            });
        }
        Ok(())
    };
    let stroke = |out: &mut Vec<SceneCommand>| -> Result<()> {
        if let Some(s) = p.stroke() {
            let style = s.to_tiny_skia();
            let scale = PathStroker::compute_resolution_scale(&ts);
            let dashed;
            let data = if let Some(dash) = &style.dash {
                let Some(result) = p.data().dash(dash, scale) else {
                    // A short path can lie entirely within a dash gap.
                    return Ok(());
                };
                dashed = result;
                &dashed
            } else {
                p.data()
            };
            // Expand before transforming, preserving non-uniformly scaled caps
            // and joins. tiny-skia-path's stroker does not apply dash itself.
            if let Some(outline) = data.stroke(&style, scale) {
                out.push(SceneCommand::FillPath {
                    path: segments(&outline, ts)?,
                    paint: paint(s.paint(), s.opacity().get(), ts)?,
                    fill_rule: FillRule::NonZero,
                });
            }
        }
        Ok(())
    };
    if p.paint_order() == usvg::PaintOrder::FillAndStroke {
        fill(out)?;
        stroke(out)?;
    } else {
        stroke(out)?;
        fill(out)?;
    }
    Ok(())
}

fn rule(r: usvg::FillRule) -> FillRule {
    match r {
        usvg::FillRule::NonZero => FillRule::NonZero,
        usvg::FillRule::EvenOdd => FillRule::EvenOdd,
    }
}

fn color(c: usvg::Color, a: f32) -> Color {
    Color {
        r: c.red,
        g: c.green,
        b: c.blue,
        a,
    }
}

fn paint(p: &usvg::Paint, alpha: f32, ts: Transform) -> Result<Paint> {
    Ok(match p {
        usvg::Paint::Color(c) => Paint::Solid(color(*c, alpha)),
        usvg::Paint::LinearGradient(g) => {
            ensure!(
                g.spread_method() == usvg::SpreadMethod::Pad,
                "vector scene supports only padded gradients"
            );
            let ts = ts.pre_concat(g.transform());
            let inverse = ts.invert().context("singular vector gradient transform")?;
            let dx = g.x2() - g.x1();
            let dy = g.y2() - g.y1();
            let len2 = dx * dx + dy * dy;
            ensure!(len2 > 0.0, "degenerate vector gradient");
            // A gradient is a scalar field. Its direction transforms by the
            // inverse transpose, not the forward transform under skew/scale.
            let vx = (inverse.sx * dx + inverse.ky * dy) / len2;
            let vy = (inverse.kx * dx + inverse.sy * dy) / len2;
            let vlen2 = vx * vx + vy * vy;
            let mut start = Point::from_xy(g.x1(), g.y1());
            ts.map_point(&mut start);
            Paint::LinearGradient {
                start: (start.x, start.y),
                end: (start.x + vx / vlen2, start.y + vy / vlen2),
                stops: g
                    .stops()
                    .iter()
                    .map(|s| GradientStop {
                        offset: s.offset().get(),
                        color: color(s.color(), alpha * s.opacity().get()),
                    })
                    .collect(),
            }
        }
        _ => bail!("vector scene does not support radial gradients or patterns"),
    })
}

fn segments(path: &Path, ts: Transform) -> Result<Vec<PathCommand>> {
    let path = path
        .clone()
        .transform(ts)
        .context("invalid vector path transform")?;
    Ok(path
        .segments()
        .map(|s| match s {
            PathSegment::MoveTo(p) => PathCommand::MoveTo { x: p.x, y: p.y },
            PathSegment::LineTo(p) => PathCommand::LineTo { x: p.x, y: p.y },
            PathSegment::QuadTo(c, p) => PathCommand::QuadTo {
                x1: c.x,
                y1: c.y,
                x: p.x,
                y: p.y,
            },
            PathSegment::CubicTo(c1, c2, p) => PathCommand::CubicTo {
                x1: c1.x,
                y1: c1.y,
                x2: c2.x,
                y2: c2.y,
                x: p.x,
                y: p.y,
            },
            PathSegment::Close => PathCommand::Close,
        })
        .collect())
}

#[cfg(test)]
mod tests;
