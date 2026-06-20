# codebase-memory-mcp 系統架構文檔

> 建立時間: 2026-06-12
> 專案類型: C 語言 MCP 伺服器 + Go/Python 封裝層 + React 前端 UI

---

## 一、整體架構概覽

```
┌─────────────────────────────────────────────────────────────────────┐
│                        codebase-memory-mcp                          │
│              Model Context Protocol 程式碼知識圖譜伺服器              │
└─────────────────────────────────────────────────────────────────────┘

                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
   ┌────▼────┐          ┌────▼────┐          ┌────▼────┐
   │  C 核心  │          │ 封裝層   │          │ 前端 UI  │
   │ (src/)  │          │ (pkg/)  │          │(graph-ui)│
   └────┬────┘          └────┬────┘          └────┬────┘
        │                    │                     │
        │  ┌─────────────────┤                     │
        │  │  Go (pkg/go/)   │                   127.0.0.1:9749
        │  │  Python (pypi/) │                    HTTP API
        │  │  Homebrew       │
        │  │  Chocolatey     │
        │  │  Scoop/Winget   │
        │  │  npm/AUR        │
        │  └─────────────────┤
        │                    │
   ┌────▼────────────────────▼──────────────────────┐
   │              SQLite Database                    │
   │          (~/.cache/codebase-memory/)            │
   │     symbols + edges + files + vectors           │
   └─────────────────────────────────────────────────┘
```

---

## 二、C 核心模組架構 (src/)

### 2.1 模組依賴圖

```
                       main.c
                          │
          ┌───────────────┼───────────────┐
          │               │               │
     ┌────▼────┐    ┌────▼────┐    ┌────▼────┐
     │   CLI   │    │   MCP   │    │ Watcher │
     │ (cli/)  │    │ (mcp/)  │    │(watcher/)│
     └────┬────┘    └────┬────┘    └────┬────┘
          │              │              │
          │        ┌─────▼─────┐        │
          │        │ Pipeline  │◄───────┘
          │        │(pipeline/)│   (觸發 reindex)
          │        └─────┬─────┘
          │              │
    ┌─────┼──────┬───────┼───────┬───────┐
    │     │      │       │       │       │
┌───▼──┐ ┌▼───┐ ┌▼────┐ ┌▼────┐ ┌▼────┐ ┌▼───────┐
│Store │ │GBuf│ │Sem. │ │Disc.│ │Cyp. │ │Foundation│
│store/│ │gbuf│ │sem. │ │disc │ │cyph │ │  (38檔) │
└──────┘ └────┘ └─────┘ └─────┘ └─────┘ └─────────┘
                                             │
                          ┌──────────────────┼──────────────┐
                          ▼                  ▼              ▼
                     ┌────────┐       ┌──────────┐    ┌─────────┐
                     │hash_   │       │   mem    │    │  arena  │
                     │table   │       │(mimalloc)│    │(slab)   │
                     └────────┘       └──────────┘    └─────────┘
```

### 2.2 核心模組說明

| 模組 | 目錄 | 職責 | 關鍵檔案 |
|------|------|------|---------|
| **Entry** | `src/main.c` | 程式入口、信號處理、背景執行緒管理 | `main.c` |
| **MCP Server** | `src/mcp/` | JSON-RPC 2.0 over stdio，提供 14+ 個 MCP 工具 | `mcp.c` (4744行), `mcp.h` |
| **Pipeline** | `src/pipeline/` | 多階段索引管線：發現→結構→定義→呼叫→使用→語意 | `pipeline.c/h` + 20 個 pass |
| **Store** | `src/store/` | SQLite 圖資料庫：節點/邊/檔案/向量 CRUD + BFS + 搜尋 | `store.c/h` (669 行 API) |
| **Graph Buffer** | `src/graph_buffer/` | 記憶體中圖緩衝區，索引期間持有節點/邊 | `graph_buffer.c/h` |
| **CLI** | `src/cli/` | 命令列工具、hook-augment、progress sink | `cli.c/h`, `hook_augment.c` |
| **Watcher** | `src/watcher/` | Git 變更輪詢、自動 reindex | `watcher.c/h` |
| **Discover** | `src/discover/` | 檔案發現、語言偵測、gitignore 匹配 | `discover.c/h`, `language.c` |
| **Semantic** | `src/semantic/` | 語意嵌入：11 種訊號的相似度評分 | `semantic.c/h`, `ast_profile.c/h` |
| **Cypher** | `src/cypher/` | 類 Cypher 查詢支援（read-only SELECT） | `cypher.c/h` |
| **Foundation** | `src/foundation/` | 基礎設施：記憶體、日誌、Hash 表、字串、平台相容 | 38 個檔案 |

