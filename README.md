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
