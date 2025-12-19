# 登録済みコマンドのエクスポート設計

## 目的

登録済みコマンドを外部ファイルに書き出し、移行・バックアップ・共有を可能にする。

## 背景 / 現状把握

- 現在の保存形式は `commands.json` の配列。
- 保存場所は `config_dir()/clipper/commands.json`。
- 現在の要素は `name` と `cmd` のみ。

例:

```json
[
  { "name": "build", "cmd": "cargo build" },
  { "name": "serve", "cmd": "python -m http.server" }
]
```

## スコープ

- 既存の登録済みコマンドをエクスポートできること。
- 形式は JSON を優先。
- 将来のインポートに備えたスキーマバージョンを付与する。

## 仕様

### CLI

新規サブコマンド `export` を追加する。

- `clipper export`
  - カレントディレクトリに `commands-export-YYYYMMDD-HHMMSS.json` を出力
- `clipper export --output <path>`
  - 指定パスに出力

将来拡張の候補:

- `--stdout` (標準出力へ書き出し)
- `--format json|yaml`
- `--include-secrets`

### 出力フォーマット

- 形式: JSON
- 文字コード: UTF-8
- ルートに `schemaVersion` と `exportedAt` を付与

スキーマ案 (v1):

```json
{
  "schemaVersion": 1,
  "exportedAt": "2025-01-01T12:00:00Z",
  "commands": [
    {
      "name": "build",
      "cmd": "cargo build"
    }
  ]
}
```

### エクスポート対象

現状のデータは `name` と `cmd` のみのため、それ以外は出力しない。
将来フィールドが追加された場合は下記の方針で拡張する。

- 追加フィールドは `commands` 内に追加する
- 破壊的変更が必要な場合は `schemaVersion` を更新する

### エラーハンドリング

- 設定ファイル読み込み失敗: パスを含めたエラー表示
- JSON 解析失敗: 不正データである旨を通知
- 書き込み失敗: 保存先パスと原因を表示

### セキュリティ / プライバシー

- 現状シークレット相当の項目は存在しない
- 将来シークレットが追加された場合は以下を検討:
  - 既定で除外
  - `--include-secrets` による明示的な含有
  - エクスポート前の警告表示

## 受け入れ条件

- 登録済みコマンドをファイルにエクスポートできる
- UTF-8 / `schemaVersion` 付き JSON が出力される
- 書き込み失敗時に原因を把握できるエラーが出る
- ドキュメントに使い方と出力例が追記される

## 実装タスク

- `ExportFile` (schemaVersion, exportedAt, commands) のDTO追加
- `commands.json` からエクスポートDTOへの変換
- `export` サブコマンドの追加
- 出力先決定ロジック (デフォルト名 + --output)
- README 更新 (使い方・出力例・注意事項)
- テスト追加 (空配列 / 多件 / 特殊文字 / 改行)

## オープンクエスチョン

- 将来 `--format yaml` を初回から入れるか
- `--stdout` を必要とするユースケースの有無
