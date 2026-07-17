use serde::Serialize;
use serde_json::Value;

use crate::model::{BacklinkGroup, Block, Page, PageRef};

/// Protocolo confirmado via `curl` direto contra a API real nesta sessão
/// (não só via `mcp-logseq`, que embrulha a resposta pra apresentação da
/// ferramenta MCP): POST em `/api`, corpo `{"method", "args"}`, resposta é
/// o resultado do método *sem envelope* — objeto, array, ou `null` quando
/// a página não existe (HTTP 200, não 404). Token ausente/errado -> 401.
pub struct Client {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

#[derive(Debug, thiserror::Error)]
pub enum LogseqApiError {
    /// Logseq desktop desligado ou API não respondendo — não confundir com
    /// "página não existe" (isso é `Ok(None)`, não erro). Ver
    /// `docs/design.md`, `## Estado "Logseq offline"`.
    #[error("não foi possível conectar à API do Logseq: {0}")]
    Unreachable(#[from] reqwest::Error),
    /// Token ausente ou inválido — confirmado via teste real: HTTP 401 com
    /// corpo `{"statusCode":401,"error":"Unauthorized",...}`. Erro de
    /// configuração, não de disponibilidade — não deveria acontecer em
    /// produção se o `.env` estiver certo.
    #[error("token da API do Logseq rejeitado (401) — verifique LOGSEQ_API_TOKEN")]
    Unauthorized,
    #[error("resposta inesperada da API do Logseq: {0}")]
    UnexpectedResponse(#[from] serde_json::Error),
}

#[derive(Serialize)]
struct Request<'a> {
    method: &'a str,
    args: Vec<Value>,
}

impl Client {
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into(),
            token: token.into(),
        }
    }

    async fn call(&self, method: &str, args: Vec<Value>) -> Result<Value, LogseqApiError> {
        let response = self
            .http
            .post(format!("{}/api", self.base_url))
            .bearer_auth(&self.token)
            .json(&Request { method, args })
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(LogseqApiError::Unauthorized);
        }

        let body = response.error_for_status()?.text().await?;
        Ok(serde_json::from_str(&body)?)
    }

    /// `None` quando a página não existe — a API devolve `null` (HTTP 200),
    /// não um erro. Distinto de `LogseqApiError`, que é sobre a API estar
    /// alcançável ou não.
    pub async fn get_page(&self, name: &str) -> Result<Option<Page>, LogseqApiError> {
        let value = self
            .call("logseq.Editor.getPage", vec![Value::String(name.into())])
            .await?;
        if value.is_null() {
            return Ok(None);
        }
        Ok(Some(serde_json::from_value(value)?))
    }

    pub async fn get_page_blocks_tree(&self, name: &str) -> Result<Vec<Block>, LogseqApiError> {
        let value = self
            .call(
                "logseq.Editor.getPageBlocksTree",
                vec![Value::String(name.into())],
            )
            .await?;
        Ok(serde_json::from_value(value)?)
    }

    pub async fn get_page_linked_references(
        &self,
        name: &str,
    ) -> Result<Vec<BacklinkGroup>, LogseqApiError> {
        let value = self
            .call(
                "logseq.Editor.getPageLinkedReferences",
                vec![Value::String(name.into())],
            )
            .await?;
        Ok(serde_json::from_value(value)?)
    }

    /// Lista de páginas pra `/api/pages` (busca fzf client-side) — chamada
    /// a cada request, sem cache (decisão já fechada em `docs/visao.md`).
    /// Filtra journals: a busca por título só faz sentido pra páginas
    /// nomeadas, não pra datas.
    pub async fn list_pages(&self) -> Result<Vec<PageRef>, LogseqApiError> {
        let value = self.call("logseq.Editor.getAllPages", vec![]).await?;
        let pages: Vec<PageRef> = serde_json::from_value(value)?;
        Ok(pages.into_iter().filter(|p| !p.journal).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sanidade de serialização do request — não depende de rede.
    #[test]
    fn request_serializes_with_method_and_args() {
        let req = Request {
            method: "logseq.Editor.getPage",
            args: vec![Value::String("teste".into())],
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["method"], "logseq.Editor.getPage");
        assert_eq!(json["args"][0], "teste");
    }
}
