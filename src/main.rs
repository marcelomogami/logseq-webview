mod handlers;
mod logseq;
mod model;
mod pwa;
mod render;
mod routing;
mod templates;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::routing::get;

use handlers::AppState;

fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(handlers::home))
        .route("/page/{*name}", get(handlers::page))
        .route("/search", get(handlers::search_page))
        .route("/api/pages", get(handlers::pages_json))
        .route("/assets/icon.png", get(pwa::nav_icon))
        .route("/assets/search.js", get(pwa::search_js))
        .route("/manifest.webmanifest", get(pwa::manifest))
        .route("/sw.js", get(pwa::service_worker))
        .route("/icons/icon-192.png", get(pwa::icon_192))
        .route("/icons/icon-512.png", get(pwa::icon_512))
        .route("/icons/icon-512-maskable.png", get(pwa::icon_512_maskable))
        .route("/icons/apple-touch-icon.png", get(pwa::apple_touch_icon))
        .fallback(handlers::not_found_handler)
        .with_state(state)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // `.env` mesma estrutura em dev e produção (docs/visao.md, ## Token
    // da API) — `dotenvy` não falha se o arquivo não existir, só se as
    // vars obrigatórias estiverem ausentes do ambiente final.
    dotenvy::dotenv().ok();

    let api_url = std::env::var("LOGSEQ_API_URL").expect("defina LOGSEQ_API_URL no .env");
    let api_token = std::env::var("LOGSEQ_API_TOKEN").expect("defina LOGSEQ_API_TOKEN no .env");

    let state = Arc::new(AppState {
        client: logseq::Client::new(api_url, api_token),
    });

    let app = build_router(state);

    // Bind em todas as interfaces: o Caddy (LAN) e, em produção, tráfego
    // vindo de outra LXC alcançam por rede, não por localhost — mesmo
    // padrão do md-reader. Autenticação é 100% infraestrutura (Caddy +
    // Cloudflare Access + firewall restrito) — este processo não valida
    // credencial nenhuma (docs/design.md, ## Firewall da LXC).
    let addr = SocketAddr::from(([0, 0, 0, 0], 47475));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| panic!("falha ao abrir {addr}: {e}"));
    tracing::info!("logseq-webview ouvindo em http://{addr}");
    axum::serve(listener, app)
        .await
        .expect("erro fatal servindo requisições");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn test_state() -> Arc<AppState> {
        Arc::new(AppState {
            // URL propositalmente inválida — os testes de roteamento aqui
            // só verificam qual rota casa e o formato da resposta de
            // erro, não o conteúdo real da API (isso é `render.rs`,
            // já coberto). Nenhum destes deve tentar rede de verdade além
            // de uma tentativa de conexão que falha rápido.
            client: logseq::Client::new("http://127.0.0.1:1", "token-de-teste"),
        })
    }

    #[tokio::test]
    async fn unknown_route_returns_404() {
        let app = build_router(test_state());
        let response = app
            .oneshot(Request::builder().uri("/rota/que/nao/existe/nenhuma").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn page_route_with_unreachable_api_returns_503() {
        let app = build_router(test_state());
        let response = app
            .oneshot(Request::builder().uri("/page/alguma-pagina").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn home_route_with_unreachable_api_returns_503() {
        let app = build_router(test_state());
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn pages_json_route_with_unreachable_api_returns_503() {
        let app = build_router(test_state());
        let response = app
            .oneshot(Request::builder().uri("/api/pages").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
