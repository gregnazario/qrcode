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

    if cli.preview {
        let img = builder.build().map_err(|e| e.to_string())?;
        // re-encode a plain matrix for the half-block preview (style-independent)
        let matrix =
            qrkit::render::Matrix::encode(&payload, Ecc::High, qrkit::style::Version::Auto)
                .map_err(|e| e.to_string())?;
        eprint!("{}", preview::half_blocks(&matrix, 2));
        for w in &img.warnings {
            eprintln!("warning: {w}");
        }
    }

    let format = output_format(cli).map_err(|e| e.to_string())?;
    let bytes = match format {
        "svg" => builder.to_svg().map_err(|e| e.to_string())?,
        "pdf" => builder.to_pdf().map_err(|e| e.to_string())?,
        "eps" => builder.to_eps().map_err(|e| e.to_string())?,
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
