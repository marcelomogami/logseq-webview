/// Vendorizado — `assets/vendor/pico.min.css` + `assets/vendor/pico-LICENSE.md`
/// (MIT, aviso de copyright completo mantido no diretório, mesmo padrão do
/// `github-markdown-css` do md-reader). Embutido inline no `<head>` de
/// cada página (não servido como rota própria) — decisão já fechada em
/// docs/visao.md, ## Interface visual: sem CDN, sem passo de build.
const PICO_CSS: &str = include_str!("../assets/vendor/pico.min.css");

/// Roxo escuro amostrado do próprio ícone (`assets/icons/source.png`) —
/// usado como `theme-color` e cor de fundo dos ícones maskable/apple, pra
/// bater com a identidade visual em vez de um azul genérico.
const THEME_COLOR: &str = "#400c5a";

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn head(title: &str, extra_head: &str) -> String {
    let title = escape_html(title);
    format!(
        r##"<!doctype html>
<html lang="pt-BR">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<link rel="manifest" href="/manifest.webmanifest">
<link rel="icon" type="image/png" href="/icons/icon-192.png">
<meta name="theme-color" content="{THEME_COLOR}">
<meta name="mobile-web-app-capable" content="yes">
<meta name="apple-mobile-web-app-capable" content="yes">
<meta name="apple-mobile-web-app-status-bar-style" content="black-translucent">
<meta name="apple-mobile-web-app-title" content="logseq-webview">
<link rel="apple-touch-icon" href="/icons/apple-touch-icon.png">
<style>{PICO_CSS}</style>
<style>
main {{ max-width: 700px; margin: 0 auto; padding: var(--pico-spacing); }}
.page-properties {{ border: 1px solid var(--pico-muted-border-color); border-radius: var(--pico-border-radius); padding: 0.75rem 1rem; margin-bottom: 1.5rem; font-size: 0.9em; }}
.page-properties p {{ margin: 0.25rem 0; }}
.backlinks {{ border-top: 1px solid var(--pico-muted-border-color); margin-top: 2rem; padding-top: 1rem; }}
mark[class^="marker-"] {{ background: none; font-weight: bold; padding: 0; }}
.marker-done {{ text-decoration: line-through; opacity: 0.7; }}
</style>
{extra_head}
</head>
<body>
<script>if ('serviceWorker' in navigator) navigator.serviceWorker.register('/sw.js');</script>
<nav class="container">
<ul><li><a href="/"><img src="/assets/icon.png" width="32" height="32" alt="logseq-webview — início"></a></li></ul>
<ul><li><a href="/search">Buscar</a></li></ul>
</nav>
"##
    )
}

const FOOT: &str = "</body>\n</html>\n";

/// Shell padrão — usado por toda página de conteúdo (home, journal,
/// página comum, 404, "Logseq offline").
pub fn page(title: &str, body_html: &str) -> String {
    let mut out = head(title, "");
    out.push_str(r#"<main class="container">"#);
    out.push_str(body_html);
    out.push_str("</main>\n");
    out.push_str(FOOT);
    out
}

/// Shell da página de busca — carrega `search.js`, que popula
/// `#search-results` a partir de `#search-input` (fuzzy match
/// client-side, sem round-trip de rede por tecla — docs/visao.md,
/// ## Navegação e UX).
pub fn search_page() -> String {
    let mut out = head("Buscar", r#"<script src="/assets/search.js" defer></script>"#);
    out.push_str(
        r#"<main class="container">
<input type="search" id="search-input" placeholder="Buscar página por título..." autocomplete="off">
<div id="search-results"></div>
</main>
"#,
    );
    out.push_str(FOOT);
    out
}

/// Highlight CSS (claro + escuro) — injetado só na página de conteúdo
/// (não na busca/404, que não têm bloco de código), depois do shell já
/// montado, pra não obrigar `page()` a saber sobre highlight.
pub fn with_highlight_css(html: &str, highlight_css: &str) -> String {
    html.replacen(
        "</head>",
        &format!("<style>{highlight_css}</style></head>"),
        1,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_includes_title_body_and_pico_css() {
        let html = page("Título", "<p>corpo</p>");
        assert!(html.contains("<title>Título</title>"));
        assert!(html.contains("<p>corpo</p>"));
        assert!(html.contains("Pico CSS"), "CSS do Pico deve estar embutido");
    }

    #[test]
    fn page_includes_favicon_link() {
        let html = page("t", "b");
        assert!(html.contains(r#"<link rel="icon" type="image/png" href="/icons/icon-192.png">"#));
    }

    #[test]
    fn page_title_is_html_escaped() {
        let html = page("<script>alert(1)</script>", "corpo");
        assert!(!html.contains("<script>alert(1)</script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn search_page_loads_search_js() {
        let html = search_page();
        assert!(html.contains(r#"<script src="/assets/search.js""#));
        assert!(html.contains(r#"id="search-input""#));
        assert!(html.contains(r#"id="search-results""#));
    }

    #[test]
    fn with_highlight_css_injects_before_head_close() {
        let html = page("t", "b");
        let with_css = with_highlight_css(&html, ".highlight { color: red; }");
        assert!(with_css.contains(".highlight { color: red; }"));
        assert!(with_css.find(".highlight").unwrap() < with_css.find("<body>").unwrap());
    }
}
