use axum::http::header;
use axum::response::{IntoResponse, Response};

// Todos vendorizados no binário — sem CDN, mesmo padrão do md-reader
// (github-markdown-css) e da decisão já fechada em docs/visao.md,
// ## Interface visual.
const MANIFEST: &str = include_str!("../assets/manifest.webmanifest");
const SERVICE_WORKER: &str = include_str!("../assets/sw.js");
const SEARCH_JS: &str = include_str!("../assets/search.js");
const NAV_ICON: &[u8] = include_bytes!("../assets/icons/nav-icon.png");
const ICON_192: &[u8] = include_bytes!("../assets/icons/icon-192.png");
const ICON_512: &[u8] = include_bytes!("../assets/icons/icon-512.png");
const ICON_512_MASKABLE: &[u8] = include_bytes!("../assets/icons/icon-512-maskable.png");
const APPLE_TOUCH_ICON: &[u8] = include_bytes!("../assets/icons/apple-touch-icon.png");

pub async fn manifest() -> Response {
    (
        [(header::CONTENT_TYPE, "application/manifest+json")],
        MANIFEST,
    )
        .into_response()
}

pub async fn service_worker() -> Response {
    ([(header::CONTENT_TYPE, "text/javascript")], SERVICE_WORKER).into_response()
}

pub async fn search_js() -> Response {
    ([(header::CONTENT_TYPE, "text/javascript")], SEARCH_JS).into_response()
}

/// Ícone do nav (96x96, distinto do conjunto de ícones PWA abaixo —
/// tamanho pequeno de propósito, servido em toda página, não só na
/// instalação). Redimensionado de `assets/icons/source.png` (500x500,
/// feito pelo Marcelo).
pub async fn nav_icon() -> Response {
    ([(header::CONTENT_TYPE, "image/png")], NAV_ICON).into_response()
}

pub async fn icon_192() -> Response {
    ([(header::CONTENT_TYPE, "image/png")], ICON_192).into_response()
}

pub async fn icon_512() -> Response {
    ([(header::CONTENT_TYPE, "image/png")], ICON_512).into_response()
}

pub async fn icon_512_maskable() -> Response {
    ([(header::CONTENT_TYPE, "image/png")], ICON_512_MASKABLE).into_response()
}

pub async fn apple_touch_icon() -> Response {
    ([(header::CONTENT_TYPE, "image/png")], APPLE_TOUCH_ICON).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_is_valid_json_with_required_fields() {
        let value: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(value["display"], "standalone");
        assert_eq!(value["start_url"], "/");
        assert!(value["icons"].as_array().unwrap().len() >= 2);
    }

    #[test]
    fn service_worker_never_intercepts_navigation() {
        // Garantia estática: o fix crítico (docs/visao.md, ## PWA) precisa
        // estar no arquivo vendorizado, não só documentado em comentário.
        assert!(SERVICE_WORKER.contains(r#"event.request.mode === "navigate""#));
    }

    #[test]
    fn service_worker_does_not_cache() {
        // Decisão explícita (comentário no próprio sw.js): sem cache,
        // porque cache-first prenderia sessão de auth expirada numa
        // versão velha. Confirma que ninguém adicionou `caches.open`
        // sem revisar essa decisão.
        assert!(!SERVICE_WORKER.contains("caches.open"));
    }
}
