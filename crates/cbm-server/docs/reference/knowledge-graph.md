# 知識圖譜分析文檔 — codebase-memory-mcp

> 建立時間: 2026-06-12
> 資料來源: CBRLM 索引 + 原始碼分析

---

## 一、圖譜統計摘要

| 指標 | 數值 |
|------|------|
| 索引檔案數 | 1,657 |
| 符號總數 | 113 (93 Function + 20 Class) |
| 邊總數 | 64 (全為 CALLS 類型) |
| 索引引擎 | SQLite |
| 支援查詢 | search_graph, trace_path, query_graph |

### 節點標籤分布

```
Function ████████████████████████████████████████████ 93 (82.3%)
Class    ████████                                     20 (17.7%)
```

### 邊類型分布

```
CALLS ████████████████████████████████████████████████ 64 (100%)
```

---

## 二、SQLite 資料表結構

### symbols 表 — 符號節點

| 欄位 | 類型 | 說明 |
|------|------|------|
| `qualified_name` | TEXT | 完整路徑名 (PK) |
| `name` | TEXT | 簡短名稱 |
| `label` | TEXT | 標籤 (Function/Class/Module...) |
| `file_path` | TEXT | 相對檔案路徑 |
| `line_start` | INTEGER | 起始行 |
| `line_end` | INTEGER | 結束行 |
| `signature` | TEXT | 函數簽名 |

### edges 表 — 關係邊

| 欄位 | 類型 | 說明 |
|------|------|------|
| `src_qn` | TEXT | 來源節點 QN |
| `dst_qn` | TEXT | 目標節點 QN |
| `edge_type` | TEXT | 邊類型 (CALLS/IMPORTS/...) |

### files 表 — 檔案內容

| 欄位 | 類型 | 說明 |
|------|------|------|
| `path` | TEXT | 檔案路徑 |
| `content` | TEXT | 原始碼內容 |
| `language` | TEXT | 程式語言 |
| `line_count` | INTEGER | 行數 |

---

## 三、呼叫圖 (Call Graph)

### 3.1 Go 封裝層呼叫鏈

```
main (pkg/go/cmd/.../main.go)
├── ensureBinary
├── binPath
├── cacheDir
├── goos
├── goarch
├── download
├── validateURLScheme
├── httpGet
├── fetchChecksums
├── verifyChecksum
├── extractTarGz
├── extractZip
├── copyFile
└── execBinary
```

### 3.2 Python 封裝層呼叫鏈

```
_validate_url_scheme (pkg/pypi/src/.../_cli.py)
├── _safe_extract_tar
├── _safe_extract_zip
├── _verify_checksum
├── _version
├── _os_name
├── _arch
├── _cache_dir
├── _bin_path
├── _download
└── main
```

### 3.3 文件 HTML 渲染呼叫鏈

```
esc (docs/index.html)
├── renderMarkdown
├── inline
├── fmtDate
└── renderRelease
```

### 3.4 前端 UI 呼叫鏈

```
GraphScene (graph-ui/src/components/GraphScene.tsx)
└── computeCameraTarget

callTool (graph-ui/src/api/rpc.ts)
└── RpcError (Class)
```

### 3.5 C Preprocessor 呼叫鏈 (simplecpp)

```
simplecpp (internal/cbm/vendored/simplecpp/simplecpp.cpp)
├── StdIStream (Class)
├── StdCharBufStream (Class)
├── FileStream (Class)
├── Macro (Class)
└── NonExistingFilesCache (Class)
```

---

## 四、熱點函數 (High Fan-out)

