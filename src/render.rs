use std::sync::OnceLock;

use comrak::adapters::SyntaxHighlighterAdapter;
use comrak::options::Plugins;
use comrak::plugins::syntect::SyntectAdapterBuilder;
use comrak::{Options, markdown_to_html_with_plugins};
use regex::Regex;
use syntect::highlighting::ThemeSet;
use syntect::html::{ClassStyle, css_for_theme_with_class_style};

use crate::model::{BacklinkGroup, Block};
use crate::routing::wikilink_href;

/// `.css()` do comrak == classes sem prefixo == `ClassStyle::Spaced` do
/// syntect — os dois lados precisam bater (reaproveitado do md-reader).
const CLASS_STYLE: ClassStyle = ClassStyle::Spaced;

fn syntax_highlighter() -> &'static dyn SyntaxHighlighterAdapter {
    static ADAPTER: OnceLock<Box<dyn SyntaxHighlighterAdapter>> = OnceLock::new();
    ADAPTER
        .get_or_init(|| Box::new(SyntectAdapterBuilder::new().css().build()))
        .as_ref()
}

fn comrak_options() -> Options<'static> {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.tasklist = true;
    options.extension.autolink = true;
    options.extension.footnotes = true;
    options.render.r#unsafe = true; // HTML embutido (<mark>, bloco de propriedades) passa direto.
    options
}

/// Sanitiza o HTML já renderizado, preservando `class` (highlight do
/// syntect e o `<mark>` do marcador de tarefa usam) — mesma configuração
/// do md-reader. Verificado nesta sessão: `mark` já sobrevive na allowlist
/// padrão do ammonia, sem precisar liberar nada extra pra ele.
fn sanitize(html: &str) -> String {
    ammonia::Builder::default()
        .add_generic_attributes(&["class"])
        .clean(html)
        .to_string()
}

/// Markdown -> HTML seguro. Reaproveitado do md-reader sem alteração —
/// única diferença é o `syntect` deste projeto vir sem Oniguruma
/// (`default-fancy`, verificado nesta sessão: compila, funciona, zero
/// dependência C).
fn markdown_to_safe_html(source: &str) -> String {
    let options = comrak_options();
    let mut plugins = Plugins::default();
    let highlighter = syntax_highlighter();
    plugins.render.codefence_syntax_highlighter = Some(highlighter);

    let html = markdown_to_html_with_plugins(source, &options, &plugins);
    sanitize(&html)
}

/// CSS de highlight (claro + escuro), pra injetar no `<head>` do shell —
/// mesma função do md-reader.
pub fn highlight_css() -> String {
    static CSS: OnceLock<String> = OnceLock::new();
    CSS.get_or_init(|| {
        let theme_set = ThemeSet::load_defaults();
        let light = theme_set
            .themes
            .get("InspiredGitHub")
            .expect("tema claro InspiredGitHub ausente do ThemeSet padrão do syntect");
        let dark = theme_set
            .themes
            .get("base16-eighties.dark")
            .expect("tema escuro base16-eighties.dark ausente do ThemeSet padrão do syntect");

        let light_css = css_for_theme_with_class_style(light, CLASS_STYLE)
            .expect("falha ao gerar CSS do tema claro");
        let dark_css = css_for_theme_with_class_style(dark, CLASS_STYLE)
            .expect("falha ao gerar CSS do tema escuro");

        format!("{light_css}\n@media (prefers-color-scheme: dark) {{\n{dark_css}\n}}\n")
    })
    .clone()
}

/// `#[[Texto]]` e `[[Texto]]` — mesmo mecanismo de referência (confirmado
/// via API real: mesmo `pathRefs`), tratados pela mesma regex. O `#`
/// opcional é preservado no texto exibido quando presente, sem risco de
/// ser reprocessado pela regex de tag solta (`substitute_bare_tags` roda
/// **antes** desta função, nunca depois — ver `render_block_text`).
fn substitute_bracket_links(text: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"(#)?\[\[(.*?)\]\]").unwrap());

    re.replace_all(text, |caps: &regex::Captures| {
        let hash = caps.get(1).map(|_| "#").unwrap_or("");
        let inner = &caps[2];
        format!("[{hash}{inner}]({})", wikilink_href(inner))
    })
    .into_owned()
}

