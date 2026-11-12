use clap::Parser;
use qrkit::builder::QrBuilder;
use qrkit::style::Ecc;
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
    let payload = content.payload().map_err(|e| e.to_string())?;
    let style = style_from_args(cli).map_err(|e| e.to_string())?;
    let builder = QrBuilder::new(content)
        .map_err(|e| e.to_string())?
        .style(style);

    // Build once for both the preview and the output so style warnings are
    // always collected (the per-format to_* helpers would silently drop them).
    let img = builder.build().map_err(|e| e.to_string())?;

    if cli.preview {
        // re-encode a plain matrix for the half-block preview (style-independent)
        let matrix =
            qrkit::render::Matrix::encode(&payload, Ecc::High, qrkit::style::Version::Auto)
                .map_err(|e| e.to_string())?;
        eprint!("{}", preview::half_blocks(&matrix, 2));
    }
    for w in &img.warnings {
        eprintln!("warning: {w}");
    }

    let format = output_format(cli).map_err(|e| e.to_string())?;
    let bytes = match format {
        "svg" => qrkit::writers::svg::write(&img.scene),
        "pdf" => qrkit::writers::pdf::write(&img.scene),
        "eps" => qrkit::writers::eps::write(&img.scene),
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
