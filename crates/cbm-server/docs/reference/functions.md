# 函數目錄 — codebase-memory-mcp

> 建立時間: 2026-06-12
> 共 93 個 Function 節點 + 20 個 Class 節點

---

## 一、C 核心模組函數

### 1.1 入口點 (src/main.c)

| 函數 | 行號 | 說明 |
|------|------|------|
| `request_shutdown` | 70-93 | 冪等關閉：取消 pipeline、停止 watcher/HTTP、關閉 stdin |
| `signal_handler` | 95-98 | SIGTERM/SIGINT 信號處理器 |
| `parent_watchdog_thread` | 111-129 | 父行程看門狗執行緒 (POSIX only) |
| `watcher_thread` | 134-140 | Watcher 背景執行緒 |
| `http_thread` | 144-148 | HTTP UI 背景執行緒 |
| `watcher_index_fn` | 152-179 | Watcher 回調：觸發 pipeline reindex |
| `cli_print_mcp_result` | 188-214 | 列印 MCP 工具結果 |
| `cli_strip_flag` | 217-229 | 從 argv 移除旗標 |
| `run_cli` | 231-278 | CLI 模式執行 |
| `print_help` | 282-304 | 印出說明 |
| `handle_subcommand` | 310-348 | 處理子命令分發 |
| `parse_ui_flags` | 351-370 | 解析 --ui / --port 旗標 |
| `setup_signal_handlers` | 373-385 | 安裝信號處理器 |
| `main` | 387-549 | 程式主入口 |

### 1.2 MCP 伺服器 (src/mcp/)

| 函數 | 檔案 | 說明 |
|------|------|------|
| `cbm_jsonrpc_parse` | mcp.c | 解析 JSON-RPC 請求行 |
| `cbm_jsonrpc_request_free` | mcp.c | 釋放 JSON-RPC 請求 |
| `cbm_jsonrpc_format_response` | mcp.c | 格式化 JSON-RPC 回應 |
| `cbm_jsonrpc_format_error` | mcp.c | 格式化 JSON-RPC 錯誤 |
| `cbm_mcp_text_result` | mcp.c | 格式化 MCP 工具結果 |
| `cbm_mcp_tools_list` | mcp.c | 產生 tools/list 回應 |
| `cbm_mcp_initialize_response` | mcp.c | 初始化回應 |
| `cbm_mcp_get_string_arg` | mcp.c | 提取字串引數 |
| `cbm_mcp_get_int_arg` | mcp.c | 提取整數引數 |
| `cbm_mcp_get_bool_arg` | mcp.c | 提取布林引數 |
| `cbm_mcp_get_tool_name` | mcp.c | 提取工具名稱 |
| `cbm_mcp_get_arguments` | mcp.c | 提取引數子物件 |
| `cbm_mcp_server_new` | mcp.c | 建立 MCP 伺服器 |
| `cbm_mcp_server_free` | mcp.c | 釋放 MCP 伺服器 |
| `cbm_mcp_server_set_watcher` | mcp.c | 設定 watcher |
| `cbm_mcp_server_set_config` | mcp.c | 設定 config |
| `cbm_mcp_server_run` | mcp.c | 執行事件迴圈 |
| `cbm_mcp_server_handle` | mcp.c | 處理單一請求 |
| `cbm_mcp_handle_tool` | mcp.c | 處理工具呼叫 |
| `cbm_mcp_server_evict_idle` | mcp.c | 閒置 store 淘汰 |
| `cbm_mcp_server_has_cached_store` | mcp.c | 檢查快取 store |
| `cbm_mcp_server_store` | mcp.c | 取得 store 控制代碼 |
| `cbm_mcp_server_set_project` | mcp.c | 設定專案名稱 |
| `cbm_mcp_server_active_pipeline` | mcp.c | 取得執行中 pipeline |
| `cbm_parse_file_uri` | mcp.c | 解析 file:// URI |

### 1.3 Pipeline 索引管線 (src/pipeline/)

