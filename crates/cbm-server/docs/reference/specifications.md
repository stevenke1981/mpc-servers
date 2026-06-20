# 規格說明文檔 — codebase-memory-mcp

> 建立時間: 2026-06-12
> 版本: dev (編譯時定義)

---

## 一、專案概述

**codebase-memory-mcp** 是一個 MCP (Model Context Protocol) 伺服器，為 AI 程式碼代理提供程式碼知識圖譜服務。它分析程式碼儲存庫、建立符號圖（函數、類別、呼叫關係等），並透過 JSON-RPC 2.0 over stdio 提供圖查詢工具。

### 1.1 核心功能

1. **程式碼索引** — 使用 tree-sitter 解析 AST，建立符號圖
2. **知識圖譜查詢** — 搜尋符號、追蹤呼叫鏈、查詢架構
3. **自動索引** — 監聽 Git 變更，自動 reindex
4. **語意嵌入** — 11 種訊號的相似度分析
5. **HTTP UI** — 可選的視覺化圖形介面 (port 9749)
6. **ADR 管理** — 架構決策記錄

---

## 二、執行模式

### 2.1 MCP 伺服器模式 (預設)

```
codebase-memory-mcp
```
- 在 stdin/stdout 上執行 JSON-RPC 2.0
- MCP 標準工具協定 (tools/list, tools/call, initialize)
- 支援 signals: SIGTERM, SIGINT 優雅關閉

### 2.2 CLI 模式

```
codebase-memory-mcp cli [--progress] [--json] <tool_name> [json_args]
```
- 執行單一 MCP 工具並列印結果
- `--progress`: 進度輸出到 stderr
- `--json`: 輸出原始 JSON

### 2.3 子命令

| 命令 | 說明 |
|------|------|
| `install [-y|-n] [--force] [--dry-run]` | 安裝 MCP 整合 |
| `uninstall [-y|-n] [--dry-run]` | 解除安裝 |
| `update [-y|-n]` | 更新 |
| `config <list\|get\|set\|reset>` | 設定管理 |
| `hook-augment` | hook 增強子命令 |

### 2.4 啟動選項

| 旗標 | 說明 |
|------|------|
| `--version` | 印出版本 |
| `--help` / `-h` | 印出說明 |
| `--ui=true/false` | 啟用/停用 HTTP UI (持久化) |
| `--port=N` | 設定 UI 埠號 (預設 9749，持久化) |
| `--profile` | 啟用效能分析 |

---

## 三、MCP 協定規格

### 3.1 JSON-RPC 2.0

- **傳輸層**: stdin/stdout (Content-Length 標頭)
- **編碼**: UTF-8
- **請求格式**: JSON-RPC 2.0
- **ID 格式**: 支援數字和字串 ID

### 3.2 工具協定

所有工具實作 `tools/call` 方法。

#### index_repository

**參數**:
- `repo_path` (string, 必填): 儲存庫絕對路徑
- `project` (string, 選填): 專案名稱
- `mode` (string, 選填): 索引模式 (`full` / `moderate` / `fast`)
- `persistence` (boolean, 選填): 寫入壓縮 artifact

**回應**: `{"success": true, "project": "...", "files_indexed": N, ...}`

#### search_graph

**參數**:
- `project` (string, 必填): 專案名稱
- `query` (string, 選填): 全文搜尋詞
- `label` (string, 選填): 標籤過濾 (`Function`, `Class`, ...)
- `name_pattern` (string, 選填): 名稱正則
- `qn_pattern` (string, 選填): QN 正則
- `file_pattern` (string, 選填): 檔案路徑 glob
- `relationship` (string, 選填): 邊類型
- `direction` (string, 選填): `inbound` / `outbound` / `any`
- `min_degree` / `max_degree` (int, 選填): 度數過濾
- `limit` (int, 選填, 預設: 200): 每頁結果
- `offset` (int, 選填): 分頁偏移

#### trace_path

**參數**:
- `project` (string, 必填): 專案名稱
- `function_name` (string, 必填): 起始 QN
- `direction` (string, 選填, 預設: `both`): `inbound` / `outbound` / `both`
- `depth` (int, 選填, 預設: 3): BFS 深度限制

#### get_code_snippet

**參數**:
- `qualified_name` (string, 必填): 符號 QN
- `project` (string, 選填): 專案名稱

#### get_architecture

**參數**:
- `project` (string, 必填): 專案名稱
- `aspects` (string[], 選填): 架構面向

#### query_graph

**參數**:
- `query` (string, 必填): SQL SELECT 語句
- `project` (string, 選填): 專案名稱

### 3.3 支援的 Agent (自動偵測)

| Agent | 說明 |
|-------|------|
| Claude Code | Anthropic |
| Codex CLI | OpenAI |
| Gemini CLI | Google |
| Zed | 編輯器內建 |
| OpenCode | 開源版 |
| Antigravity | 社群 |
| Aider | 開源版 |
| KiloCode | 社群 |
| Kiro | 社群 |

---

## 四、索引系統規格

