//! QR code rendering for testnet receiving addresses.

use dioxus::prelude::*;
use qrcodegen::{QrCode as QrCodeGenerator, QrCodeEcc};

const QUIET_ZONE: i32 = 4;

/// Render a QR code as an inline SVG.
#[component]
pub fn AddressQrCode(value: String) -> Element {
    let svg = qr_svg(&value).unwrap_or_default();

    rsx! {
        div {
            class: "wallet-qr",
            role: "img",
            aria_label: "QR code for the selected testnet receiving address",
            dangerous_inner_html: "{svg}",
        }
    }
}

fn qr_svg(value: &str) -> Result<String, qrcodegen::DataTooLong> {
    let qr = QrCodeGenerator::encode_text(value, QrCodeEcc::Medium)?;
    let modules = qr.size();
    let dimension = modules + QUIET_ZONE * 2;
    let mut path = String::new();

    for y in 0..modules {
        for x in 0..modules {
            if qr.get_module(x, y) {
                let x = x + QUIET_ZONE;
                let y = y + QUIET_ZONE;
                path.push_str(&format!("M{x} {y}h1v1h-1z"));
            }
        }
    }

    Ok(format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {dimension} {dimension}" shape-rendering="crispEdges"><rect width="{dimension}" height="{dimension}" fill="#ffffff"/><path d="{path}" fill="#07110c"/></svg>"##
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qr_svg_renders_an_accessible_inline_code() {
        let svg = qr_svg("tb1q6rz28mcfaxtmd6v789l9rrlrusdprr9pqcpvkl").unwrap();

        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("viewBox="));
        assert!(svg.contains("<path"));
        assert!(!svg.contains("script"));
    }

    #[test]
    fn qr_svg_rejects_values_that_are_too_long() {
        assert!(qr_svg(&"a".repeat(10_000)).is_err());
    }
}