| 函數 | 檔案 | 呼叫數 | 說明 |
|------|------|--------|------|
| `main` | `pkg/go/cmd/.../main.go` | 14 | Go 封裝層入口 |
| `_validate_url_scheme` | `pkg/pypi/src/.../_cli.py` | 10 | Python 封裝層入口 |
| `esc` | `docs/index.html` | 4 | HTML 文件轉義 |
| `is_method` | `scripts/gen-py-stdlib.py` | 10 | Python stub 分析方法 |
| `is_code_relevant` | `scripts/extract_nomic_vectors.py` | 9 | Nomic 嵌入過濾 |
| `GraphScene` | `graph-ui/src/.../GraphScene.tsx` | 1 | 3D 場景(含相機目標) |

---

## 五、資料流分析

### 5.1 索引資料流

```
原始碼檔案
    │
    ▼
tree-sitter AST 解析 ───→ cbm_gbuf (記憶體圖緩衝區)
    │                              │
    ├── Function/Class 節點         │
    ├── CALLS/IMPORTS 邊            │
    └── 屬性 (簽名、複雜度)          │
                                    │
                                    ▼
                              SQLite Store
                              (符號 + 邊 + 向量)
                                    │
                    ┌───────────────┼───────────────┐
                    ▼               ▼               ▼
              search_graph    trace_path      get_architecture
```

### 5.2 查詢資料流

```
MCP Client (Agent)
    │
    ▼ JSON-RPC (stdin/stdout)
cbm_mcp_server_t
    │
    ├── cbm_store_search()     → search_graph
    ├── cbm_store_bfs()       → trace_path
    ├── cbm_store_vector_search() → vector search
    ├── cbm_store_get_architecture() → get_architecture
    └── query_graph()          → SQL SELECT
    │
    ▼ JSON-RPC Response
MCP Client
```

---

## 六、語意嵌入系統

### 6.1 11 種相似度訊號

| # | 訊號 | 權重 | 說明 |
|---|------|------|------|
| 1 | TF-IDF | 0.15 | 中繼資料詞彙重疊 |
| 2 | Random Indexing (RI) | 0.15 | 共現語意橋接 |
| 3 | MinHash 結構 | 0.12 | 程式結構指紋 |
| 4 | API 簽名向量 | 0.10 | 相同 callee → 相關 |
| 5 | 型別簽名向量 | 0.10 | 相同參數/回傳型別 |
| 6 | 模組鄰近度 | x1.0-1.2 | 同目錄加成 |
| 7 | 裝飾器模式向量 | 0.08 | 相同註解/裝飾器 |
| 8 | AST 結構輪廓 | 0.10 | 控制流程形狀 |
| 9 | 近似資料流 | 0.08 | 參數→回傳/條件 |
| 10 | 圖擴散 | α=0.3 | 鄰居嵌入混合 |
| 11 | Halstead 輕量 | 0.05 | 運算子/運算元複雜度 |

### 6.2 向量維度
- Random Indexing: 768 維 (匹配 nomic-embed-code)
- 量化: int8 (每維 1 byte)
- 相似度閾值: 0.75 (SIMILAR_TO 邊)
- 每節點最大邊數: 10

---

## 七、社區偵測 (Leiden 演算法)

### 7.1 演算法
- 實作 Traag, Waltman & van Eck 2019 多層 Leiden 演算法
- 包含 Local Moving + Refinement + Aggregation
- 保證每個社區內部連通
- 解析度參數控制粒度 (預設 1.0)

### 7.2 API
```c
int cbm_leiden(const int64_t *nodes, int node_count,
               const cbm_louvain_edge_t *edges, int edge_count,
               double resolution,
               cbm_louvain_result_t **out, int *out_count);
```

---

## 八、ADR 架構決策記錄

支援架構決策記錄 (ADR) 的 CRUD 操作：
- 每個專案儲存一份 ADR
- 最大長度 8000 字元
- 支援章節解析/渲染 (`cbm_adr_parse_sections`, `cbm_adr_render`)
- 驗證內容和章節鍵名

### ADR 章節格式
```
---
## Title: <標題>
## Status: [Proposed|Accepted|Deprecated|Superseded]
## Context: <決策背景>
## Decision: <決策內容>
## Consequences: <影響>
---
```
