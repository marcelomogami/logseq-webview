use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Formato confirmado via `curl` direto na API bruta (não via `mcp-logseq`,
/// que embrulha a resposta de outro jeito por conveniência da ferramenta
/// MCP) — a resposta de `getPage` é o objeto abaixo, sem envelope.
#[derive(Debug, Clone, Deserialize)]
pub struct Page {
    pub id: i64,
    pub name: String,
    #[serde(rename = "originalName")]
    pub original_name: String,
    pub uuid: String,
    #[serde(rename = "journal?", default)]
    pub journal: bool,
    #[serde(rename = "journalDay", default)]
    pub journal_day: Option<i64>,
    #[serde(default)]
    pub properties: HashMap<String, Value>,
}

/// Idem — `getPageBlocksTree` devolve o array de blocos direto, sem
/// envelope `{"blocks": [...]}`.
#[derive(Debug, Clone, Deserialize)]
pub struct Block {
    pub id: i64,
    pub uuid: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub level: i64,
    #[serde(default)]
    pub children: Vec<Block>,
    #[serde(rename = "preBlock?", default)]
    pub pre_block: bool,
    #[serde(default)]
    pub properties: HashMap<String, Value>,
    /// TODO/DOING/DONE/CANCELED/WAITING (ou outro marcador) — a API já
    /// devolve isso como campo estruturado, separado do prefixo redundante
    /// que também aparece em `content` (ex: `content: "DONE Enviar..."`,
    /// `marker: "DONE"`). Achado desta sessão: não precisa de regex pra
    /// extrair o marcador, só ler este campo — mas o prefixo redundante em
    /// `content` ainda precisa ser removido antes de renderizar, senão
    /// duplica.
    #[serde(default)]
    pub marker: Option<String>,
}

/// Página resumida como aparece dentro de `getPageLinkedReferences` —
/// confirmado via `curl` bruto: **sem `uuid`**, diferente do objeto cheio
/// que `getPage` devolve (por isso não é o mesmo `Page`; um `Page` com
/// `uuid` obrigatório falharia ao desserializar isto).
///
/// Também `Serialize`: é o mesmo shape usado pra devolver `/api/pages`
/// (busca fzf client-side) — reaproveitado em vez de duplicar um struct
/// só pra saída.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRef {
    pub id: i64,
    pub name: String,
    #[serde(rename = "originalName")]
    pub original_name: String,
    #[serde(rename = "journal?", default)]
    pub journal: bool,
    #[serde(rename = "journalDay", default)]
    pub journal_day: Option<i64>,
}

impl PageRef {
    /// **Não usar o campo `journal` cru pra decidir isso** — achado ao
    /// rodar os testes contra dado real: dentro de `getPageLinkedReferences`
    /// a página aninhada pode vir **sem a chave `journal?`** mesmo sendo de
    /// fato um journal (só `journalDay` aparece, e o `#[serde(default)]`
    /// silenciosamente vira `false`). `journalDay` presente é o sinal
    /// confiável nesse contexto — o booleano só é confiável vindo de
    /// `getPage`/`getAllPages`.
    pub fn is_journal(&self) -> bool {
        self.journal || self.journal_day.is_some()
    }
}

/// Um grupo de backlinks: a página de origem + os blocos dela que
/// referenciam a página consultada. Confirmado via `curl` bruto contra
/// `getPageLinkedReferences`: o formato é um array de tuplas
/// `[página, [blocos]]`, não um objeto `{"page":..., "blocks":...}` como
/// o `mcp-logseq` apresenta.
#[derive(Debug, Clone, Deserialize)]
pub struct BacklinkGroup(pub PageRef, pub Vec<Block>);

