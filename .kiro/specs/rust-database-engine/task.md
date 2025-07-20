# SerinDB 開発タスク分解書

このドキュメントは `requirements.md` と `design.md` をもとに、SerinDB を世界最高レベルで実現するための開発タスクを段階的 (Phase) に整理したものです。各 Phase は完了基準 (Definition of Done; DoD) を含み、依存関係がない限り並行実施を許容します。

---

## 目次
1. Phase 0: プロジェクト基盤構築
2. Phase 1: コア言語処理系 (Parser & CLI)
3. Phase 2: ストレージエンジン基盤
4. Phase 3: トランザクションマネージャ
5. Phase 4: クエリ最適化 & 実行エンジン (MVP)
6. Phase 5: インデックス実装
7. Phase 6: ネットワーク & ドライバ互換層
8. Phase 7: レプリケーション & クラスタリング
9. Phase 8: 観測性 & 運用ツール
10. Phase 9: 拡張データモデル (ドキュメント/グラフ/時系列)
11. Phase 10: セキュリティ & 暗号化
12. Phase 11: 国際化 (i18n) / ローカライゼーション (l10n)
13. Phase 12: パフォーマンス最適化 & ベンチマーク
14. Phase 13: リリース & 移行ツール

---

## Phase 0: プロジェクト基盤構築
| # | タスク | 詳細 | DoD |
|---|--------|------|------|
| 0.1 | リポジトリ初期化 | `cargo new --lib serindb`、ワークスペース構成 | GitHub 上で Rust ワークスペースが作成され CI が走る |
| 0.2 | CI/CD パイプライン | GitHub Actions で multi-platform build, test, clippy, fmt | main へ push 時に全ジョブ成功 |
| 0.3 | コード規約 & Lint | `rustfmt.toml`, `clippy.toml`, `cargo-deny` | PR で自動チェック & ブロック |
| 0.4 | ドキュメントサイト基盤 | `mdBook` + GitHub Pages | 自動デプロイで design/docs 表示 |
| 0.5 | コンテナビルド | `Dockerfile` (alpine, debian) & `docker-compose` | `docker run serindb --help` が動作 |

---

## Phase 1: コア言語処理系 (Parser & CLI)
| # | タスク | 詳細 | DoD |
|---|--------|------|------|
| 1.1 | Lex/Tokenize | `logos` ベース UTF-8 トークナイザ | すべての SQL キーワードを正しく認識 |
| 1.2 | AST 定義 | `enum Expr`, `enum Statement` など | 単体テストで SELECT/INSERT/UPDATE/DELETE パース成功 |
| 1.3 | SQL Parser | Antlr4 → Rust コード生成、エラーハンドリング | 100% カバレッジで負ケース含む |
| 1.4 | 拡張構文 (Cypher, JSON) | Cypher ライク & SQL/JSON 関数 | 複合クエリが AST 化される |
| 1.5 | CLI (`serinctl`) MVP | `readline` 補完、スクリプト実行、カラープロンプト | `serinctl -e "SELECT 1"` が 1 を返す |

---

## Phase 2: ストレージエンジン基盤
| # | タスク | 詳細 | DoD |
|---|--------|------|------|
| 2.1 | ページフォーマット実装 | `PageHeader`, `TupleSlot`, CRC | Unit Test でシリアライズ/逆シリアライズ一致 |
| 2.2 | Buffer Manager 2Q | `BufferFrame`, `BufferPool`, A1in/A1out/Am | p95 ページ取得 10µs 未満 |
| 2.3 | WAL Writer | 事前書き込み + グループコミット | クラッシュ後に WAL リプレイで整合性復旧 |
| 2.4 | Disk I/O (io_uring) | 非同期 pread/pwrite, DMA | ベンチで 500k IOPS 達成 |
| 2.5 | ストレージサブレイヤ API | `store::read_page`, `store::write_page` | 上位層から抽象 API 利用可能 |

---