/// Aplica `f` só **fora** de trechos entre crase simples (`` `código` ``)
/// — dentro de code span, CommonMark nunca interpreta link/tag/ênfase, e
/// nosso pré-processamento (que roda **antes** do comrak) precisa
/// espelhar essa regra manualmente, senão vaza sintaxe interna
/// (`[#tag](href)` cru) pro `<code>` final. Não é um tokenizer completo
/// de CommonMark (não cobre crase dupla/tripla) — cobre o caso real do
/// grafo, que só usa crase simples.
fn substitute_outside_code_spans(text: &str, f: impl Fn(&str) -> String) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"`[^`]*`").unwrap());

    let mut out = String::new();
    let mut last = 0;
    for m in re.find_iter(text) {
        out.push_str(&f(&text[last..m.start()]));
        out.push_str(m.as_str()); // code span intocado, cru
        last = m.end();
    }
    out.push_str(&f(&text[last..]));
    out
}

/// `#tag` sem colchete — mesma referência que wikilink (achado desta
/// sessão: `Ganha-Pão` tem 118 referências no grafo, quase todas via tag,
/// não wikilink — omitir quebraria navegação real, não é feature cosmética).
///
/// **Precisa rodar antes de `substitute_bracket_links`**, nunca depois:
/// depois que um `#[[Tag]]` vira `[#Tag](href)`, o `#` que sobra dentro do
/// texto do link não pode ser escaneado de novo por esta função, senão
/// tenta linkar `#Tag` uma segunda vez e corrompe o markdown. Rodando
/// antes, o `#` de um `#[[...]]` nunca casa aqui porque `[` não é
/// `[\w-]`, então esta função already deixa esse caso pra função seguinte.
///
/// Respeita `\#NNN` escapado (não converte, preserva o backslash pro
/// comrak desfazer na etapa seguinte — comrak já faz isso de graça,
/// verificado nesta sessão).
fn substitute_bare_tags(text: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"(\\)?#([\w-]+)").unwrap());

    re.replace_all(text, |caps: &regex::Captures| {
        let full = caps.get(0).unwrap().as_str();
        if caps.get(1).is_some() {
            // Escapado (`\#NNN`) — não é tag de verdade, deixa intacto
            // (backslash incluso) pro comrak resolver depois.
            return full.to_string();
        }
        let tag = &caps[2];
        format!("[#{tag}]({})", wikilink_href(tag))
    })
    .into_owned()
}

/// Remove o prefixo `#`/`##`/etc redundante que a API também inclui em
/// `content` pra blocos de heading (confirmado via API real: bloco com
/// `properties.heading: 1` tinha `content: "# 20 de Março de 2026"`) — sem
/// isso, o heading próprio que a gente monta a partir de `heading_level()`
/// duplicaria o `#`.
fn strip_heading_prefix(text: &str) -> &str {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"^#{1,6}\s+").unwrap());
    match re.find(text) {
        Some(m) => &text[m.end()..],
        None => text,
    }
}

/// Remove o prefixo do marcador (`DONE `, `TODO `, etc.) que a API também
/// inclui em `content`, redundante com o campo `marker` estruturado —
/// senão duplicaria ao prepender o `<mark>`.
fn strip_marker_prefix<'a>(text: &'a str, marker: &str) -> &'a str {
    let prefix = format!("{marker} ");
    text.strip_prefix(prefix.as_str()).unwrap_or(text)
}

/// Texto de um bloco pronto pra entrar no markdown combinado da página —
/// `None` se, depois de tudo, não sobrar nada pra mostrar (bloco vazio).
fn render_block_text(block: &Block) -> Option<String> {
    let mut text = block.content.as_str();

    if block.heading_level().is_some() {
        text = strip_heading_prefix(text);
    }
    if let Some(marker) = &block.marker {
        text = strip_marker_prefix(text, marker);
    }

    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    // Ordem importa dentro de cada trecho — ver docstring de
    // `substitute_bare_tags`. E só roda **fora** de code span (` `...` `):
    // achado ao vivo (não pego por teste unitário nenhum até então), o
    // pré-processamento roda antes do comrak, então sem essa proteção
    // `` `#tag` `` virava `<code>[#tag](/page/tag)</code>` — sintaxe
    // interna vazando pro HTML final, exatamente o risco que
    // `docs/design.md` já tinha avisado ("cuidado com # dentro de code
    // span") e que só apareceu de fato rodando contra conteúdo real.
    let text = substitute_outside_code_spans(text, |segment| {
        let s = substitute_bare_tags(segment);
        substitute_bracket_links(&s)
    });

    let text = match &block.marker {
        Some(marker) => format!(
            r#"<mark class="marker-{}">{marker}</mark> {text}"#,
            marker.to_lowercase()
        ),
        None => text,
    };

    Some(text)
}