### 2.3 Pipeline 索引流程

```
     ┌──────────┐
     │ Discover │  掃描目錄樹，套用 gitignore/.cbmignore，偵測語言
     └─────┬────┘
           ▼
     ┌──────────┐
     │Structure │  建立 Project/Folder/Package/File 節點
     └─────┬────┘
           ▼
     ┌──────────┐
     │Bulk Load │  讀取原始碼 + LZ4 HC 壓縮
     └─────┬────┘
           ▼
     ┌──────────┐
     │Extract   │  用 tree-sitter 解析 AST → 寫節點 + 建立 Registry
     └─────┬────┘
           ▼
     ┌──────────┐
     │ Imports  │  解析 IMPORT 邊
     └─────┬────┘
           ▼
     ┌──────────┐
     │  Calls   │  呼叫解析 (Registry + LSP cross-file)
     └─────┬────┘
           ▼
     ┌──────────┐
     │ Usages   │  Usage/TypeRef 邊
     └─────┬────┘
           ▼
     ┌──────────┐
     │ Semantic │  Inherits/Decorates/Implements + RI 語意嵌入
     └─────┬────┘
           ▼
     ┌──────────┐
     │  Post    │  Tests, Communities (Leiden), HTTP links, Config, Git history
     └─────┬────┘
           ▼
     ┌──────────┐
     │ Dump to  │  寫入 SQLite (graph_buffer → store)
     │ SQLite   │
     └──────────┘
```

### 2.4 Watcher 自動索引

```
     ┌──────────────┐
     │  cbm_watcher  │  背景執行緒，每隔 base_interval_ms (5s base)
     │  .poll_once() │  檢查每個 watch project 的 git HEAD 變動
     └──────┬───────┘
            │ 有變更？
            ▼
     ┌──────────────┐
     │  Pipeline     │  cbm_pipeline_try_lock() → run()
     │  reindex      │  非封鎖：忙碌則跳過，下次再試
     └──────────────┘
```

---

## 三、知識圖譜資料模型

### 3.1 節點 (Symbols/Nodes)

| 欄位 | 類型 | 說明 |
|------|------|------|
| `id` | int64 | 唯一 ID |
| `project` | string | 專案名稱 |
| `label` | string | Function / Class / Module / File / Package / Folder |
| `name` | string | 簡短名稱 |
| `qualified_name` | string | 完整路徑名 (e.g. `pkg.main::main`) |
| `file_path` | string | 相對檔案路徑 |
| `start_line` | int | 起始行號 |
| `end_line` | int | 結束行號 |
| `properties_json` | string | JSON 屬性 (簽名、複雜度... ) |

### 3.2 邊 (Edges/Relationships)

| 邊類型 | 說明 | 來源→目標 |
|--------|------|----------|
| `CALLS` | 函數呼叫 | Caller → Callee |
| `HTTP_CALLS` | HTTP 呼叫 | Client → Endpoint |
| `ASYNC_CALLS` | 非同步呼叫 | Source → Target |
| `IMPORTS` | 匯入關係 | File → Module/Symbol |
| `INHERITS` | 繼承關係 | Class → Parent Class |
| `DECORATES` | 裝飾器 | Decorator → Target |
| `IMPLEMENTS` | 實作介面 | Class → Interface |
| `CONTAINS` | 容器關係 | Module → Symbol |
| `SIMILAR_TO` | 語意相似 (>=0.75) | Function → Function |
| `SEMANTICALLY_RELATED` | 語意相關 | Symbol → Symbol |
| `HTTP_ROUTE` | HTTP 路由 | Route → Handler |