impl Block {
    /// Nível de heading (1-6), se este bloco for um heading — vem de
    /// `properties.heading`, não do prefixo `#` que também aparece
    /// redundantemente em `content` pra blocos de heading (confirmado
    /// contra a API real: bloco de journal com `content: "# 20 de Março..."`
    /// tinha `properties: {"heading": 1}` ao mesmo tempo).
    pub fn heading_level(&self) -> Option<u8> {
        self.properties
            .get("heading")
            .and_then(Value::as_u64)
            .map(|n| n as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Capturado via `curl` direto contra a API real nesta sessão —
    /// `getPage` pra uma página comum.
    const RAW_PAGE: &str = r##"{"properties":{"type":"[[Módulo]]","tags":"#pessoal"},"updatedAt":1784299646512,"createdAt":1784235368935,"id":4859,"name":"ferramentas pessoais/logseq-webview","uuid":"6a594568-00a3-4a26-91bb-92f472a4ba7c","journal?":false,"originalName":"Ferramentas Pessoais/logseq-webview","file":{"id":4884},"namespace":{"id":32},"format":"markdown"}"##;

    /// Capturado via `mcp-logseq` nesta sessão — página de journal.
    const RAW_JOURNAL_PAGE: &str = r##"{"updatedAt":1784174403371,"journalDay":20260716,"createdAt":1784174403371,"id":4814,"name":"2026-07-16","uuid":"6a594342-00c0-4423-ad42-7deebbd95dd7","journal?":true,"originalName":"2026-07-16","file":{"id":4824},"format":"markdown","properties":{}}"##;

    /// Capturado via API real: bloco com marker + prefixo redundante em
    /// `content`, e refs (que ignoramos de propósito — não precisamos de
    /// resolução de id, ver docs/design.md).
    const RAW_BLOCK_WITH_MARKER: &str = r##"{"parent":{"id":1274},"children":[],"id":1316,"pathRefs":[{"id":7},{"id":1110}],"level":1,"uuid":"6a3afd26-e555-4725-b6db-567e3e04b0cc","content":"DONE Enviar e-mail sobre a tarefa [[Exemplo]]","marker":"DONE","page":{"id":1274},"format":"markdown","refs":[{"id":7}]}"##;

    /// Bloco de journal com heading — content tem o "# " redundante,
    /// properties.heading também está presente.
    const RAW_HEADING_BLOCK: &str = r##"{"properties":{"heading":1},"parent":{"id":1274},"children":[],"id":1311,"pathRefs":[{"id":1274}],"level":1,"uuid":"6a3afd26-846a-40d3-8435-7880bd8849bb","content":"# 20 de Março de 2026","page":{"id":1274},"propertiesOrder":[],"left":{"id":1274},"format":"markdown"}"##;

    #[test]
    fn deserializes_real_page() {
        let page: Page = serde_json::from_str(RAW_PAGE).expect("deve desserializar página real");
        assert_eq!(page.id, 4859);
        assert_eq!(page.name, "ferramentas pessoais/logseq-webview");
        assert_eq!(page.original_name, "Ferramentas Pessoais/logseq-webview");
        assert!(!page.journal);
        assert_eq!(page.journal_day, None);
    }

    #[test]
    fn deserializes_real_journal_page() {
        let page: Page =
            serde_json::from_str(RAW_JOURNAL_PAGE).expect("deve desserializar journal real");
        assert!(page.journal);
        assert_eq!(page.journal_day, Some(20260716));
        assert_eq!(page.name, "2026-07-16");
    }

    #[test]
    fn deserializes_block_with_marker() {
        let block: Block =
            serde_json::from_str(RAW_BLOCK_WITH_MARKER).expect("deve desserializar bloco real");
        assert_eq!(block.marker.as_deref(), Some("DONE"));
        assert!(block.content.starts_with("DONE "));
        assert!(!block.pre_block);
    }

    #[test]
    fn deserializes_block_without_marker() {
        // A maioria dos blocos não tem `marker` no JSON (chave ausente, não
        // null) — confirma que o `#[serde(default)]` é necessário aqui.
        let raw = r##"{"parent":{"id":1},"children":[],"id":2,"level":1,"uuid":"x","content":"texto qualquer","page":{"id":1},"format":"markdown"}"##;
        let block: Block = serde_json::from_str(raw).expect("deve desserializar sem marker");
        assert_eq!(block.marker, None);
    }

    #[test]
    fn heading_level_reads_from_properties() {
        let block: Block =
            serde_json::from_str(RAW_HEADING_BLOCK).expect("deve desserializar bloco de heading");
        assert_eq!(block.heading_level(), Some(1));
        // Confirma o achado: o "# " também está em `content`, redundante —
        // quem monta o markdown final precisa remover isso antes de
        // reaplicar o prefixo a partir de `heading_level()`, senão duplica.
        assert!(block.content.starts_with("# "));
    }

    #[test]
    fn heading_level_none_when_absent() {
        let block: Block =
            serde_json::from_str(RAW_BLOCK_WITH_MARKER).expect("deve desserializar");
        assert_eq!(block.heading_level(), None);
    }

    /// Capturado via `curl` direto contra `getPageLinkedReferences` nesta
    /// sessão — confirma o formato tupla `[página-sem-uuid, [blocos]]`.
    const RAW_BACKLINK_GROUP: &str = r##"[
        {"journalDay":20260716,"name":"2026-07-16","originalName":"2026-07-16","id":4814},
        [{"properties":{},"parent":{"id":4822},"id":4919,"pathRefs":[{"id":32}],"uuid":"6a5a2421-50a9-4cdd-b8fb-8e3caf9cf559","content":"Iniciado o projeto [[Ferramentas Pessoais/logseq-webview]]","page":{"id":4814},"left":{"id":4858},"format":"markdown"}]
    ]"##;

    #[test]
    fn deserializes_real_backlink_group() {
        let group: BacklinkGroup =
            serde_json::from_str(RAW_BACKLINK_GROUP).expect("deve desserializar backlink real");
        assert_eq!(group.0.id, 4814);
        // Achado real: o campo `journal?` cru vem ausente aqui (não
        // `false` explícito) — `is_journal()` existe exatamente pra não
        // confiar nele sozinho nesse contexto. Ver doc do método.
        assert!(!group.0.journal, "journal? não vem nesta resposta — confirma o achado");
        assert!(group.0.is_journal(), "journalDay presente deveria bastar");
        assert_eq!(group.1.len(), 1);
        assert!(group.1[0].content.contains("logseq-webview"));
    }
}