/// Escapa texto pra uso seguro dentro de HTML embutido que a gente monta à
/// mão (bloco de propriedades) — não passa pelo comrak, que só escapa
/// texto markdown, não os valores que a gente injeta direto.
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Converte `[texto](href)` (saída das funções de substituição) em `<a>`
/// de verdade — usado só pro valor de propriedade, que é HTML embutido à
/// mão, não markdown passando pelo comrak.
fn markdown_links_to_html(text: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"\[([^\]]*)\]\(([^)]*)\)").unwrap());
    re.replace_all(text, |caps: &regex::Captures| {
        format!(r#"<a href="{}">{}</a>"#, &caps[2], &caps[1])
    })
    .into_owned()
}

/// Valor de propriedade -> HTML, resolvendo wikilink/tag igual ao resto do
/// conteúdo. **Achado ao rodar contra a API real** (não pego por teste
/// unitário nenhum): `type:: [[Módulo]]`, `part-of:: [[Ferramentas
/// Pessoais]]` e `tags:: #pessoal` são o padrão comum de valor de
/// propriedade no grafo, e a primeira versão desta função só escapava o
/// texto — `[[Módulo]]` aparecia cru na página em vez de virar link.
fn linkify_property_value(value: &str) -> String {
    let escaped = escape_html(value);
    let with_bare_tags = substitute_bare_tags(&escaped);
    let with_links = substitute_bracket_links(&with_bare_tags);
    markdown_links_to_html(&with_links)
}