| 函數 | 檔案 | 說明 |
|------|------|------|
| `cbm_pipeline_try_lock` | pipeline.c | 非封鎖嘗試取得鎖 |
| `cbm_pipeline_lock` | pipeline.c | 封鎖等待鎖 |
| `cbm_pipeline_unlock` | pipeline.c | 釋放鎖 |
| `cbm_pipeline_new` | pipeline.c | 建立 pipeline |
| `cbm_pipeline_free` | pipeline.c | 釋放 pipeline |
| `cbm_pipeline_set_persistence` | pipeline.c | 啟用持續性匯出 |
| `cbm_pipeline_run` | pipeline.c | 執行完整索引 |
| `cbm_pipeline_cancel` | pipeline.c | 請求取消 |
| `cbm_pipeline_project_name` | pipeline.c | 取得專案名稱 |
| `cbm_pipeline_get_mode` | pipeline.c | 取得索引模式 |
| `cbm_pipeline_get_excluded` | pipeline.c | 取得排除目錄 |
| `cbm_pipeline_fqn_compute` | pipeline/fqn.c | 計算合格名稱 |
| `cbm_pipeline_fqn_module` | pipeline/fqn.c | 模組 QN |
| `cbm_pipeline_fqn_folder` | pipeline/fqn.c | 資料夾 QN |
| `cbm_pipeline_resolve_relative_import` | pipeline/fqn.c | 解析相對匯入 |
| `cbm_project_name_from_path` | pipeline/fqn.c | 從路徑衍生專案名 |
| `cbm_registry_new` | pipeline/registry.c | 建立函數註冊表 |
| `cbm_registry_free` | pipeline/registry.c | 釋放註冊表 |
| `cbm_registry_add` | pipeline/registry.c | 註冊函數/方法/類別 |
| `cbm_registry_resolve` | pipeline/registry.c | 解析 callee 名稱 |
| `cbm_registry_exists` | pipeline/registry.c | 檢查 QN 是否存在 |
| `cbm_registry_label_of` | pipeline/registry.c | 取得 QN 的標籤 |
| `cbm_registry_find_by_name` | pipeline/registry.c | 依名稱查找 QN |
| `cbm_registry_size` | pipeline/registry.c | 註冊表大小 |
| `cbm_registry_find_ending_with` | pipeline/registry.c | 後綴匹配 |
| `cbm_registry_is_import_reachable` | pipeline/registry.c | 檢查匯入可達性 |
| `cbm_registry_fuzzy_resolve` | pipeline/registry.c | 模糊解析 |
| `cbm_confidence_band` | pipeline/registry.c | 信心區間字串 |

### 1.4 SQLite Store (src/store/)

| 函數 | 說明 |
|------|------|
| `cbm_store_open_memory` | 開啟記憶體資料庫 |
| `cbm_store_open_path` | 開啟檔案資料庫 |
| `cbm_store_open_path_query` | 唯讀開啟 |
| `cbm_store_check_integrity` | 完整性檢查 |
| `cbm_store_open` | 依專案名稱開啟 |
| `cbm_store_close` | 關閉 store |
| `cbm_store_get_db` | 取得 sqlite3 控制代碼 |
| `cbm_store_error` | 取得錯誤訊息 |
| `cbm_store_begin` | 開始交易 |
| `cbm_store_commit` | 提交交易 |
| `cbm_store_rollback` | 回滾交易 |
| `cbm_store_begin_bulk` | 批次寫入最佳化 |
| `cbm_store_end_bulk` | 還原一般 pragma |
| `cbm_store_drop_indexes` | 刪除索引 |
| `cbm_store_create_indexes` | 重建索引 |
| `cbm_store_checkpoint` | WAL checkpoint |
| `cbm_store_resolve_mmap_size` | 解析 mmap 大小 |
| `cbm_store_dump_to_file` | 傾印到檔案 |
| `cbm_store_upsert_project` | 新增/更新專案 |
| `cbm_store_get_project` | 取得專案資訊 |
| `cbm_store_list_projects` | 列出專案 |
| `cbm_store_delete_project` | 刪除專案 |
| `cbm_store_upsert_node` | 新增/更新節點 |
| `cbm_store_upsert_node_batch` | 批次新增節點 |
| `cbm_store_find_node_by_id` | 依 ID 找節點 |
| `cbm_store_find_node_by_qn` | 依 QN 找節點 |
| `cbm_store_find_node_by_qn_any` | 全域 QN 查找 |
| `cbm_store_find_nodes_by_name` | 依名稱找節點 |
| `cbm_store_find_nodes_by_name_any` | 全域名稱查找 |
| `cbm_store_find_nodes_by_label` | 依標籤找節點 |
| `cbm_store_find_nodes_by_file` | 依檔案找節點 |
| `cbm_store_find_node_ids_by_qns` | 批次 QN→ID |
| `cbm_store_count_nodes` | 計數節點 |
| `cbm_store_delete_nodes_by_project` | 刪除專案節點 |
| `cbm_store_delete_nodes_by_file` | 依檔案刪除節點 |
| `cbm_store_delete_nodes_by_label` | 依標籤刪除節點 |
| `cbm_store_insert_edge` | 插入邊 |
| `cbm_store_insert_edge_batch` | 批次插入邊 |
| `cbm_store_find_edges_by_source` | 依來源找邊 |
| `cbm_store_find_edges_by_target` | 依目標找邊 |
| `cbm_store_find_edges_by_source_type` | 依來源+類型找邊 |
| `cbm_store_find_edges_by_target_type` | 依目標+類型找邊 |
| `cbm_store_find_edges_by_type` | 依類型找邊 |
| `cbm_store_count_edges` | 計數邊 |
| `cbm_store_count_edges_by_type` | 依類型計數邊 |
| `cbm_store_search` | 搜尋節點 |
| `cbm_store_bfs` | BFS 遍歷 |
| `cbm_store_get_schema` | 取得 schema |
| `cbm_store_get_schema_counts` | 取得計數 schema |
| `cbm_store_get_architecture` | 取得架構資訊 |
| `cbm_store_vector_search` | 向量搜尋 |
| `cbm_store_count_vectors` | 計數向量 |
| `cbm_store_exec` | 執行 SQL 語句 |
| `cbm_leiden` | Leiden 社區偵測 |
| `cbm_louvain` | Louvain 社區偵測 (res=1.0) |

