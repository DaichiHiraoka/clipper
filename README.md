# clipper

よく使うコマンドを登録・実行するための軽量なコマンドランナーツール + ツール環境のブートストラップ

## 概要

`clipper`は、頻繁に使用するコマンドラインコマンドをエイリアスとして登録し、簡単に実行できるCLIツールです。ファジー検索機能により、部分一致で素早くコマンドを見つけて実行できます。

さらに、**Tool Bootstrap機能**により、複数マシン間でCLIツールの環境を簡単に引き継ぐことができます。ツールのインストール方法を記録し、shimを生成することで、どのマシンでも同じコマンド名でツールを使用できます。

## 特徴

### コマンドランナー機能
- シンプルなJSON形式でコマンドを管理
- ファジー検索による高速なコマンド選択
- 対話的なUIでコマンドの追加・実行が可能

### Tool Bootstrap機能（新機能）
- ツールのインストールレシピを記録・実行
- shimによる統一されたコマンド名の保証
- export/importで環境をまるごと移行
- クロスプラットフォーム対応（Windows/Linux/macOS）

### その他
- サイズ最適化されたバイナリ
- Windows/Linux/macOS 対応

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

### コマンドの削除

```bash
clipper delete
```

登録済みのコマンドをファジー検索で選択して削除します。

---

## Tool Bootstrap機能

Tool Bootstrap機能を使うと、CLIツールのインストール方法を記録し、新しいマシンで簡単に環境を再現できます。

### ツールの登録

ツールのインストールコマンドと実行コマンドを登録します。

```bash
# 基本的な登録
clipper tool add <tool-name> --install "<install-command>" --exec "<exec-command>"
```

#### 例: ripgrep (rg) の登録

```bash
# Debian/Ubuntu
clipper tool add rg --install "sudo apt-get update && sudo apt-get install -y ripgrep" --exec "rg"

# macOS (Homebrew)
clipper tool add rg --install "brew install ripgrep" --exec "rg"

# Cargo
clipper tool add rg --install "cargo install ripgrep" --exec "rg"
```

#### 例: Node.js グローバルツール

```bash
clipper tool add http-server --install "npm install -g http-server" --exec "http-server"
```

#### 例: curlでインストールするツール

```bash
clipper tool add uv --install "curl -LsSf https://astral.sh/uv/install.sh | sh" --exec "uv"
```

#### execなしで登録（shimのみ）

`--exec` を省略すると、shimは作成されますが、実行時にエラーメッセージが表示されます。後から設定できます。

```bash
clipper tool add mytool --install "curl -fsSL https://example.com/install.sh | sh"
```

### shimの生成（bootstrap）

登録したすべてのツールに対してshimを生成します。

```bash
clipper bootstrap
```

shimは以下のディレクトリに生成されます：
- **Linux/macOS**: `~/.local/bin/`
- **Windows**: `%LOCALAPPDATA%\clipper\bin\`

環境変数 `CLIPPER_SHIM_DIR` で出力先を変更できます。

#### PATHの設定を表示

```bash
clipper bootstrap --print-path
```

shimディレクトリをPATHに追加する方法が表示されます。

#### 既存のshimを上書き

```bash
clipper bootstrap --force
```

既に存在するshimファイルを上書きします。

### ツールのインストール

登録したインストールコマンドを実行します。

```bash
# 特定のツールをインストール
clipper tool install <tool-name>

# すべてのツールをインストール
clipper tool install --all
```

#### 例

```bash
# rgをインストール
clipper tool install rg