### 3.3 向量搜尋

- 768 維 int8 量化向量
- 基於 Random Indexing + TF-IDF 的語意嵌入
- 支援 `cbm_store_vector_search()` 進行 cosine scan

---

## 四、MCP 工具清單

| 工具名稱 | 說明 |
|---------|------|
| `index_repository` | 索引一個儲存庫 |
| `search_graph` | 搜尋圖節點（支援多種過濾器） |
| `query_graph` | SQL 查詢（SELECT on symbols/edges/files） |
| `trace_path` | BFS 路徑追蹤（呼叫鏈） |
| `get_code_snippet` | 取得節點原始碼 |
| `get_graph_schema` | 圖結構 schema |
| `get_architecture` | 專案架構摘要 |
| `search_code` | 全文程式碼搜尋 |
| `list_projects` | 列出已索引專案 |
| `delete_project` | 刪除專案索引 |
| `index_status` | 索引狀態查詢 |
| `detect_changes` | Git 變更偵測 |
| `manage_adr` | 架構決策記錄 CRUD |
| `ingest_traces` | 執行期軌跡導入 |
| `manage_adr_sections` | ADR 章節管理 |

---

## 五、Foundation 基礎設施

### 5.1 記憶體管理 (`foundation/mem.h`)
- 基於 mimalloc 的統一記憶體管理
- RSS 預算追蹤 (`mi_process_info()`)
- 預設使用 50% 實體 RAM

### 5.2 雜湊表 (`foundation/hash_table.h`)
- 採用 Verstable (2024) 開源定址雜湊表
- 二次探測 + per-bucket 4-bit hash fragments
- 字串鍵不複製（borrowed pointers）

### 5.3 其他工具
| 模組 | 說明 |
|------|------|
| `arena.c/h` | 記憶體池分配器 |
| `log.c/h` | 結構化日誌系統 |
| `platform.c/h` | 平台抽象層 |
| `compat*.c/h` | 相容層 (檔案系統、執行緒、正則) |
| `str_util.c/h` | 字串工具函數 |
| `str_intern.c/h` | 字串實習池 |
| `profile.c/h` | 效能分析支援 |
| `diagnostics.c/h` | 診斷資訊 |
| `slab_alloc.c/h` | Slab 分配器 |
| `vmem.c/h` | 虛擬記憶體 |
| `yaml.c/h` | YAML 解析器 |

---

## 六、Pipeline Passes 詳解

| Pass | 檔案 | 功能 |
|------|------|------|
| `pass_definitions` | `pass_definitions.c` | tree-sitter AST 解析，提取函數/類別/方法定義 |
| `pass_calls` | `pass_calls.c` | CALLS 邊解析（註冊表 + LSP） |
| `pass_usages` | `pass_usages.c` | Usage/TypeRef 邊 |
| `pass_semantic` | `pass_semantic.c` | 語意相似度 SIMILAR_TO |
| `pass_semantic_edges` | `pass_semantic_edges.c` | SEMANTICALLY_RELATED 邊 |
| `pass_parallel` | `pass_parallel.c` | 平行 worker pool 分發 |
| `pass_complexity` | `pass_complexity.c` | 程式複雜度分析 |
| `pass_configlink` | `pass_configlink.c` | 配置連結分析 |
| `pass_configures` | `pass_configures.c` | Build 配置分析 |
| `pass_cross_repo` | `pass_cross_repo.c` | 跨儲存庫引用 |
| `pass_envscan` | `pass_envscan.c` | 環境變數掃描 |
| `pass_gitdiff` | `pass_gitdiff.c` | Git diff 分析 |
| `pass_githistory` | `pass_githistory.c` | Git 歷史分析 |
| `pass_infrascan` | `pass_infrascan.c` | 基礎設施掃描 |
| `pass_k8s` | `pass_k8s.c` | Kubernetes 資源分析 |
| `pass_lsp_cross` | `pass_lsp_cross.c` | LSP cross-file 解析 |
| `pass_pkgmap` | `pass_pkgmap.c` | 套件映射 |
| `pass_route_nodes` | `pass_route_nodes.c` | HTTP 路由節點 |
| `pass_tests` | `pass_tests.c` | 測試關聯分析 |
| `pass_similarity` | `pass_similarity.c` | MinHash 結構相似度 |