### 1.5 Graph Buffer (src/graph_buffer/)

| 函數 | 說明 |
|------|------|
| `cbm_gbuf_new` | 建立圖緩衝區 |
| `cbm_gbuf_new_shared_ids` | 建立共享 ID 源緩衝區 |
| `cbm_gbuf_free` | 釋放緩衝區 |
| `cbm_gbuf_merge` | 合併緩衝區 |
| `cbm_gbuf_upsert_node` | 新增/更新節點 |
| `cbm_gbuf_find_by_qn` | 依 QN 查找 |
| `cbm_gbuf_find_by_id` | 依 ID 查找 |
| `cbm_gbuf_find_by_label` | 依標籤查找 |
| `cbm_gbuf_find_by_name` | 依名稱查找 |
| `cbm_gbuf_node_count` | 節點計數 |
| `cbm_gbuf_next_id` | 下一個 ID |
| `cbm_gbuf_set_next_id` | 設定 ID 計數器 |
| `cbm_gbuf_delete_by_label` | 依標籤刪除 |
| `cbm_gbuf_delete_by_file` | 依檔案刪除 |
| `cbm_gbuf_load_from_db` | 從 SQLite 載入 |
| `cbm_gbuf_foreach_node` | 迭代節點 |
| `cbm_gbuf_foreach_edge` | 迭代邊 |
| `cbm_gbuf_insert_edge` | 插入邊 |
| `cbm_gbuf_find_edges_by_source_type` | 依來源+類型找邊 |
| `cbm_gbuf_find_edges_by_target_type` | 依目標+類型找邊 |
| `cbm_gbuf_find_edges_by_type` | 依類型找邊 |
| `cbm_gbuf_edge_count` | 邊計數 |
| `cbm_gbuf_edge_count_by_type` | 依類型計數邊 |
| `cbm_gbuf_delete_edges_by_type` | 依類型刪除邊 |
| `cbm_gbuf_store_vector` | 儲存向量 |
| `cbm_gbuf_store_token_vector` | 儲存 token 向量 |
| `cbm_gbuf_dump_to_sqlite` | 傾印到 SQLite |
| `cbm_gbuf_flush_to_store` | 寫入 store |
| `cbm_gbuf_merge_into_store` | 合併到 store |

### 1.6 Watcher (src/watcher/)

| 函數 | 說明 |
|------|------|
| `cbm_watcher_new` | 建立 watcher |
| `cbm_watcher_free` | 釋放 watcher |
| `cbm_watcher_watch` | 加入監看清單 |
| `cbm_watcher_unwatch` | 移除監看 |
| `cbm_watcher_touch` | 更新時間戳 (重置退避) |
| `cbm_watcher_poll_once` | 執行單次輪詢 |
| `cbm_watcher_run` | 執行輪詢迴圈 |
| `cbm_watcher_stop` | 請求停止 |
| `cbm_watcher_watch_count` | 監看專案數 |
| `cbm_watcher_poll_interval_ms` | 適應性輪詢間隔 |