## Phase 3: トランザクションマネージャ
| # | タスク | 詳細 | DoD |
|---|--------|------|------|
| 3.1 | MVCC スナップショット | `min_ts`, `max_ts` フィールド導入 | 同一テーブル並行 SELECT/UPDATE で正値確認 |
| 3.2 | ロックマネージャ | 意図ロック (IS, IX, S, X) + Wait-For Graph | デッドロックシミュレーションテスト合格 |
| 3.3 | GTM 実装 (シングルノード) | 単調増加タイムスタンプ | 高負荷でも一意タイムスタンプ保証 |
| 3.4 | シングルノード 2PC | Prepare/Commit ログ | パワーフェイルテストで Atomicity 保持 |

---

## Phase 4: クエリ最適化 & 実行エンジン (MVP)
| # | タスク | 詳細 | DoD |
|---|--------|------|------|
| 4.1 | 論理プラン生成 | `LogicalScan`, `LogicalFilter` など | Explain で表示確認 |
| 4.2 | 物理プラン決定 | `SeqScan`, `HashJoin`, `Sort` | コスト関数ユニットテスト |
| 4.3 | Vectorized Execution | `ColumnBatch` パイプライン | `SELECT * FROM t` で 500 MB/s
| 4.4 | JIT Expression | Cranelift で WHERE 条件 | ベンチで 2× 非JIT パフォーマンス |

---

## Phase 5: インデックス実装
| # | タスク | 詳細 | DoD |
|---|--------|------|------|
| 5.1 | B+Tree | 分割/マージ/フェンスキー | 100 万挿入で O(log N) 速度 |
| 5.2 | LSM Tree レベル 0-1 | MemTable -> SSTable | 書込スループット 1M rec/s |
| 5.3 | GiST / R-Tree | 空間データサポート | 四角形クエリで 90% フィルタ率 |
| 5.4 | ブルームフィルタ | 不存在高速判定 | 誤陽性率 <1% |

---

## Phase 6: ネットワーク & ドライバ互換層
| # | タスク | 詳細 | DoD |
|---|--------|------|------|
| 6.1 | PostgreSQL Wire v3 | Startup, Query, Sync | `psql` から接続/SELECT 成功 |
| 6.2 | 認証 (password, md5) | StartupMessage flow | 認証失敗時に正エラー |
| 6.3 | ドライバ Rust/Go | `tokio-postgres` 互換サンプル | ORM から CRUD 成功 |

---

## Phase 7: レプリケーション & クラスタリング
| # | タスク | 詳細 | DoD |
|---|--------|------|------|
| 7.1 | Raft 実装 (ログ複製) | `raft-rs` ベース | 3 ノードでリーダフェイルオーバー 3s 内 |
| 7.2 | シャーディングメタデータ | `serin_partitions` カタログ | 分散テーブル作成 CLI |
| 7.3 | 自動リバランス | 2D ビンパッキング | 不均衡 10% 以下維持 |

---

## Phase 8: 観測性 & 運用ツール
| # | タスク | 詳細 | DoD |
|---|--------|------|------|
| 8.1 | OpenTelemetry Trace | gRPC OTLP Exporter | Jaeger UI にトレース表示 |
| 8.2 | Prometheus Metrics | `/metrics` エンドポイント | Grafana ダッシュボード表示 |
| 8.3 | serinctl ダンプ | バックアップ/リストア/設定変更 | `serinctl backup` が S3 へ出力 |

---

## Phase 9: 拡張データモデル
| # | タスク | 詳細 | DoD |
|---|--------|------|------|
| 9.1 | JSON ドキュメント型 | JSONPath インデックス | `SELECT doc->>'name'` OK |
| 9.2 | Property Graph | ノード/エッジテーブル & Cypher パーサ | `MATCH (n)-[]->() RETURN n` 成功 |
| 9.3 | 時系列ストレージ | 列指向チャンク + 圧縮 | 2M rec/s 書込み達成 |

---

