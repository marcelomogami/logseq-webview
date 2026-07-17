use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

use crate::logseq::{Client, LogseqApiError};
use crate::render;
use crate::routing::path_to_name;
use crate::templates;

pub struct AppState {
    pub client: Client,
}

fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Html(templates::page("Não encontrada", "<h1>404</h1><p>Página não encontrada.</p>")),
    )
        .into_response()
}

/// Estado "Logseq offline" — decisão já fechada em `docs/design.md`,
/// `## Estado "Logseq offline"`: nunca um 500 cru pra esse caso
/// específico, porque é o preço aceito de rodar em LXC separada da
/// máquina do Logseq, não uma falha real do serviço.
fn offline_page() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Html(templates::page(
            "Logseq offline",
            "<h1>Logseq offline</h1><p>Não foi possível conectar à API do Logseq — verifique se o app está aberto no desktop.</p>",
        )),
    )
        .into_response()
}

fn internal_error(context: &str, err: impl std::fmt::Display) -> Response {
    tracing::error!("{context}: {err}");
    (StatusCode::INTERNAL_SERVER_ERROR, "erro interno").into_response()
}

async fn render_page_by_name(state: &AppState, name: &str) -> Response {
    let page = match state.client.get_page(name).await {
        Ok(Some(p)) => p,
        Ok(None) => return not_found(),
        Err(LogseqApiError::Unreachable(_)) => return offline_page(),
        Err(e) => return internal_error("get_page", e),
    };

    let blocks = match state.client.get_page_blocks_tree(name).await {
        Ok(b) => b,
        Err(LogseqApiError::Unreachable(_)) => return offline_page(),
        Err(e) => return internal_error("get_page_blocks_tree", e),
    };

    // Backlinks tratados com a mesma regra de offline das outras duas
    // chamadas (mesma API, mesma máquina) — não degrada silenciosamente
    // pra "sem backlinks" só porque essa terceira chamada específica
    // falhou por rede, o que confundiria mais do que ajudaria.
    let backlinks = match state.client.get_page_linked_references(name).await {
        Ok(b) => b,
        Err(LogseqApiError::Unreachable(_)) => return offline_page(),
        Err(e) => return internal_error("get_page_linked_references", e),
    };

    let content_html = render::render_page_content(&blocks);
    let backlinks_html = render::render_backlinks(&backlinks);
    let body = format!("<h1>{}</h1>{content_html}{backlinks_html}", page.original_name);

    let html = templates::page(&page.original_name, &body);
    let html = templates::with_highlight_css(&html, &render::highlight_css());
    Html(html).into_response()
}

/// `GET /page/{*name}` — qualquer página, inclusive journal (sem rota
/// própria pra journal — ver `docs/design.md`, `## Esquema de rota`).
pub async fn page(State(state): State<Arc<AppState>>, Path(path): Path<String>) -> Response {
    let name = path_to_name(&path);
    render_page_by_name(&state, &name).await
}

/// `GET /` — home é o journal do dia. Fuso horário do processo decide o
/// que é "hoje": a LXC de produção (Fase 5) precisa estar configurada
/// pro fuso de Mato Grosso (America/Cuiaba, UTC-4), senão a home
/// desalinha do que o Logseq desktop considera "hoje" perto da meia-noite
/// — achado da Fase 3, não estava em `docs/design.md`.
pub async fn home(State(state): State<Arc<AppState>>) -> Response {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    // Journal do dia pode não existir ainda (Logseq não foi aberto hoje)
    // — decisão de `docs/visao.md`: estado vazio simples, não 404/erro.
    match state.client.get_page(&today).await {
        Ok(Some(_)) => render_page_by_name(&state, &today).await,
        Ok(None) => Html(templates::page(
            &today,
            &format!("<h1>{today}</h1><p><em>Nenhuma entrada ainda hoje.</em></p>"),
        ))
        .into_response(),
        Err(LogseqApiError::Unreachable(_)) => offline_page(),
        Err(e) => internal_error("home/get_page", e),
    }
}

/// `GET /search` — shell com o campo de busca; a lógica de fuzzy match
/// roda inteira em `search.js` (client-side, ver `docs/visao.md`,
/// ## Navegação e UX). Este handler não fala com a API do Logseq — só
/// serve o HTML; quem busca dado é `/api/pages`, chamado pelo JS.
pub async fn search_page() -> Response {
    Html(templates::search_page()).into_response()
}

/// `GET /api/pages` — lista de páginas (sem journals) pro `search.js`
/// consumir, client-side. Sem cache — chama a API a cada request, decisão
/// já fechada (`docs/design.md`, `## Esquema de rota`).
pub async fn pages_json(State(state): State<Arc<AppState>>) -> Response {
    match state.client.list_pages().await {
        Ok(pages) => Json(pages).into_response(),
        Err(LogseqApiError::Unreachable(_)) => offline_page(),
        Err(e) => internal_error("list_pages", e),
    }
}

pub async fn not_found_handler() -> Response {
    not_found()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_has_404_status() {
        let response = not_found();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn offline_page_has_503_status() {
        let response = offline_page();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