### 1.7 Discover (src/discover/)

| 函數 | 說明 |
|------|------|
| `cbm_language_for_filename` | 依檔名偵測語言 |
| `cbm_language_for_extension` | 依副檔名偵測語言 |
| `cbm_language_name` | 取得語言名稱 |
| `cbm_disambiguate_m` | 區分 .m 檔案類型 |
| `cbm_gitignore_load` | 載入 gitignore |
| `cbm_gitignore_parse` | 解析 gitignore 字串 |
| `cbm_gitignore_matches` | 檢查路徑匹配 |
| `cbm_gitignore_free` | 釋放 gitignore |
| `cbm_should_skip_dir` | 是否跳過目錄 |
| `cbm_has_ignored_suffix` | 是否忽略後綴 |
| `cbm_should_skip_filename` | 是否跳過檔名 |
| `cbm_matches_fast_pattern` | 快速模式匹配 |
| `cbm_discover` | 檔案發現 |
| `cbm_discover_ex` | 檔案發現 (含排除清單) |
| `cbm_discover_free` | 釋放檔案列表 |
| `cbm_discover_free_excluded` | 釋放排除清單 |

### 1.8 Semantic (src/semantic/)

| 函數 | 說明 |
|------|------|
| `cbm_sem_get_config` | 取得語意設定 |
| `cbm_sem_is_enabled` | 檢查語意嵌入啟用 |
| `cbm_sem_tokenize` | token 化 |
| `cbm_sem_cosine` | cosine 相似度 |
| `cbm_sem_random_index` | 產生隨機索引向量 |
| `cbm_sem_ensure_ready` | 預先初始化查詢表 |
| `cbm_sem_normalize` | L2 正規化 |
| `cbm_sem_vec_add_scaled` | 向量加法 (含縮放) |
| `cbm_sem_corpus_new` | 建立語料庫 |
| `cbm_sem_corpus_add_doc` | 註冊文件 token |
| `cbm_sem_corpus_add_docs_batch` | 批次註冊 |
| `cbm_sem_corpus_finalize` | 完成 IDF 計算 |
| `cbm_sem_corpus_idf` | 取得 IDF 權重 |
| `cbm_sem_corpus_ri_vec` | 取得 RI 向量 |
| `cbm_sem_corpus_doc_count` | 文件數 |
| `cbm_sem_corpus_token_count` | token 數 |
| `cbm_sem_corpus_token_at` | 依索引取得 token |
| `cbm_sem_corpus_free` | 釋放語料庫 |
| `cbm_sem_combined_score` | 組合相似度評分 |
| `cbm_sem_proximity` | 模組鄰近度乘數 |
| `cbm_sem_diffuse` | 圖擴散 |

---

## 二、前端 UI 函數 (graph-ui/)

### 2.1 React 元件

| 函數 | 檔案 | 說明 |
|------|------|------|
| `App` | `App.tsx` | 主應用元件 |
| `GraphScene` | `components/GraphScene.tsx` | 3D 圖形場景 |
| `computeCameraTarget` | `components/GraphScene.tsx` | 相機目標計算 |
| `EdgeLines` | `components/EdgeLines.tsx` | 邊線渲染 |
| `NodeLabels` | `components/NodeLabels.tsx` | 節點標籤 |
| `NodeTooltip` | `components/NodeTooltip.tsx` | 節點提示 |
| `NodeCloud` | `components/NodeCloud.tsx` | 節點雲 |
| `NodeDetailPanel` | `components/NodeDetailPanel.tsx` | 節點詳情面板 |
| `ControlTab` | `components/ControlTab.tsx` | 控制分頁 |
| `StatsTab` | `components/StatsTab.tsx` | 統計分頁 |
| `GraphTab` | `components/GraphTab.tsx` | 圖形分頁 |
| `Sidebar` | `components/Sidebar.tsx` | 側邊欄 |
| `FilterPanel` | `components/FilterPanel.tsx` | 過濾面板 |
| `ProjectCard` | `components/ProjectCard.tsx` | 專案卡片 |
| `ResizeHandle` | `components/ResizeHandle.tsx` | 縮放控制 |
| `ErrorBoundary` | `components/ErrorBoundary.tsx` | 錯誤邊界 |

### 2.2 Hooks

| 函數 | 說明 |
|------|------|
| `useGraphData` | 圖資料擷取 Hook |
| `useProjects` | 專案列表 Hook |