## Phase 10: セキュリティ & 暗号化
| # | タスク | 詳細 | DoD |
|---|--------|------|------|
| 10.1 | mTLS ハンドシェイク | Rustls + client cert | 不正証明書拒否 |
| 10.2 | TDE (AES-GCM) | 透過カラム暗号化 | 復号とパフォーマンス測定 |
| 10.3 | RBAC 実装 | `role`, `privilege`, `object` | 権限昇格攻撃テスト失敗 |

---

## Phase 11: 国際化 / ローカライゼーション
| # | タスク | 詳細 | DoD |
|---|--------|------|------|
| 11.1 | ICU4X 統合 | ロケール検出 & 数値/日付フォーマット | 日本語/英語/ドイツ語 表示一致 |
| 11.2 | Fluent メッセージカタログ | `.ftl` ローダ & fallback | 未翻訳キーは英語 |
| 11.3 | 多言語ドキュメント | GitHub Action, Crowdin sync | 3 言語以上公開 |

---

## Phase 12: パフォーマンス最適化 & ベンチマーク
| # | タスク | 詳細 | DoD |
|---|--------|------|------|
| 12.1 | TPC-C 10k Warehouse | tpmC 計測 | 30M 超え |
| 12.2 | TPC-DS 10TB | クエリ実行時間計測 | 5M QphH 達成 |
| 12.3 | TSBS 時系列 | 2M rec/s 書込維持 | Latency p95 < 5ms |
| 12.4 | Macro/Micro 回帰テスト | 前回比較 CI | 2% 劣化でアラート |

---

## Phase 13: リリース & 移行ツール
| # | タスク | 詳細 | DoD |
|---|--------|------|------|
| 13.1 | Helm Chart GA | k8s 1-click デプロイ | `helm install serindb` 成功 |
| 13.2 | pg_dump/restore 互換 | シームレス移行 | データ差分ゼロ保証 |
| 13.3 | バージョニング & SBOM | SemVer + CycloneDX | リリース artefact 署名 & 公開 |

---

### ガバナンス
* **Definition of Ready (DoR):** 仕様 & 受け入れ基準が明確、依存タスク完了。
* **Definition of Done (DoD):** コードレビュー、テスト 100%、CI pass、ドキュメント更新。

---

これらのタスクは `design.md` のアーキテクチャと `requirements.md` の機能・非機能要件を完全にカバーし、段階的に SerinDB を世界最高のデータベースエンジンへと発展させるロードマップを提供します。 

## チェックリスト (進捗管理用)
以下のチェックリストは各フェーズのタスクを細分化し、Markdown の `- [ ]` 形式で進捗をトラッキングできるようにしたものです。

### Phase 0: プロジェクト基盤構築
- [x] **0.1 リポジトリ初期化**
  - [x] Cargo ワークスペース作成 (`cargo new --lib serindb`)
  - [x] `.cargo/config.toml` でターゲット設定統一
  - [x] GitHub リポジトリ作成 & 初期コミット
- [x] **0.2 CI/CD パイプライン**
  - [x] GitHub Actions ワークフロー `rust.yml` 作成
  - [x] Build Matrix (linux, windows, macos, arm64)
  - [x] `cargo test`, `clippy`, `fmt` を並列実行
- [x] **0.3 コード規約 & Lint**
  - [x] `rustfmt.toml` に max_width / edition 設定
  - [x] `clippy.toml` で許容ルール定義
  - [x] `cargo-deny` でライセンス & セキュリティチェック
- [ ] **0.4 ドキュメントサイト基盤**
  - [ ] `mdBook` テンプレート生成
  - [ ] GitHub Pages デプロイ Workflow 作成
  - [ ] `design.md` & `requirements.md` を自動取り込み
- [ ] **0.5 コンテナビルド**
  - [ ] Multi-stage `Dockerfile` (Alpine)
  - [ ] `docker-compose.yml` に dev サービス追加
  - [ ] `docker run serindb --help` テスト

