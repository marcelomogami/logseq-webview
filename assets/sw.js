// Service worker mínimo — só o necessário pra instalabilidade PWA. Sem
// cache: todo fetch vai direto pra rede. Um SW cache-first aqui prenderia
// o usuário numa versão velha/quebrada sempre que a sessão de auth na
// frente deste servidor expirar e os requests começarem a ser
// redirecionados (cross-origin) pra uma página de login.

self.addEventListener("install", (event) => {
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(self.clients.claim());
});

self.addEventListener("fetch", (event) => {
  // Nunca interceptar navegação (carregamento de página): a barreira de
  // auth na frente deste servidor (Cloudflare Access na rota externa,
  // basic auth do Caddy na LAN — ver docs/design.md, ## Firewall da LXC)
  // pode redirecionar cross-origin pra uma página de login, e um service
  // worker que entra no meio com fetch(event.request) quebra esse
  // redirect — especialmente na janela standalone de um PWA instalado
  // (ERR_FAILED). Deixa o navegador cuidar disso nativamente.
  //
  // Bug real e conhecido (não teórico): o md-reader só descobriu isso
  // testando instalado no celular, depois do fato — aqui entra desde o
  // commit inicial (docs/visao.md, ## PWA).
  if (event.request.mode === "navigate") {
    return;
  }
  event.respondWith(fetch(event.request));
});