### 2.3 工具

| 函數 | 說明 |
|------|------|
| `callTool` | MCP JSON-RPC 工具呼叫 |
| `colorForLabel` | 標籤配色 |
| `cn` | CSS 類別合併 |

---

## 三、Go 封裝層 (pkg/go/)

| 函數 | 說明 |
|------|------|
| `main` | Go 封裝入口 |
| `ensureBinary` | 確保二進位存在 |
| `binPath` | 取得二進位路徑 |
| `cacheDir` | 取得快取目錄 |
| `goos` | 取得作業系統 |
| `goarch` | 取得架構 |
| `download` | 下載二進位 |
| `validateURLScheme` | 驗證 URL scheme |
| `httpGet` | HTTP GET 請求 |
| `fetchChecksums` | 取得 checksums |
| `verifyChecksum` | 驗證 checksum |
| `extractTarGz` | 解壓 tar.gz |
| `extractZip` | 解壓 zip |
| `copyFile` | 複製檔案 |
| `execBinary` | 執行二進位 |

---

## 四、Python 封裝層 (pkg/pypi/)

| 函數 | 說明 |
|------|------|
| `_validate_url_scheme` | 驗證 URL scheme |
| `_safe_extract_tar` | 安全解壓 tar |
| `_safe_extract_zip` | 安全解壓 zip |
| `_verify_checksum` | 驗證 checksum |
| `_version` | 取得版本 |
| `_os_name` | 取得 OS 名稱 |
| `_arch` | 取得架構 |
| `_cache_dir` | 取得快取目錄 |
| `_bin_path` | 取得二進位路徑 |
| `_download` | 下載二進位 |
| `main` | Python 封裝入口 |

---

## 五、Class 節點

| 類別 | 檔案 | 說明 |
|------|------|------|
| `RpcError` | `graph-ui/src/api/rpc.ts` | RPC 錯誤類別 |
| `ErrorBoundary` | `graph-ui/src/components/ErrorBoundary.tsx` | React 錯誤邊界 |
| `simplecpp` | `internal/.../simplecpp.cpp` | C preprocessor (namespace) |
| `StdIStream` | `internal/.../simplecpp.cpp` | 標準輸入串流 |
| `StdCharBufStream` | `internal/.../simplecpp.cpp` | 字元緩衝串流 |
| `FileStream` | `internal/.../simplecpp.cpp` | 檔案串流 |
| `Macro` | `internal/.../simplecpp.cpp/.h` | 巨集定義 |
| `NonExistingFilesCache` | `internal/.../simplecpp.cpp` | 不存在的檔案快取 |
| `Stream` | `internal/.../simplecpp.h` | 串流抽象基底 |
| `SIMPLECPP_LIB` | `internal/.../simplecpp.h` | simplecpp 函式庫標記 |
| `CodebaseMemoryMcp` | `pkg/homebrew/...rb` | Homebrew formula |
| `StubMethod` | `scripts/gen-py-stdlib.py` | Python stub 方法 |
| `StubClass` | `scripts/gen-py-stdlib.py` | Python stub 類別 |
| `StubFunction` | `scripts/gen-py-stdlib.py` | Python stub 函數 |
| `ModuleStubs` | `scripts/gen-py-stdlib.py` | 模組 stub 集合 |
| `StarReExport` | `scripts/gen-py-stdlib.py` | Star 重新匯出 |
| `Config` | `scripts/smoke-test.sh` | 測試設定 |
| `Model` | `scripts/soak-test.sh` | 壓力測試模型 |
| `classcallcheck` | `vendored/nomic/code_tokens.txt` | Nomic token 資料 |

---

## 六、函數分布總覽

### 依目錄

```
src/main.c           ████████████████████████████  14
src/pipeline/        ██████████████████████████████  29 (估計)
src/store/           ██████████████████████████████  60+ (估計)
src/graph_buffer/    ██████████████████████  30 (估計)
src/watcher/         ██████████  10
src/discover/        ████████████████  16
src/semantic/        █████████████████████  21
graph-ui/src/        ████████████████████  20
pkg/go/              ████████████████  15
pkg/pypi/            ████████████  11
scripts/             █████████████████████████  27
vendored/            ████  4
```

### 依程式語言

```
C/C++        ████████████████████████████████████████  190+
TypeScript   ██████████████  20
Go           ████████████  15
Python       ██████████████████  27
Shell        ██████  3
Ruby         █  1
PowerShell   ██████  3
```
