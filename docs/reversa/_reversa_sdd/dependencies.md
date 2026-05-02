# Dependencias — logseq

> Generado por el Scout en 2026-05-02

## deps.edn (Clojure/ClojureScript)

```clojure
{:paths ["src/main" "src/electron" "src/resources"]
 :deps
 {org.clojure/clojure                   {:mvn/version "1.12.4"}
  rum/rum                               {:git/url "https://github.com/logseq/rum" :sha "5d672bf84ed944414b9f61eeb83808ead7be9127"}
  datascript/datascript                 {:git/url "https://github.com/logseq/datascript" :sha "f91fec561ee2c11d6bf323feae365e9033585411"}
  datascript-transit/datascript-transit {:mvn/version "0.3.0"}
  borkdude/rewrite-edn                  {:mvn/version "0.5.9"}
  funcool/promesa                       {:mvn/version "11.0.678"}
  medley/medley                         {:mvn/version "1.4.0"}
  metosin/reitit-frontend               {:mvn/version "0.10.1"}
  cljs-bean/cljs-bean                   {:mvn/version "1.9.0"}
  prismatic/dommy                       {:mvn/version "1.1.0"}
  org.clojure/core.match                {:mvn/version "1.1.0"}
  com.andrewmcveigh/cljs-time           {:git/url "https://github.com/logseq/cljs-time" :sha "5704fbf48d3478eedcf24d458c8964b3c2fd59a9"}
  cljs-drag-n-drop/cljs-drag-n-drop     {:mvn/version "0.1.0"}
  cljs-http/cljs-http                   {:mvn/version "0.1.49"}
  org.babashka/sci                     {:mvn/version "0.12.51"}
  org.clj-commons/hickory               {:mvn/version "0.7.7"}
  org.clj-commons/humanize              {:mvn/version "1.2"}
  org.babashka/cli                      {:mvn/version "0.8.67"}
  hiccups/hiccups                       {:mvn/version "0.3.0"}
  tongue/tongue                         {:mvn/version "0.4.4"}
  org.clojure/core.async                {:mvn/version "1.8.741"}
  thheller/shadow-cljs                  {:mvn/version "3.4.4"}
  expound/expound                       {:mvn/version "0.9.0"}
  com.lambdaisland/glogi                {:git/url "https://github.com/lambdaisland/glogi" :sha "30328a045141717aadbbb693465aed55f0904976"}
  camel-snake-kebab/camel-snake-kebab   {:mvn/version "0.4.3"}
  instaparse/instaparse                 {:mvn/version "1.5.0"}
  org.clojars.mmb90/cljs-cache          {:mvn/version "0.1.4"}
  fipp/fipp                             {:mvn/version "0.6.29"}
  metosin/malli                         {:git/url "https://github.com/metosin/malli" :sha "52ea58a36ff5172b38dfc526ca638afa7226a4a0"}
  com.cognitect/transit-cljs            {:mvn/version "0.8.280"}
  missionary/missionary                 {:mvn/version "b.47"}
  tick/tick                             {:mvn/version "1.0"}
  logseq/logseq-schema                  {:git/url "https://github.com/logseq/logseq-schema" :sha "6eeb51cd6d80bbffa0873c1e79790dc1f4ff68cf"}}

 :local/root deps
 {logseq/common          "deps/common"
  logseq/graph-parser    "deps/graph-parser"
  logseq/outliner        "deps/outliner"
  logseq/db-sync         "deps/db-sync"
  logseq/publishing       "deps/publishing"
  logseq/cli             "deps/cli"
  logseq/shui            "deps/shui"}}
```

## package.json (JavaScript/Node)

**Principales dependencias:**
- `react` - UI framework
- `@tanstack/react-query` - Data fetching
- `dnd-kit` - Drag and drop
- `vite` - Bundler
- `electron` - Desktop app
- `@capacitor/*` - Mobile

**DevDependencies:**
- `shadow-cljs` - ClojureScript compiler
- `playwright` - E2E testing
- `tailwindcss` - CSS framework
- `typescript` - Type system

## Dependencias Locales (deps/)

| Librería | Función |
|----------|---------|
| graph-parser | Parsea grafos de Logseq a DB |
| outliner | Sistema outliner jerárquico |
| db-sync | Sincronización de base de datos |
| common | Código compartido |
| publishing | Sistema de publicación |
| cli | Interfaz de línea de comandos |
| shui | Sistema de componentes UI |