### Phase 1: コア言語処理系 (Parser & CLI)
- [ ] **1.1 Lex/Tokenize**
  - [ ] `logos` ベース lexer 実装
  - [ ] SQL92 全キーワードユニットテスト
- [ ] **1.2 AST 定義**
  - [ ] `Expr`, `Stmt`, `DataType` Enum 設計
  - [ ] Serde でデバッグ表示実装
- [ ] **1.3 SQL Parser**
  - [ ] Antlr4 grammar 生成スクリプト
  - [ ] エラーリカバリ (panic / resume) 実装
  - [ ] パーサー fuzz テスト
- [ ] **1.4 拡張構文 (Cypher, JSON)**
  - [ ] Cypher grammar 追加
  - [ ] JSONPath サポート
- [ ] **1.5 CLI (`serinctl`) MVP**
  - [ ] `rustyline` インタラクティブ Shell
  - [ ] `.serinrc` 設定ファイル読み込み
  - [ ] バッチスクリプト実行オプション `-f`

### Phase 2: ストレージエンジン基盤
- [ ] **2.1 ページフォーマット実装**
  - [ ] `PageHeader`, `TupleSlot` 構造体
  - [ ] CRC32C 検証関数
  - [ ] 可変長データスロット実装
- [ ] **2.2 Buffer Manager 2Q**
  - [ ] A1in/A1out/Am リスト構造
  - [ ] CLOCK ハンド victim 選定ロジック
  - [ ] NUMA-aware アロケータ検証
- [ ] **2.3 WAL Writer**
  - [ ] Write-Ahead Logging バイナリフォーマット
  - [ ] グループコミット & fsync バッチ
  - [ ] クラッシュリカバリユニットテスト
- [ ] **2.4 Disk I/O (io_uring)**
  - [ ] `tokio-uring` ベース async pread/pwrite
  - [ ] IOPS ベンチツール作成
- [ ] **2.5 ストレージサブレイヤ API**
  - [ ] `StorageEngine` trait 定義
  - [ ] Mock Storage で上位テスト

### Phase 3: トランザクションマネージャ
- [ ] **3.1 MVCC スナップショット**
  - [ ] `VersionedTuple` 構造体に ts フィールド追加
  - [ ] Read & Write set 検証テスト
- [ ] **3.2 ロックマネージャ**
  - [ ] Intention lock table 実装
  - [ ] Deadlock detector (Wait-For Graph BFS)
- [ ] **3.3 GTM (シングルノード)**
  - [ ] AtomicU64 ベース timestamp allocator
  - [ ] Benchmark で 1M tx/s 確認
- [ ] **3.4 2PC (Single Node)**
  - [ ] Prepare log record 永続化
  - [ ] Crash simulation テスト

### Phase 4: クエリ最適化 & 実行エンジン (MVP)
- [ ] **4.1 論理プラン生成**
  - [ ] SELECT/WHERE push-down optimizer pass
- [ ] **4.2 物理プラン決定**
  - [ ] Cost model 関数実装 (CPU/IO)
- [ ] **4.3 Vectorized Execution**
  - [ ] ColumnBatch 4096 rows 実装
  - [ ] SIMD filter オペレータ (AVX2)
- [ ] **4.4 JIT Expression**
  - [ ] Cranelift backend 統合
  - [ ] Hotspot detection	

### Phase 5: インデックス実装
- [ ] **5.1 B+Tree**
  - [ ] Split/Merge 操作
  - [ ] BulkLoad ユーティリティ
- [ ] **5.2 LSM Tree レベル 0-1**
  - [ ] MemTable (skiplist) 実装
  - [ ] SSTable writer/reader
- [ ] **5.3 GiST / R-Tree**
  - [ ] STR 分割アルゴリズム
- [ ] **5.4 ブルームフィルタ**
  - [ ] MurmurHash3 実装

