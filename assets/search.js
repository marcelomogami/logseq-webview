// Busca fzf-style: lista de páginas carregada uma vez (/api/pages), fuzzy
// match tecla a tecla 100% no navegador, sem round-trip de rede por
// caractere digitado — mesma sensação do fzf de terminal. Decisão já
// fechada em docs/visao.md, ## Navegação e UX: só título de página, não
// conteúdo de bloco.

/**
 * Match de subsequência: `query` bate se todos os caracteres aparecem em
 * `text`, na ordem, não necessariamente contíguos.
 *
 * **Normalizado pelo span do match** (`ultimoÍndice - primeiroÍndice`) —
 * sem isso, testado ao vivo contra o grafo real: buscar "mdrdr" botava
 * vários nomes longos do Detran (que só por coincidência espalham as
 * mesmas letras) na frente de "Ferramentas Pessoais/md-reader", o
 * resultado óbvio. A pontuação bruta (soma de match) recompensava string
 * longa com muita coincidência espalhada tanto quanto uma tight match
 * curta — dividir pelo span corrige isso, porque span cresce muito mais
 * rápido que a soma numa string onde a query está espalhada.
 */
function fuzzyScore(query, text) {
  if (query.length === 0) return 0;
  let qi = 0;
  let score = 0;
  let firstMatchIndex = -1;
  let lastMatchIndex = -1;
  for (let ti = 0; ti < text.length && qi < query.length; ti++) {
    if (text[ti] === query[qi]) {
      score += 1;
      if (lastMatchIndex === ti - 1) score += 2; // bônus por contiguidade
      if (firstMatchIndex === -1) firstMatchIndex = ti;
      lastMatchIndex = ti;
      qi++;
    }
  }
  if (qi !== query.length) return -1; // não bateu tudo

  const span = lastMatchIndex - firstMatchIndex + 1;
  const density = score / span; // 0 < density <= ~query.length
  const startBonus = firstMatchIndex === 0 ? 3 : 1 / (firstMatchIndex + 1);
  return density * 100 + startBonus; // density domina; começo do texto desempata
}

async function loadPages() {
  const res = await fetch("/api/pages");
  if (!res.ok) return [];
  return res.json();
}

function pathFromName(name) {
  return name
    .split("/")
    .map((seg) => encodeURIComponent(seg))
    .join("/");
}

function renderResults(container, pages, query) {
  container.innerHTML = "";
  if (query.trim() === "") return;

  const q = query.toLowerCase();
  const scored = pages
    .map((p) => ({ page: p, score: fuzzyScore(q, p.originalName.toLowerCase()) }))
    .filter((r) => r.score >= 0)
    .sort((a, b) => b.score - a.score)
    .slice(0, 30);

  const list = document.createElement("ul");
  for (const { page } of scored) {
    const li = document.createElement("li");
    const a = document.createElement("a");
    a.href = `/page/${pathFromName(page.name)}`;
    a.textContent = page.originalName;
    li.appendChild(a);
    list.appendChild(li);
  }
  container.appendChild(list);
}

async function initSearch() {
  const input = document.getElementById("search-input");
  const results = document.getElementById("search-results");
  if (!input || !results) return;

  const pages = await loadPages();
  input.addEventListener("input", () => renderResults(results, pages, input.value));
  input.focus();
}

document.addEventListener("DOMContentLoaded", initSearch);
