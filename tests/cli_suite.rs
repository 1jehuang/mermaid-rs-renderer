use std::io::Write;
use std::process::{Command, Stdio};

fn mmdr() -> Command {
    Command::new(env!("CARGO_BIN_EXE_mmdr"))
}

fn assert_svg_size(svg: &str, width: &str, height: &str) {
    let root = svg
        .split('>')
        .next()
        .expect("SVG output should include a root element");
    assert!(
        root.contains(&format!("width=\"{width}\"")),
        "expected width={width:?} in root element: {root}"
    );
    assert!(
        root.contains(&format!("height=\"{height}\"")),
        "expected height={height:?} in root element: {root}"
    );
}

#[test]
fn cli_width_height_affect_stdout_svg() {
    let mut child = mmdr()
        .args(["--width", "321", "--height", "123", "--input", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to run mmdr");
    child
        .stdin
        .as_mut()
        .expect("failed to open stdin")
        .write_all(b"flowchart TD\n  A-->B\n")
        .expect("failed to write stdin");
    let output = child.wait_with_output().expect("failed to wait for mmdr");

    assert!(
        output.status.success(),
        "mmdr failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let svg = String::from_utf8(output.stdout).expect("SVG stdout should be UTF-8");
    assert_svg_size(&svg, "321", "123");
}

#[test]
fn cli_width_height_affect_file_svg() {
    let dir = std::env::temp_dir().join(format!(
        "mermaid-rs-renderer-cli-test-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("failed to create temp dir");
    let input = dir.join("input.mmd");
    let output_path = dir.join("output.svg");
    std::fs::write(&input, "flowchart TD\n  A-->B\n").expect("failed to write input");

    let output = mmdr()
        .args([
            "--width",
            "654",
            "--height",
            "456",
            "--input",
            input.to_str().unwrap(),
            "--output",
            output_path.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run mmdr");

    assert!(
        output.status.success(),
        "mmdr failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let svg = std::fs::read_to_string(&output_path).expect("failed to read SVG output");
    assert_svg_size(&svg, "654", "456");

    let _ = std::fs::remove_file(input);
    let _ = std::fs::remove_file(output_path);
    let _ = std::fs::remove_dir(dir);
}

#[test]
fn cli_info_diagram_reports_crate_version() {
    // Mirrors `echo "info" | cargo run -- -i - -o info.svg`: the `info`
    // diagram has no body, just a header, and always renders this crate's
    // own name and version (matching how Mermaid.js's own `info` diagram
    // reports its version).
    let dir = std::env::temp_dir().join(format!(
        "mermaid-rs-renderer-cli-info-test-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("failed to create temp dir");
    let output_path = dir.join("info.svg");

    let mut child = mmdr()
        .args(["-i", "-", "-o", output_path.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to run mmdr");
    child
        .stdin
        .as_mut()
        .expect("failed to open stdin")
        .write_all(b"info\n")
        .expect("failed to write stdin");
    let output = child.wait_with_output().expect("failed to wait for mmdr");

    assert!(
        output.status.success(),
        "mmdr failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let svg = std::fs::read_to_string(&output_path).expect("failed to read SVG output");
    let expected_text = format!("mermaid-rs {}", env!("CARGO_PKG_VERSION"));
    assert!(
        svg.contains(&expected_text),
        "expected info SVG to contain {expected_text:?}: {svg}"
    );

    let _ = std::fs::remove_file(output_path);
    let _ = std::fs::remove_dir(dir);
}