### Phase 6 以降
- [ ] **6.1 PostgreSQL Wire v3**
  - [ ] ハンドシェイク & StartupMessage 実装
  - [ ] Simple Query プロトコル (Q, T, D メッセージ)
  - [ ] Extended Query (Parse/Bind/Execute/Sync)
  - [ ] ErrorResponse & NoticeResponse コード体系
  - [ ] COPY IN/OUT プロトコルサポート
  - [ ] TLS ネゴシエーション (SSLRequest)
- [ ] **6.2 認証方式**
  - [ ] パスワード(MD5) 認証実装
  - [ ] SCRAM-SHA-256 認証
  - [ ] 認証設定 YAML 読み込み
  - [ ] 不正アクセス試験ケース
- [ ] **6.3 ドライバ SDK**
  - [ ] Rust (`serin-rs`) tokio-postgres ラッパー
  - [ ] Go (`serin-go`) database/sql Driver
  - [ ] Java (`serin-jdbc`) JDBC 4.3 Driver
  - [ ] Python (`serin-py`) asyncpg ラッパー
  - [ ] Node.js (`serin-js`) pg ラッパー
  - [ ] 各言語で CRUD サンプルアプリ
- [ ] **6.4 接続プール & プロキシ**
  - [ ] サーバー側プール（設定: max_idle, max_active）
  - [ ] ステートメントキャッシュ
  - [ ] ラウンドロビン/最小接続ロードバランシング
  - [ ] ヘルスチェックエンドポイント `/readyz`

### Phase 7: レプリケーション & クラスタリング
- [ ] **7.1 Raft 実装**
  - [ ] ログ複製モジュール
  - [ ] リーダ選出 + 心拍
  - [ ] スナップショット & ログ圧縮
  - [ ] メンバーシップ変更プロトコル
- [ ] **7.2 クラスタメタデータサービス**
  - [ ] システムカタログの複製
  - [ ] シャードマップ API
  - [ ] ノード検出 (gRPC)
- [ ] **7.3 シャーディングメカニズム**
  - [ ] ハッシュシャーディング実装
  - [ ] レンジシャーディング実装
  - [ ] ReShard オペレーション CLI
- [ ] **7.4 自動リバランサ**
  - [ ] QPS/サイズメトリクス収集
  - [ ] 2D ビンパッキングアルゴリズム
  - [ ] データムーブレート制御
- [ ] **7.5 マルチDC レプリケーション**
  - [ ] 非同期レプリケーションチャネル
  - [ ] 衝突検出 & 解決ポリシー
  - [ ] レイテンシ KPI テスト
- [ ] **7.6 分散一貫性テスト**
  - [ ] Jepsen テストシナリオ
  - [ ] ネットワーク分断/障害注入

### Phase 8: 観測性 & 運用ツール
- [ ] **8.1 OpenTelemetry トレース**
  - [ ] スパン計装 (パーサ→ストレージ)
  - [ ] gRPC OTLP Exporter
  - [ ] サンプリング設定 (率/条件)
- [ ] **8.2 Prometheus メトリクス**
  - [ ] Core メトリクス (QPS, latency, hit ratio)
  - [ ] Histogram バケット最適化
  - [ ] `/metrics` 認証オプション
- [ ] **8.3 構造化ログ**
  - [ ] JSON Lines + trace_id 埋め込み
  - [ ] ローテーション & 圧縮
  - [ ] 動的ログレベル変更 API
- [ ] **8.4 ダッシュボード & アラート**
  - [ ] Grafana ダッシュボードテンプレート
  - [ ] Alertmanager ルール
  - [ ] SLO 定義 (p99 <10ms 等)
- [ ] **8.5 serinctl 運用コマンド**
  - [ ] `backup`, `restore`, `analyze`, `health`
  - [ ] `config set` ホットリロード
  - [ ] `top` でライブメトリクス

### Phase 9: 拡張データモデル
- [ ] **9.1 JSON ドキュメント型**
  - [ ] JSONB ストレージエンコード
  - [ ] JSONPath 評価器
  - [ ] GIN インデックス
  - [ ] JSON スキーマ検証