### 4.1 索引模式

| 模式 | 說明 | 檔案過濾 | SIMILAR_TO | SEMANTICALLY_RELATED |
|------|------|---------|------------|---------------------|
| `FULL` | 完整索引 | 標準 | ✅ | ✅ |
| `MODERATE` | 中度索引 | 積極過濾 | ✅ | ✅ |
| `FAST` | 快速索引 | 積極過濾 | ❌ | ❌ |

### 4.2 檔案發現規則

- **永遠跳過**: `.git`, `node_modules`, `__pycache__`, `.venv` 等
- **快速模式跳過**: `dist/`, `build/`, `vendor/` (有條件)
- **忽略後綴**: `.pyc`, `.pyo`, `.exe`, `.dll`, `.so`, `.dylib`, `.o`, `.a`, `.lib`, 圖片, 影片, 音檔, 字型, 壓縮檔等
- **跳過檔名**: `LICENSE`, `go.sum`, `package-lock.json`, `yarn.lock` 等 (快速模式)
- **支援 `.cbmignore`**: 自訂忽略規則

### 4.3 Pipeline 最佳化

- **平行 worker pool**: 多執行緒 AST 解析
- **批次寫入**: bulk insert pragma 最佳化
- **增量索引**: 僅 reindex 變更檔案
- **記憶體預算**: 預設 50% 實體 RAM
- **WAL journal**: 崩潰安全

### 4.4 增量索引流程

1. Watcher 偵測 Git HEAD 變更或 dirty tree
2. 計算變更檔案列表 (git diff)
3. 從 store 刪除舊檔案節點/邊
4. 重新提取變更檔案
5. 重新解析呼叫和使用關係
6. 合併到 store

---

## 五、資料庫規格

### 5.1 SQLite Schema

```sql
-- 符號節點表
CREATE TABLE symbols (
    qualified_name TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    label TEXT NOT NULL,        -- Function/Class/Module/File/Package/Folder
    file_path TEXT NOT NULL,
    line_start INTEGER,
    line_end INTEGER,
    signature TEXT,
    properties_json TEXT         -- JSON 屬性
);

-- 關係邊表
CREATE TABLE edges (
    src_qn TEXT NOT NULL REFERENCES symbols(qualified_name),
    dst_qn TEXT NOT NULL REFERENCES symbols(qualified_name),
    edge_type TEXT NOT NULL,    -- CALLS/IMPORTS/INHERITS/...
    properties_json TEXT,
    UNIQUE(src_qn, dst_qn, edge_type)
);

-- 原始碼檔案表
CREATE TABLE files (
    path TEXT PRIMARY KEY,
    content TEXT,               -- 原始碼 (LZ4 壓縮)
    language TEXT,
    line_count INTEGER
);

-- 後設資料表
CREATE TABLE meta (
    key TEXT PRIMARY KEY,
    value TEXT
);
```

### 5.2 壓縮 artifact

- 路徑: `.codebase-memory/graph.db.zst`
- 使用 Zstandard 壓縮
- `pipeline_set_persistence(true)` 啟用

### 5.3 WAL 設定

- journal_mode: WAL
- mmap_size: 64 MB (可透過 `CBM_SQLITE_MMAP_SIZE` 環境變數設定)
- synchronous: NORMAL (批次寫入期間切換 OFF)

---

## 六、環境變數

| 變數 | 說明 | 預設值 |
|------|------|--------|
| `CBM_LOG_LEVEL` | 日誌層級 | info |
| `CBM_DIAGNOSTICS` | 診斷啟用 | 0 (disabled) |
| `CBM_PROFILE` | 效能分析啟用 | 0 (disabled) |
| `CBM_SEMANTIC_ENABLED` | 語意嵌入啟用 | 0 (disabled) |
| `CBM_SQLITE_MMAP_SIZE` | SQLite mmap 大小 (bytes) | 67108864 (64 MB) |

---

## 七、安全規格

### 7.1 URL 安全
- 僅允許 `https://` 和 `http://` scheme
- 阻擋 localhost/私有 IP 的 URL
- 路徑穿越防護 (`../`)

### 7.2 封裝層安全
- Go: URL scheme 驗證 + 路徑穿越防護
- Python: Tar 安全解壓 (`filter='data'`), Zip 路徑穿越檢查
- Checksum 驗證 (SHA256)

### 7.3 Gitignore 安全
- 路徑遍歷限制
- Gitignore 模式匹配安全處理

---

## 八、記憶體管理規格

### 8.1 分配器
- **主要分配器**: mimalloc (全域 override)
- **第三方綁定**: tree-sitter, sqlite3, libgit2 全部使用 mimalloc
- **初始化**: `cbm_alloc_init()` — 必須在 `main()` 的第一條語句

### 8.2 記憶體預算
- **預算計算**: `ram_fraction * total_physical_ram`
- **預設比例**: 0.5 (50%)
- **監控**: `mi_process_info()` RSS 追蹤
- **觸發回收**: RSS 超過預算時

