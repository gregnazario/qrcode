use qrkit::scene::{CornerRadii, Fill, Rgba};

#[test]
fn hex_parsing_supports_3_4_6_8_digit_forms() {
    assert_eq!(
        Rgba::from_hex("#abc").unwrap(),
        Rgba {
            r: 170,
            g: 187,
            b: 204,
            a: 255
        }
    ); // #aabbcc
    assert_eq!(
        Rgba::from_hex("#abcd").unwrap(),
        Rgba {
            r: 170,
            g: 187,
            b: 204,
            a: 221
        }
    ); // #aabbccdd
    assert_eq!(
        Rgba::from_hex("#0d47a1").unwrap(),
        Rgba {
            r: 13,
            g: 71,
            b: 161,
            a: 255
        }
    );
    assert_eq!(
        Rgba::from_hex("#0d47a180").unwrap(),
        Rgba {
            r: 13,
            g: 71,
            b: 161,
            a: 128
        }
    );
    assert_eq!(
        Rgba::from_hex("0d47a1").unwrap(),
        Rgba {
            r: 13,
            g: 71,
            b: 161,
            a: 255
        }
    ); // # optional
    assert!(Rgba::from_hex("#12345").is_err());
    assert!(Rgba::from_hex("#zzzzzz").is_err());
}

#[test]
fn corner_radii_helpers() {
    assert_eq!(
        CornerRadii::all(0.25),
        CornerRadii {
            tl: 0.25,
            tr: 0.25,
            br: 0.25,
            bl: 0.25
        }
    );
    assert_eq!(
        CornerRadii::zero(),
        CornerRadii {
            tl: 0.0,
            tr: 0.0,
            br: 0.0,
            bl: 0.0
        }
    );
}

#[test]
fn fills_are_comparable_and_debuggable() {
    let a = Fill::Solid(Rgba::BLACK);
    let b = Fill::Solid(Rgba::BLACK);
    assert_eq!(a, b);
    let g = Fill::LinearGradient {
        from: Rgba::BLACK,
        to: Rgba::WHITE,
        angle_deg: 45.0,
    };
    assert_ne!(a, g);
    assert!(format!("{g:?}").contains("LinearGradient"));
}