---

## 七、支援語言

支援 64+ 種程式語言，透過 tree-sitter 解析 AST：

C, C++, C#, Go, Rust, Python, JavaScript, TypeScript, Java, Kotlin,
Swift, Ruby, PHP, Zig, Odin, Magma, Lua, Haskell, Scala, Dart,
Elixir, Erlang, Julia, R, Shell (Bash/Zsh), PowerShell, Perl,
Terraform, Dockerfile, YAML, JSON, TOML, CMake, Makefile, Protocol Buffers,
Thrift, GraphQL, SQL, HTML, CSS, SCSS, Svelte, Vue, Astro, Gleam,
Mojo, Move, Cairo, Solidity, Vyper, Nix, Cabal, Elisp, Julia,
Common Lisp, Clojure, F#, OCaml, COBOL, Fortran, Ada, Meson ...

---

## 八、外部依賴

| 依賴 | 用途 |
|------|------|
| [mimalloc](https://github.com/microsoft/mimalloc) | 高效記憶體分配器 |
| [tree-sitter](https://tree-sitter.github.io/) | AST 解析（支援 64+ 語言） |
| [sqlite3](https://www.sqlite.org/) | 圖資料庫儲存 |
| [libgit2](https://libgit2.org/) | Git 操作 |
| [yyjson](https://github.com/ibireme/yyjson) | 高效 JSON 解析 |
| [liblz4](https://lz4.github.io/lz4/) | 原始碼壓縮 |
| [Verstable](https://github.com/JacksonAllan/Verstable) | 開源定址雜湊表 |

---

## 九、封裝層 (pkg/)

| 封裝 | 目錄 | 說明 |
|------|------|------|
| **Go** | `pkg/go/` | Go CLI 封裝，下載二進位檔並執行 |
| **Python** | `pkg/pypi/` | PyPI 套件，Python CLI 封裝 |
| **Homebrew** | `pkg/homebrew/` | macOS Homebrew formula |
| **Chocolatey** | `pkg/chocolatey/` | Windows Chocolatey 套件 |
| **Scoop** | `pkg/scoop/` | Windows Scoop manifest |
| **Winget** | `pkg/winget/` | Windows Winget 套件 |
| **npm** | `pkg/npm/` | npm 套件封裝 |
| **AUR** | `pkg/aur/` | Arch Linux AUR |
| **glama** | `pkg/glama/` | Glama.ai 整合 |

---

## 十、前端 UI (graph-ui/)

```
graph-ui/
├── src/
│   ├── api/rpc.ts          # MCP JSON-RPC 客戶端
│   ├── App.tsx             # 主元件
│   ├── components/
│   │   ├── GraphScene.tsx  # 3D 圖形場景 (Three.js)
│   │   ├── ControlTab.tsx  # 控制面板
│   │   ├── StatsTab.tsx    # 統計面板
│   │   ├── Sidebar.tsx     # 側邊欄
│   │   ├── FilterPanel.tsx # 過濾面板
│   │   ├── NodeCloud.tsx   # 節點雲
│   │   ├── EdgeLines.tsx   # 邊線渲染
│   │   └── ...
│   ├── hooks/
│   │   ├── useGraphData.ts # 圖資料 Hook
│   │   └── useProjects.ts  # 專案列表 Hook
│   └── lib/
│       ├── colors.ts       # 節點配色
│       └── utils.ts        # 工具函數
└── ...
```
