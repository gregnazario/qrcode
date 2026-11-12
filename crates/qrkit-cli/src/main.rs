use clap::Parser;
use qrkit::builder::QrBuilder;
use qrkit_cli::args::{content_from_args, output_format, style_from_args, Cli};
use qrkit_cli::preview;

fn main() {
    let cli = Cli::parse();
    if let Err(msg) = run(&cli) {
        eprintln!("error: {msg}");
        std::process::exit(1);
    }
}

fn run(cli: &Cli) -> Result<(), String> {
    let content = content_from_args(cli).map_err(|e| e.to_string())?;
    let style = style_from_args(cli).map_err(|e| e.to_string())?;
    let format = output_format(cli).map_err(|e| e.to_string())?;
    // The term renderer works from the matrix + style (not the scene); the
    // clone can carry logo bytes, so only pay it for that format.
    let style_for_term = if format == "term" {
        style.clone()
    } else {
        Default::default()
    };
    let builder = QrBuilder::new(content)
        .map_err(|e| e.to_string())?
        .style(style);

    // Build once for both the preview and the output so style warnings are
    // always collected (the per-format to_* helpers would silently drop them).
    let img = builder.build().map_err(|e| e.to_string())?;

    if cli.preview {
        // Plain half-block preview from the SAME matrix as the output — a
        // pinned version or custom ECC is reflected accurately.
        eprint!("{}", preview::half_blocks(&img.matrix, 2));
    }
    for w in &img.warnings {
        eprintln!("warning: {w}");
    }

    let bytes = match format {
        "svg" => qrkit::writers::svg::write(&img.scene),
        "pdf" => qrkit::writers::pdf::write(&img.scene),
        "eps" => qrkit::writers::eps::write(&img.scene),
        "term" => qrkit::writers::term::render(&img.matrix, &style_for_term),
        _ => unreachable!(),
    };

    match &cli.output {
        Some(path) => {
            std::fs::write(path, bytes).map_err(|e| format!("cannot write {path:?}: {e}"))?;
            eprintln!("wrote {path:?} ({format})");
        }
        None => {
            use std::io::Write as _;
            std::io::stdout()
                .write_all(&bytes)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