### 8.3 其他分配器
- `cbm_arena_t`: 區域分配器 (scenario-based)
- `cbm_slab_t`: Slab 分配器 (固定大小物件)
- `hash_table`: Verstable (開源定址)

---

## 九、平行處理規格

### 9.1 Worker Pool (pipeline/worker_pool.h)

- 每個 pipeline pass 使用 worker pool 平行處理
- 每個 worker 處理單一檔案
- 使用 mutex 保護共享資源 (graph buffer)
- 可配置 worker 數量

### 9.2 執行緒模型

```
main thread:     MCP event loop / CLI
background 1:    Watcher (git polling)
background 2:    HTTP UI server (選用)
background 3:    Parent watchdog (POSIX only)
pipeline:        Worker pool (N threads)
```

### 9.3 同步原語

- `atomic_int`: 取消旗標、關閉旗標、pipeline 忙碌旗標
- `cbm_mutex_t` (`foundation/compat_thread.h`): 平台抽象 mutex
- `cbm_thread_t`: 平台抽象執行緒

---

## 十、平台相容性

### 10.1 支援平台

| 平台 | 支援狀態 |
|------|---------|
| Linux (x86_64, aarch64) | ✅ 完整支援 |
| macOS (x86_64, arm64) | ✅ 完整支援 |
| Windows (x86_64, arm64) | ✅ 完整支援 |

### 10.2 平台特定功能

| 功能 | Linux | macOS | Windows |
|------|-------|-------|---------|
| Parent watchdog | ✅ (pthread) | ✅ (pthread) | N/A (job object) |
| Signal handling | ✅ (sigaction) | ✅ (sigaction) | ✅ (signal) |
| HTTP UI | ✅ | ✅ | ✅ |
| Filesystem | POSIX | POSIX | Win32 API |
| 安裝路徑 | `~/.local/bin` | `~/.local/bin` | `%LOCALAPPDATA%` |

### 10.3 安裝方式

| 平台 | 方式 |
|------|------|
| Linux | `install.sh`，或 AUR (archlinux)、Homebrew/Linuxbrew |
| macOS | `install.sh`，或 Homebrew |
| Windows | `install.ps1`，或 Chocolatey、Scoop、Winget |
| 跨平台 | Go 封裝 (`go install`)、Python 封裝 (`pip install`)、npm 封裝 |

---

## 十一、效能指標

| 操作 | 預期效能 |
|------|---------|
| 小型專案索引 (<1000 檔案) | < 5 秒 |
| 中型專案索引 (<10000 檔案) | < 30 秒 |
| 大型專案索引 (<100000 檔案) | < 5 分鐘 |
| search_graph 查詢 | < 100ms |
| trace_path (depth=3) | < 50ms |
| get_architecture | < 200ms |
| 向量搜尋 (<50000 節點) | < 500ms |
| SQLite 資料庫大小 | 原始碼的 ~2-5x |

---

## 十二、限制與假設

### 12.1 已知限制

1. **單一 store 非執行緒安全** — 每個執行緒需要獨立 store 控制代碼
2. **Cypher 支援有限** — 僅 read-only SELECT (SQL 層)
3. **語意嵌入需顯式啟用** — `CBM_SEMANTIC_ENABLED=1`
4. **tree-sitter 語言支援** — 需編譯對應的 grammar
5. **HTTP UI 需特殊建置** — `make -f Makefile.cbm cbm-with-ui`

### 12.2 假設

1. 專案使用 Git 版本控制
2. 原始碼可讀取且有權限訪問
3. 足夠的 RAM 用於索引 (至少 1GB)
4. 檔案編碼為 UTF-8

---

## 十三、開發建置

### 13.1 建置系統

| 系統 | 說明 |
|------|------|
| `Makefile.cbm` | 主要建置 (GNU Make) |
| `flake.nix` | Nix 建置 |
| `CMakeLists.txt` | CMake 建置 (部分平臺) |

### 13.2 程式碼品質

| 工具 | 用途 |
|------|------|
| `.clang-format` | 格式化 (LLVM style) |
| `.clang-tidy` | 靜態分析 |
| `.cppcheck` | Cppcheck 配置 |
| `.gitleaksignore` | 金鑰洩漏掃描 |
| `test-infrastructure/` | 測試基礎設施 |

### 13.3 發布套件

| 格式 | 維護者 |
|------|--------|
| Standalone binary | 核心 |
| Go module (`go install`) | 社群 |
| PyPI (`pip install`) | 社群 |
| npm package | 社群 |
| Homebrew | 社群 |
| Chocolatey | 社群 |
| Scoop | 社群 |
| Winget | 社群 |
| AUR | 社群 |
| glama.ai | 官方 |

---

## 十四、版本相容性

### MCP 協定版本
- 實作 JSON-RPC 2.0
- 相容 MCP 標準工具協定
- 支援 MCP `initialize` 版本協商

### 資料庫相容性
- SQLite 3.x (使用 `sqlite3_api` 介面)
- Schema 變更需遷移處理
- WAL 格式跨版本相容