- [ ] **9.2 Property Graph**
  - [ ] ノード/エッジテーブルスキーマ
  - [ ] Cypher パーサ & プランナー
  - [ ] グラフトラバーサル演算子
  - [ ] グラフインデックス (AdjacencyList)
- [ ] **9.3 時系列ストレージ**
  - [ ] 列指向チャンクライター
  - [ ] Delta + Gorilla 圧縮
  - [ ] Time-bucketing インデックス
  - [ ] Continuous Aggregate ビュー
- [ ] **9.4 ベクトル検索**
  - [ ] HNSW インデックス builder
  - [ ] KNN オペレータ & プランノード
  - [ ] ef/efConstruction チューニング

### Phase 10: セキュリティ & 暗号化
- [ ] **10.1 mTLS**
  - [ ] TLSConfig + Cert Reload
  - [ ] 証明書ローテーションテスト
- [ ] **10.2 TDE (AES-GCM)**
  - [ ] Key Envelope (KEK/DEK) 実装
  - [ ] バックグラウンド Re-Key 処理
- [ ] **10.3 RBAC**
  - [ ] `CREATE ROLE`, `GRANT` 文実装
  - [ ] 階層型権限マッピング
- [ ] **10.4 監査ログ**
  - [ ] Query Audit, Security Event
  - [ ] WORM ストレージ出力
- [ ] **10.5 鍵管理 (KMS)**
  - [ ] Hashicorp Vault プラグイン
  - [ ] AWS KMS / GCP KMS プラグイン

### Phase 11: 国際化 / ローカライゼーション
- [ ] **11.1 ICU4X 統合**
  - [ ] 数値/日付/通貨フォーマッタ
  - [ ] ロケール判定 API
- [ ] **11.2 Fluent メッセージカタログ**
  - [ ] `.ftl` ローダ
  - [ ] フォールバックチェイン
- [ ] **11.3 ドキュメント多言語化**
  - [ ] mdBook i18n モジュール
  - [ ] Crowdin CI 同期
- [ ] **11.4 エラーメッセージ翻訳**
  - [ ] 100% キー網羅テスト

### Phase 12: パフォーマンス最適化 & ベンチマーク
- [ ] **12.1 TPC-C ベンチ**
  - [ ] Workload Generator 10k WH
  - [ ] tpmC 計測 & グラフ
- [ ] **12.2 TPC-DS ベンチ**
  - [ ] 10TB データロード
  - [ ] 99 クエリ実行 & 計測
- [ ] **12.3 TSBS 時系列**
  - [ ] Write Path 2M rec/s
  - [ ] Query Latency p95 <5ms
- [ ] **12.4 プロファイリング & 回帰**
  - [ ] perf/flamegraph 脚本
  - [ ] CI で 2% 劣化検出
- [ ] **12.5 最適化タスク**
  - [ ] SIMD 演算改善
  - [ ] アロケータチューニング

### Phase 13: リリース & 移行ツール (追加タスク)
- [ ] **13.1 Helm Chart GA**
  - [ ] values.yml パラメータ整理
  - [ ] Rolling Upgrade 試験
- [ ] **13.2 移行ツール**
  - [ ] `pg_dump`/`pg_restore` ラッパー
  - [ ] MySQL → Serin delta sync
  - [ ] MongoDB BSON インポータ
- [ ] **13.3 SBOM & 署名**
  - [ ] CycloneDX SBOM 自動生成
  - [ ] Sigstore (cosign) 署名
- [ ] **13.4 リリースパッケージング**
  - [ ] クロスコンパイル (musl/gnu)
  - [ ] Homebrew / Winget formula
  - [ ] Release Note 自動生成
- [ ] **13.5 コミュニティリソース**
  - [ ] ガイド付きチュートリアル
  - [ ] Issue/PR テンプレート
  - [ ] 貢献ドキュメント更新 