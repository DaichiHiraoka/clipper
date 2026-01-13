あなたはこのリポジトリの実装担当です。@docs/add_feature1.md の仕様どおりに機能を追加してください。

要件:
- まず「失敗するテスト」を先に追加し、その後に実装してテストを通してください（TDD）。
- テストは Rust ではなく Python（pytest）で実装してください（コンパイル負担軽減のため）。
- テストは CLI のブラックボックステストとして書き、stdout/stderr/exit code と生成ファイル内容で検証してください。
- テスト実行時にユーザー環境を汚さないこと（HOME / XDG_CONFIG_HOME / APPDATA などを一時ディレクトリに向ける）。
- テストは OS 非依存にすること。OS依存の期待値（shimの拡張子等）は条件分岐で吸収してください。

スコープ（@docs/add_feature1.md から最低限これを実装）:
1) `clipper tool add <name> --install "<cmd>" [--exec "<cmd>"]`
2) `clipper bootstrap [--force] [--print-path]` : shim 生成（POSIXは実行権限付与、Windowsは .cmd）
3) `clipper tool install <name>` および `clipper tool install --all`
4) 既存の export/import に tools と shim.dir を含める（schemaVersion を更新し、v1は後方互換で読める）
5) `clipper __shim__ <name> -- <args...>`（内部コマンド。shimから呼ばれる）
   - exec があればそれを起動して引数を渡す
   - exec が無ければ `<name>` を直接起動しようとするが、shim再帰を検知して分かりやすく失敗させる

テスト方針:
- `target/debug/clipper` を使って実行してください。テスト開始時に `cargo build` は1回だけ行う想定でよいです（pytest内で必要なら呼ぶ）。
- すべて `tmp_path` を使い、環境変数で設定ファイル保存先/ホームを tmp に向けて隔離してください。
- install コマンドはネットワーク不要でテストできるように「疑似インストール」にしてください:
  - 例: install cmd として「tmp内にダミー実行ファイルを生成する」コマンドを使う
  - POSIX: `sh -lc 'printf ... > <bin>/dummy && chmod +x <bin>/dummy'`
  - Windows: `cmd /C "echo ... > <bin>\\dummy.cmd"` など
- shim 生成先ディレクトリは tmp 内に固定できるように実装側で設定可能にしてください（例: configのshim.dir、または環境変数 CLIPPER_SHIM_DIR を優先）。テストは必ず tmp に出力させる。

最低限追加すべきpytestケース:
A. tool add → config に tools が保存される
B. bootstrap → shim が生成される（OSに応じた形式）＋ `--print-path` がパスを出力する
C. tool install → install cmd が実行され、ダミー実行ファイルが作られる
D. shim実行 → `shim -> clipper __shim__ -> exec` の流れでダミーが呼ばれる（引数が渡ること）
E. export/import → tools と shim.dir が保持される（import後に bootstrap 可能）

実装ガイド:
- Rust側は既存の設定ファイル構造と schemaVersion を尊重し、v1->v2 のマイグレーションを追加してください。
- CLIの追加は既存のコマンド体系に合わせてください（help表示やエラー文も簡潔に）。
- 失敗時は「次に何をすれば良いか」が分かるエラーメッセージにしてください（例: exec未設定、PATH未設定、shim衝突等）。

成果物:
- pytest テストファイル（例: `tests_py/test_tool_bootstrap.py`）と必要な補助コード
- Rust実装の変更一式
- `pytest -q` が通り、既存機能も壊れていないこと
