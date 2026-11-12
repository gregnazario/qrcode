use qrkit::content::{Content, Encryption};
use qrkit::style::{Ecc, ModuleShape, Version};
use qrkit_cli::args::{content_from_args, style_from_args, Cli, Commands};

fn cli_for(cmd: Commands) -> Cli {
    Cli {
        command: cmd,
        output: None,
        format: None,
        ecc: None,
        version: None,
        margin: 4,
        shape: None,
        dot_radius: 0.5,
        eye: None,
        fg: None,
        bg: None,
        gradient: None,
        gradient_to: None,
        gradient_angle: 0.0,
        eye_color: None,
        eye_inner_color: None,
        decorative_eyes: false,
        logo: None,
        logo_size: None,
        logo_padding: None,
        px: 24,
        preview: false,
    }
}

#[test]
fn wifi_args_map_to_content() {
    let cli = cli_for(Commands::Wifi {
        ssid: "Cafe Guest".into(),
        password: Some("latte-2026".into()),
        encryption: None,
        hidden: true,
    });
    let c = content_from_args(&cli).unwrap();
    match c {
        Content::Wifi {
            ssid,
            password,
            encryption,
            hidden,
        } => {
            assert_eq!(ssid, "Cafe Guest");
            assert_eq!(password.as_deref(), Some("latte-2026"));
            assert_eq!(encryption, Encryption::Wpa); // default
            assert!(hidden);
        }
        other => panic!("wrong variant: {other:?}"),
    }
}

#[test]
fn url_args_normalize_scheme() {
    let c = content_from_args(&cli_for(Commands::Url {
        url: "example.com/promo".into(),
    }))
    .unwrap();
    assert_eq!(c.payload().unwrap(), "https://example.com/promo");
}

#[test]
fn style_flags_map_to_config() {
    let mut cli = cli_for(Commands::Text { text: "hi".into() });
    cli.fg = Some("#0d47a1".into());
    cli.gradient_to = Some("#42a5f5".into());
    cli.gradient_angle = 45.0;
    cli.shape = Some("dots".into());
    cli.dot_radius = 0.4;
    cli.eye = Some("circle".into());
    cli.ecc = Some("high".into());
    cli.margin = 2;
    cli.version = Some(10);
    let s = style_from_args(&cli).unwrap();
    assert_eq!(s.ecc, Ecc::High);
    assert_eq!(s.margin_modules, 2);
    assert_eq!(s.version, Version::Fixed(10));
    assert_eq!(s.eye_shape, qrkit::style::EyeShape::Circle);
    assert!(matches!(s.shape, ModuleShape::Dots { radius: 0.4 }));
    assert!(matches!(s.fill, qrkit::scene::Fill::LinearGradient { .. }));
}

#[test]
fn bg_transparent_maps_to_none() {
    let mut cli = cli_for(Commands::Text { text: "hi".into() });
    cli.bg = Some("transparent".into());
    let s = style_from_args(&cli).unwrap();
    assert_eq!(s.background, None);
}

#[test]
fn decorative_eyes_flag_maps_to_style() {
    let mut cli = cli_for(Commands::Text { text: "hi".into() });
    cli.decorative_eyes = true;
    let s = style_from_args(&cli).unwrap();
    assert!(s.decorative_eyes);
    assert!(s.validate().iter().any(|w| w.contains("decorative eyes")));
    assert!(
        !style_from_args(&cli_for(Commands::Text { text: "hi".into() }))
            .unwrap()
            .decorative_eyes
    );
}

#[test]
fn completions_and_man_generate() {
    use qrkit_cli::args::{completions_for, man_page};
    let bash = String::from_utf8(completions_for(clap_complete::Shell::Bash)).unwrap();
    assert!(bash.contains("qrkit"), "bash script references the binary");
    let fish = String::from_utf8(completions_for(clap_complete::Shell::Fish)).unwrap();
    assert!(fish.contains("qrkit"));
    let man = String::from_utf8(man_page().unwrap()).unwrap();
    assert!(man.contains(".TH qrkit 1"), "man header");
    // ROFF escapes hyphens in long flags ("decorative\-eyes").
    assert!(man.contains("decorative"), "flags documented");
}