/// Bloco de propriedades da página (`preBlock?: true`) — renderizado à
/// parte, não como item de lista, mesmo tratamento visual do Logseq
/// nativo. HTML embutido direto na string markdown (passa por `unsafe_`).
fn render_properties_block(block: &Block) -> String {
    let mut html = String::from(r#"<div class="page-properties">"#);
    for (key, value) in &block.properties {
        let value_text = match value {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        html.push_str(&format!(
            "<p><strong>{}</strong>: {}</p>",
            escape_html(key),
            linkify_property_value(&value_text)
        ));
    }
    html.push_str("</div>\n\n");
    html
}

/// Percorre a árvore de blocos e monta uma string markdown única pra
/// página inteira — adaptado do percurso de
/// `third-party/astroplugin-logseq/src/utils/recursively-get-content.ts`,
/// com wikilink/tag virando link de verdade (o astroplugin só remove
/// colchetes) e tratamento à parte pro bloco de propriedades.
fn walk_blocks(blocks: &[Block], depth: usize) -> String {
    let mut out = String::new();
    let indent = "  ".repeat(depth);

    for block in blocks {
        if block.pre_block {
            out.push_str(&render_properties_block(block));
        } else if let Some(text) = render_block_text(block) {
            if let Some(level) = block.heading_level() {
                out.push_str(&format!("{indent}{} {text}\n\n", "#".repeat(level as usize)));
            } else if depth == 0 {
                out.push_str(&format!("{text}\n\n"));
            } else {
                out.push_str(&format!("{indent}- {text}\n"));
            }
        }

        if !block.children.is_empty() {
            out.push_str(&walk_blocks(&block.children, depth + 1));
        }
    }

    out
}

/// Ponto de entrada: árvore de blocos de uma página -> HTML pronto pra
/// embutir no shell (`templates.rs`, fase futura).
pub fn render_page_content(blocks: &[Block]) -> String {
    let markdown = walk_blocks(blocks, 0);
    markdown_to_safe_html(&markdown)
}

/// Seção de backlinks — fora do pipeline de markdown (é lista de página +
/// trecho, não bloco do Logseq). `href` fica a cargo de quem chama
/// (`handlers.rs`, fase futura), já que monta a partir de `PageRef.name`.
pub fn render_backlinks(groups: &[BacklinkGroup]) -> String {
    if groups.is_empty() {
        return String::new();
    }
    let mut html = String::from(r#"<section class="backlinks"><h4>Linked References</h4><ul>"#);
    for group in groups {
        let href = wikilink_href(&group.0.original_name);
        html.push_str(&format!(
            r#"<li><a href="{}">{}</a></li>"#,
            escape_html(&href),
            escape_html(&group.0.original_name)
        ));
    }
    html.push_str("</ul></section>");
    html
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(content: &str) -> Block {
        serde_json::from_value(serde_json::json!({
            "id": 1,
            "uuid": "u",
            "content": content,
            "level": 1,
            "children": [],
        }))
        .unwrap()
    }

    fn block_with(content: &str, extra: serde_json::Value) -> Block {
        let mut base = serde_json::json!({
            "id": 1,
            "uuid": "u",
            "content": content,
            "level": 1,
            "children": [],
        });
        for (k, v) in extra.as_object().unwrap() {
            base[k] = v.clone();
        }
        serde_json::from_value(base).unwrap()
    }

    #[test]
    fn wikilink_becomes_real_link() {
        let text = render_block_text(&block("veja [[Ferramentas Pessoais/md-reader]] aqui"))
            .unwrap();
        assert!(text.contains("[Ferramentas Pessoais/md-reader](/page/ferramentas%20pessoais/md-reader)"));
    }

    #[test]
    fn bracket_tag_becomes_link_with_hash_preserved_in_display() {
        let text = render_block_text(&block("marcado #[[ganha-pão]] aqui")).unwrap();
        assert!(text.contains("[#ganha-pão](/page/ganha-p%C3%A3o)"));
    }

    #[test]
    fn bare_tag_becomes_link() {
        let text = render_block_text(&block("revisão do #trânsito hoje")).unwrap();
        assert!(text.contains("[#trânsito](/page/tr%C3%A2nsito)"));
    }

    #[test]
    fn escaped_hash_number_is_not_linked() {
        // Caso real do grafo: `\#005` não pode virar link pra página "005".
        let text = render_block_text(&block(r"corrigida (\#005): sucesso")).unwrap();
        assert!(!text.contains("](/page/005)"));
        assert!(text.contains(r"\#005"), "backslash deve sobreviver pro comrak desfazer: {text}");
    }

    #[test]
    fn hash_inside_code_span_is_not_linked() {
        // Bug real, achado rodando o servidor de verdade contra o próprio
        // journal desta sessão: `` `#tag` `` (texto literal, mencionando a
        // sintaxe de tag) virava `<code>[#tag](/page/tag)</code>` — a
        // substituição rodava dentro da crase, vazando markdown cru pro
        // HTML final em vez de deixar o comrak tratar como código literal.
        let text = render_block_text(&block("suporte a tag (`#tag`/`#[[Tag]]`) adicionado")).unwrap();
        assert!(
            !text.contains("](/page/"),
            "não deveria ter link nenhum, tudo estava entre crase: {text}"
        );
        assert!(text.contains("`#tag`"), "conteúdo do code span deve sobreviver intacto: {text}");
        assert!(text.contains("`#[[Tag]]`"), "conteúdo do code span deve sobreviver intacto: {text}");
    }

    #[test]
    fn wikilink_outside_code_span_still_works_when_block_also_has_code_span() {
        // Garante que a proteção não é ampla demais — texto fora da crase
        // no mesmo bloco continua sendo processado normalmente.
        let text = render_block_text(&block("veja [[Ferramentas Pessoais/md-reader]] e o `código literal`")).unwrap();
        assert!(text.contains("[Ferramentas Pessoais/md-reader](/page/ferramentas%20pessoais/md-reader)"));
        assert!(text.contains("`código literal`"));
    }

    #[test]
    fn bracket_tag_is_not_double_processed_by_bare_tag_regex() {
        // Se a ordem estivesse errada, "#ganha-pão" dentro do texto de
        // exibição do link gerado seria escaneado de novo.
        let text = render_block_text(&block("#[[ganha-pão]]")).unwrap();
        // Só um link deve existir — não dois aninhados/quebrados.
        assert_eq!(text.matches("](/page/").count(), 1);
    }

    #[test]
    fn marker_prefix_is_stripped_and_wrapped_in_mark() {
        let b = block_with("DONE Enviar e-mail", serde_json::json!({"marker": "DONE"}));
        let text = render_block_text(&b).unwrap();
        assert_eq!(text, r#"<mark class="marker-done">DONE</mark> Enviar e-mail"#);
    }

    #[test]
    fn heading_prefix_is_stripped_and_not_duplicated() {
        let b = block_with(
            "# 20 de Março de 2026",
            serde_json::json!({"properties": {"heading": 1}}),
        );
        let markdown = walk_blocks(std::slice::from_ref(&b), 0);
        // Só um "#" deve aparecer no início da linha final, não "# # ...".
        assert!(markdown.starts_with("# 20 de Março de 2026"));
        assert!(!markdown.starts_with("# #"));
    }

    #[test]
    fn empty_block_renders_nothing_but_children_still_walk() {
        let mut parent = block("");
        parent.children = vec![block("filho com conteúdo")];
        let markdown = walk_blocks(std::slice::from_ref(&parent), 0);
        assert!(markdown.contains("filho com conteúdo"));
    }

    #[test]
    fn properties_block_renders_separately_not_as_bullet() {
        let b = block_with(
            "type:: [[Módulo]]\ndescription:: teste",
            serde_json::json!({
                "preBlock?": true,
                "properties": {"type": "[[Módulo]]", "description": "teste"}
            }),
        );
        let markdown = walk_blocks(std::slice::from_ref(&b), 0);
        assert!(markdown.contains(r#"<div class="page-properties">"#));
        assert!(!markdown.contains("- type::"));
    }

    #[test]
    fn property_value_wikilink_becomes_real_link() {
        // Bug real achado rodando contra a API (não pego por teste nenhum
        // até então): valor de propriedade com `[[wikilink]]` aparecia cru
        // — `type:: [[Módulo]]`, `part-of:: [[Ferramentas Pessoais]]` são o
        // padrão comum de property no grafo.
        let b = block_with(
            "type:: [[Módulo]]",
            serde_json::json!({
                "preBlock?": true,
                "properties": {"type": "[[Módulo]]", "tags": "#pessoal"}
            }),
        );
        let markdown = walk_blocks(std::slice::from_ref(&b), 0);
        assert!(
            markdown.contains(r#"<a href="/page/m%C3%B3dulo">Módulo</a>"#),
            "esperava link resolvido no valor da propriedade: {markdown}"
        );
        assert!(
            markdown.contains(r#"<a href="/page/pessoal">#pessoal</a>"#),
            "esperava tag resolvida no valor da propriedade: {markdown}"
        );
    }

    #[test]
    fn full_pipeline_produces_sanitized_html_with_link() {
        let b = block("veja [[Alguma Página]] e #trabalho");
        let html = render_page_content(std::slice::from_ref(&b));
        assert!(html.contains("<a href=\"/page/alguma%20p"));
        assert!(html.contains("<a href=\"/page/trabalho\""));
    }

    #[test]
    fn script_tag_is_stripped_by_sanitizer() {
        let b = block("texto <script>alert(1)</script> fim");
        let html = render_page_content(std::slice::from_ref(&b));
        assert!(!html.contains("<script"));
    }

    #[test]
    fn code_block_uses_css_classes_for_highlight() {
        let b = block("```rust\nfn main() {}\n```");
        let html = render_page_content(std::slice::from_ref(&b));
        assert!(html.contains("class="));
    }

    #[test]
    fn highlight_css_has_both_themes() {
        let css = highlight_css();
        assert!(css.contains("prefers-color-scheme: dark"));
    }

    #[test]
    fn backlinks_render_empty_string_when_no_groups() {
        assert_eq!(render_backlinks(&[]), "");
    }

    #[test]
    fn backlinks_render_links_from_groups() {
        let group: BacklinkGroup = serde_json::from_value(serde_json::json!([
            {"id": 1, "name": "ganha-pão", "originalName": "Ganha-Pão"},
            []
        ]))
        .unwrap();
        let html = render_backlinks(&[group]);
        assert!(html.contains("Ganha-Pão"));
        assert!(html.contains("/page/ganha-p"));
    }
}