# すべてのツールを一括インストール
clipper tool install --all
```

### 新しいマシンでの環境構築フロー

1. **元のマシンでエクスポート**
   ```bash
   clipper export
   ```

2. **新しいマシンにファイルをコピー**
   エクスポートしたJSONファイルを新しいマシンに転送します。

3. **新しいマシンでインポート**
   ```bash
   clipper import commands-export-20260113-123456.json --overwrite
   ```

4. **shimを生成**
   ```bash
   clipper bootstrap --print-path
   ```
   表示された手順に従ってPATHを設定します。

5. **ツールをインストール**
   ```bash
   clipper tool install --all
   ```

6. **完了！**
   元のマシンと同じコマンド名でツールが使用できます。

---

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
  "schemaVersion": 2,
  "exportedAt": "2026-01-13T12:00:00Z",
  "commands": [
    { "name": "build", "cmd": "cargo build" }
  ],
  "tools": {
    "rg": {
      "name": "rg",
      "install": {
        "cmd": "cargo install ripgrep",
        "shell": "sh"
      },
      "exec": {
        "type": "string",
        "value": "rg"
      },
      "createdAt": "2026-01-13T12:00:00Z",
      "updatedAt": "2026-01-13T12:00:00Z"
    }
  },
  "shim": {
    "dir": "/home/user/.local/bin"
  }
}
```

### コマンドのインポート

エクスポートしたJSONファイルからコマンドとツールをインポートします。他の環境への移行やバックアップからの復元に便利です。

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

- ファイル読み込みエラー（例: ファイルが存在しない場合など）: `Error: failed to read import file: <path>`（原因は「No such file or directory」「Permission denied」などにより異なります）
- 不正なJSON: `Error: invalid JSON in import file: <path>`
- サポート外のスキーマバージョン: `Error: Unsupported schema version: 3. This version of clipper supports v1-v2 only.`

---

## 設定ファイル

コマンドとツールは以下の場所にJSON形式で保存されます：

- **Linux**: `~/.config/clipper/commands.json`
- **Windows**: `%APPDATA%\clipper\commands.json`

環境変数 `XDG_CONFIG_HOME` (Linux) または `APPDATA` (Windows) で変更できます。

### commands.jsonの形式（Schema Version 2）

```json
{
  "schemaVersion": 2,
  "commands": [
    { "name": "build", "cmd": "cargo build" },
    { "name": "serve", "cmd": "python -m http.server" },
    { "name": "test-all", "cmd": "cargo test --all" }
  ],
  "tools": {
    "rg": {
      "name": "rg",
      "install": {
        "cmd": "cargo install ripgrep",
        "shell": "sh"
      },
      "exec": {
        "type": "string",
        "value": "rg"
      },
      "createdAt": "2026-01-13T12:00:00Z",
      "updatedAt": "2026-01-13T12:00:00Z"
    }
  },
  "shim": {
    "dir": "/home/user/.local/bin"
  }
}
```

### 後方互換性

v1形式（配列形式）のファイルは自動的にv2形式に変換されます。

```json
// v1形式（旧形式）- 読み込み時に自動変換されます
[
  { "name": "build", "cmd": "cargo build" }
]
```

## 使用例

### コマンドランナー機能の例

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

# コマンドを削除
clipper delete
```

### Tool Bootstrap機能の例

```bash
# ツールを登録
clipper tool add fd --install "cargo install fd-find" --exec "fd"
clipper tool add bat --install "cargo install bat" --exec "bat"
clipper tool add ripgrep --install "cargo install ripgrep" --exec "rg"

# shimを生成してPATH設定を表示
clipper bootstrap --print-path

# ツールをインストール
clipper tool install --all

# エクスポート
clipper export

# 新しいマシンでインポート
clipper import commands-export-20260113-123456.json --overwrite

# 新しいマシンでbootstrap
clipper bootstrap

# 新しいマシンでツールをインストール
clipper tool install --all
```

## 依存関係

- `anyhow`: エラーハンドリング
- `serde`/`serde_json`: JSON処理
- `dirs`: クロスプラットフォームの設定ディレクトリパス取得
- `dialoguer`: 対話的なCLI UI
- `chrono`: タイムスタンプ生成

## ライセンス

このプロジェクトのライセンスは未指定です。
