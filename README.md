# clipper

よく使うコマンドを登録・実行するための軽量なコマンドランナーツール

## 概要

`clipper`は、頻繁に使用するコマンドラインコマンドをエイリアスとして登録し、簡単に実行できるCLIツールです。ファジー検索機能により、部分一致で素早くコマンドを見つけて実行できます。

## 特徴

- シンプルなJSON形式でコマンドを管理
- ファジー検索による高速なコマンド選択
- 対話的なUIでコマンドの追加・実行が可能
- Windows/Linux両対応
- サイズ最適化されたバイナリ

## インストール

### ビルド

```bash
cargo build --release
```

ビルドされたバイナリは `target/release/clipper` に生成されます。

### パスの設定

ビルドしたバイナリをパスの通った場所に配置するか、パスに追加してください。

```bash
# 例: ~/.local/bin にコピー
cp target/release/clipper ~/.local/bin/
```

## 使い方

### コマンドの追加

#### 対話モードで追加

```bash
clipper add
```

エイリアス名と実行コマンドを入力します。

#### コマンドラインで直接追加

```bash
clipper add serve "python -m http.server"
clipper add build "cargo build --release"
clipper add test-all "cargo test --all"
```

### コマンドの実行

#### すべてのコマンドから選択

```bash
clipper run
```

登録されているすべてのコマンドがファジー検索可能な形で表示されます。

#### 部分一致で絞り込んで実行

```bash
clipper run bu
```

"bu"を含むコマンド（例: "build"）に絞り込まれます。
- 候補が1件のみ：即座に実行
- 候補が複数：ファジー検索で選択

### コマンドのエクスポート

登録済みコマンドをJSONファイルとして書き出します。

```bash
clipper export
```

出力先を指定する場合は `--output` を使います。

```bash
clipper export --output ./commands-export.json
```

出力例:

```json
{
  "schemaVersion": 1,
  "exportedAt": "2025-01-01T12:00:00Z",
  "commands": [
    { "name": "build", "cmd": "cargo build" }
  ]
}
```

### コマンドのインポート

エクスポートしたJSONファイルからコマンドをインポートします。他の環境への移行やバックアップからの復元に便利です。

#### 基本的なインポート

```bash
clipper import commands-export-20250105-120000.json
```

デフォルトでは、既存のコマンドと名前が重複する場合はスキップされます。

#### オプション

##### --overwrite: 既存のコマンドを上書き

```bash
clipper import commands.json --overwrite
```

重複する名前のコマンドがある場合、インポートするコマンドで上書きします。

##### --merge: 対話的にマージ

```bash
clipper import commands.json --merge
```

重複するコマンドごとに、以下の選択肢から対処方法を選べます：
- 上書き: インポートするコマンドで置き換える
- スキップ: 既存のコマンドを保持
- リネーム: 新しい名前（`{元の名前}_imported`）でインポート

##### --append-only: 新規コマンドのみ追加

```bash
clipper import commands.json --append-only
```

デフォルト動作と同じですが、明示的に指定できます。

#### インポート結果

インポート後には、以下のようなサマリーが表示されます：

```
インポートしました: 3件追加、2件スキップ（重複）
```

#### エラーハンドリング

- ファイル読み込みエラー（例: ファイルが存在しない場合など）: `Error: failed to read <path>: <reason>`（`<reason>` は「No such file or directory」「Permission denied」など原因により異なります）
- 不正なJSON: `Error: invalid JSON in import file: <path>`
- サポート外のスキーマバージョン: `Error: Unsupported schema version: 2. This version of clipper supports v1 only.`

## 設定ファイル

コマンドは以下の場所にJSON形式で保存されます：

- **Linux**: `~/.config/clipper/commands.json`
- **Windows**: `%APPDATA%\clipper\commands.json`

### commands.jsonの形式

```json
[
  { "name": "build", "cmd": "cargo build" },
  { "name": "serve", "cmd": "python -m http.server" },
  { "name": "test-all", "cmd": "cargo test --all" }
]
```

## 使用例

```bash
# Webサーバーを起動するコマンドを追加
clipper add serve "python -m http.server 8000"

# ビルドコマンドを追加
clipper add build "cargo build --release"

# 自作のexeファイルを登録（フルパス）
clipper add mytool "C:\tools\mytool.exe --config config.json"

# PATHに追加済みのexeファイルを登録
clipper add deploy "deploy.exe --production"

# "bu"で始まるコマンドを検索して実行
clipper run bu

# すべてのコマンドから選択
clipper run
```

## 依存関係

- `anyhow`: エラーハンドリング
- `serde`/`serde_json`: JSON処理
- `dirs`: クロスプラットフォームの設定ディレクトリパス取得
- `dialoguer`: 対話的なCLI UI

## ライセンス

このプロジェクトのライセンスは未指定です。
