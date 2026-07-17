use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};

/// `NON_ALPHANUMERIC` minus RFC 3986's "unreserved" characters (`- _ . ~`) —
/// mesmo conjunto do md-reader (`templates.rs`), pra não virar `%2D` em nome
/// com hífen.
const PATH_SEGMENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

/// Nome de página (já em minúsculo, como `page.name` da API) -> segmento de
/// rota. `/` do namespace vira separador de path, não fica escapado.
pub fn name_to_path(name: &str) -> String {
    name.split('/')
        .map(|seg| utf8_percent_encode(seg, PATH_SEGMENT).to_string())
        .collect::<Vec<_>>()
        .join("/")
}

/// Rota recebida -> nome de página, pra buscar via `logseq::get_page`.
pub fn path_to_name(path: &str) -> String {
    path.split('/')
        .map(|seg| percent_decode_str(seg).decode_utf8_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/")
}

/// Href de um wikilink/tag encontrado no texto de um bloco — recebe o texto
/// literal de dentro de `[[...]]` ou `#...`, sem chamada à API: `page.name`
/// é sempre `originalName.toLowerCase()` (confirmado contra a API real —
/// ver `docs/design.md`), então minúsculo + mesma codificação da rota
/// própria da página já bate.
pub fn wikilink_href(literal_text: &str) -> String {
    format!("/page/{}", name_to_path(&literal_text.to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_simple_name() {
        let name = "ferramentas pessoais/md-reader";
        assert_eq!(path_to_name(&name_to_path(name)), name);
    }

    #[test]
    fn round_trip_accented_name() {
        // Caso hostil: nome com acento precisa sobreviver ao round-trip
        // (verificado direto na API — ver docs/visao.md, ## Esquema de rota).
        let name = "joão flávio";
        assert_eq!(path_to_name(&name_to_path(name)), name);
    }

    #[test]
    fn round_trip_double_space_name() {
        // Caso hostil: página fantasma com espaço duplo no nome — precisa
        // sobreviver ao round-trip sem virar a versão com espaço simples,
        // senão a bijetividade que motivou o percent-encoding quebra.
        let name = "maria  das neves";
        assert_eq!(path_to_name(&name_to_path(name)), name);
    }

    #[test]
    fn round_trip_punctuation_name() {
        // Caso hostil real: página fantasma "005)".
        let name = "005)";
        assert_eq!(path_to_name(&name_to_path(name)), name);
    }

    #[test]
    fn round_trip_namespace_slash() {
        let name = "trabalho/departamento de exemplo";
        assert_eq!(path_to_name(&name_to_path(name)), name);
    }

    #[test]
    fn accented_and_double_space_names_produce_different_paths() {
        // A propriedade central da decisão: nomes diferentes (mesmo que só
        // por espaço) nunca colidem na mesma rota.
        let a = name_to_path("maria  das neves");
        let b = name_to_path("maria das neves");
        assert_ne!(a, b);
    }

    #[test]
    fn hyphen_is_not_percent_encoded() {
        assert_eq!(name_to_path("second-brain"), "second-brain");
    }

    #[test]
    fn wikilink_href_lowercases_before_encoding() {
        // page.name já vem minúsculo; o literal do wikilink pode vir com
        // qualquer caixa — precisa bater com a rota real da página.
        assert_eq!(
            wikilink_href("Ferramentas Pessoais/md-reader"),
            "/page/ferramentas%20pessoais/md-reader"
        );
    }

    #[test]
    fn wikilink_href_matches_page_own_route() {
        // A garantia central: gerar o href a partir do texto literal do
        // wikilink bate com a rota que a própria página geraria a partir
        // do seu `name` — sem precisar consultar a API pra montar o link.
        let literal = "João Flávio";
        let page_name = "joão flávio"; // como a API devolveria em `page.name`
        assert_eq!(wikilink_href(literal), format!("/page/{}", name_to_path(page_name)));
    }
}
