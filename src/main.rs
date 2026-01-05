use anyhow::{bail, Context, Result};
use dialoguer::{theme::ColorfulTheme, Confirm, FuzzySelect, Input};
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use chrono::Utc;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CmdEntry {
    name: String,
    cmd: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExportFile {
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    #[serde(rename = "exportedAt")]
    exported_at: String,
    commands: Vec<CmdEntry>,
}

const EXPORT_SCHEMA_VERSION: u32 = 1;

fn config_root() -> Result<PathBuf> {
    // Windowsでは通常: C:\Users\<you>\AppData\Roaming
    let base = config_dir().context("cannot locate config dir")?;
    let dir = base.join("clipper");
    if !dir.exists() {
        fs::create_dir_all(&dir).context("failed to create config dir")?;
    }
    Ok(dir)
}

fn commands_path() -> Result<PathBuf> {
    Ok(config_root()?.join("commands.json"))
}

fn ensure_commands_file(path: &Path) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    let sample = r#"
[
  { "name": "build",    "cmd": "cargo build" },
  { "name": "serve",    "cmd": "python -m http.server" },
  { "name": "test-all", "cmd": "cargo test --all" }
]
"#;
    fs::write(path, sample.trim_start())
        .with_context(|| format!("failed to write sample commands to {}", path.display()))?;
    Ok(())
}

fn load_commands(path: &Path) -> Result<Vec<CmdEntry>> {
    let data =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let list: Vec<CmdEntry> =
        serde_json::from_str(&data).with_context(|| format!("invalid JSON: {}", path.display()))?;
    Ok(list)
}

fn save_commands(path: &Path, cmds: &[CmdEntry]) -> Result<()> {
    let data = serde_json::to_string_pretty(cmds)?;
    fs::write(path, data)?;
    Ok(())
}

fn default_export_path() -> Result<PathBuf> {
    let dir = std::env::current_dir().context("cannot locate current dir")?;
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    Ok(dir.join(format!("commands-export-{}.json", timestamp)))
}

fn run_shell(cmdline: &str) -> Result<()> {
    println!("▶ {}", cmdline);
    // Windowsネイティブ：cmd /C 経由で実行
    let status = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", cmdline]).status()?
    } else {
        Command::new("sh").arg("-c").arg(cmdline).status()?
    };
    if !status.success() {
        bail!("command failed (exit code: {:?})", status.code());
    }
    Ok(())
}

/* ------------ commands ------------ */

fn cmd_run(args: &[String]) -> Result<()> {
    let initial = if args.len() >= 2 { &args[1] } else { "" };

    let path = commands_path()?;
    ensure_commands_file(&path)?;
    let cmds = load_commands(&path)?;

    // 事前フィルタ（部分一致）
    let filtered: Vec<&CmdEntry> = if initial.is_empty() {
        cmds.iter().collect()
    } else {
        cmds.iter().filter(|c| c.name.contains(initial)).collect()
    };

    match filtered.len() {
        0 => {
            println!("no match: {}", initial);
            return Ok(());
        }
        1 => {
            // 候補1件は即実行
            return run_shell(&filtered[0].cmd);
        }
        _ => {
            // 複数候補 → FuzzySelect で絞り込み & 選択
            let theme = ColorfulTheme::default();
            let items: Vec<String> = filtered
                .iter()
                .map(|c| format!("{}  →  {}", c.name, c.cmd))
                .collect();

            let sel = FuzzySelect::with_theme(&theme)
                .with_prompt("実行するコマンドを選択（タイプで絞込）")
                .items(&items)
                .default(0)
                .interact()?;

            return run_shell(&filtered[sel].cmd);
        }
    }
}

fn cmd_add(args: &[String]) -> Result<()> {
    let path = commands_path()?;
    ensure_commands_file(&path)?;
    let mut cmds = load_commands(&path)?;

    // 引数で name/command が来ていればそれを使う
    let (name, cmd) = if args.len() >= 3 {
        (args[1].clone(), args[2..].join(" "))
    } else {
        // 対話で入力
        let name: String = Input::new()
            .with_prompt("エイリアス名")
            .interact_text()?;
        let cmd: String = Input::new()
            .with_prompt("実行コマンド")
            .interact_text()?;
        (name, cmd)
    };

    if let Some(i) = cmds.iter().position(|c| c.name == name) {
        if !Confirm::new()
            .with_prompt(format!("'{}' は既に存在します。上書きしますか？", name))
            .interact()?
        {
            println!("キャンセルしました");
            return Ok(());
        }
        cmds[i] = CmdEntry { name, cmd };
    } else {
        cmds.push(CmdEntry { name, cmd });
    }

    save_commands(&path, &cmds)?;
    println!("コマンドを追加しました");
    Ok(())
}

fn cmd_delete() -> Result<()> {
    let path = commands_path()?;
    ensure_commands_file(&path)?;
    let mut cmds = load_commands(&path)?;

    if cmds.is_empty() {
        println!("登録済みのコマンドがありません");
        return Ok(());
    }

    let theme = ColorfulTheme::default();
    let items: Vec<String> = cmds
        .iter()
        .map(|c| format!("{}  →  {}", c.name, c.cmd))
        .collect();

    let sel = FuzzySelect::with_theme(&theme)
        .with_prompt("削除するコマンドを選択（タイプで絞込）")
        .items(&items)
        .default(0)
        .interact()?;

    let target = cmds[sel].clone();
    let confirmed = Confirm::new()
        .with_prompt(format!("'{}' を削除しますか？ (Y/n)", target.name))
        .default(true)
        .interact()?;

    if !confirmed {
        println!("キャンセルしました");
        return Ok(());
    }

    cmds.remove(sel);
    save_commands(&path, &cmds)?;
    println!("削除しました");
    Ok(())
}

fn cmd_export(args: &[String]) -> Result<()> {
    let path = commands_path()?;
    ensure_commands_file(&path)?;
    let cmds = load_commands(&path)?;

    let output = match args.len() {
        1 => default_export_path()?,
        3 if args[1] == "--output" || args[1] == "-o" => PathBuf::from(&args[2]),
        _ => {
            bail!("invalid args for export. use: clipper export [--output <path>]");
        }
    };

    let export_file = ExportFile {
        schema_version: EXPORT_SCHEMA_VERSION,
        exported_at: Utc::now().to_rfc3339(),
        commands: cmds,
    };

    let data = serde_json::to_string_pretty(&export_file)?;
    fs::write(&output, data)
        .with_context(|| format!("failed to write export to {}", output.display()))?;
    println!("エクスポートしました: {}", output.display());
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum MergeStrategy {
    Skip,
    Overwrite,
    Interactive,
}

fn validate_import_file(file: &ExportFile) -> Result<()> {
    if file.schema_version != EXPORT_SCHEMA_VERSION {
        bail!(
            "Unsupported schema version: {}. This version of clipper supports v{} only.",
            file.schema_version,
            EXPORT_SCHEMA_VERSION
        );
    }
    Ok(())
}

fn merge_commands(
    existing: Vec<CmdEntry>,
    imported: Vec<CmdEntry>,
    strategy: MergeStrategy,
) -> Result<Vec<CmdEntry>> {
    let mut result = existing.clone();
    let mut added = 0;
    let mut skipped = 0;
    let mut overwritten = 0;

    for import_cmd in imported {
        if let Some(pos) = result.iter().position(|c| c.name == import_cmd.name) {
            // 衝突が発生
            match strategy {
                MergeStrategy::Skip => {
                    skipped += 1;
                }
                MergeStrategy::Overwrite => {
                    result[pos] = import_cmd;
                    overwritten += 1;
                }
                MergeStrategy::Interactive => {
                    let theme = ColorfulTheme::default();
                    let choices = vec![
                        format!("[上書き] インポートするコマンドで置き換える ({})", import_cmd.cmd),
                        format!("[スキップ] 既存のコマンドを保持 ({})", result[pos].cmd),
                        format!("[リネーム] 新しい名前でインポート ({}_imported)", import_cmd.name),
                    ];

                    let sel = FuzzySelect::with_theme(&theme)
                        .with_prompt(format!("'{}' は既に存在します。どうしますか？", import_cmd.name))
                        .items(&choices)
                        .default(0)
                        .interact()?;

                    match sel {
                        0 => {
                            // 上書き
                            result[pos] = import_cmd;
                            overwritten += 1;
                        }
                        1 => {
                            // スキップ
                            skipped += 1;
                        }
                        2 => {
                            // リネーム
                            let new_name = format!("{}_imported", import_cmd.name);
                            result.push(CmdEntry {
                                name: new_name,
                                cmd: import_cmd.cmd,
                            });
                            added += 1;
                        }
                        _ => unreachable!(),
                    }
                }
            }
        } else {
            // 衝突なし、追加
            result.push(import_cmd);
            added += 1;
        }
    }

    // サマリー表示
    let mut summary_parts = Vec::new();
    if added > 0 {
        summary_parts.push(format!("{}件追加", added));
    }
    if overwritten > 0 {
        summary_parts.push(format!("{}件上書き", overwritten));
    }
    if skipped > 0 {
        summary_parts.push(format!("{}件スキップ（重複）", skipped));
    }

    if summary_parts.is_empty() {
        println!("インポートするコマンドがありませんでした");
    } else {
        println!("インポートしました: {}", summary_parts.join("、"));
    }

    Ok(result)
}

fn cmd_import(args: &[String]) -> Result<()> {
    if args.len() < 2 {
        bail!("clipper import <path> [--overwrite|--merge|--append-only]");
    }

    let import_path = PathBuf::from(&args[1]);

    // オプション解析
    let strategy = if args.len() >= 3 {
        match args[2].as_str() {
            "--overwrite" => MergeStrategy::Overwrite,
            "--merge" => MergeStrategy::Interactive,
            "--append-only" => MergeStrategy::Skip,
            _ => bail!("unknown option: {}. use --overwrite, --merge, or --append-only", args[2]),
        }
    } else {
        MergeStrategy::Skip
    };

    // インポートファイルを読み込み
    let import_data = fs::read_to_string(&import_path)
        .with_context(|| format!("failed to read import file: {}", import_path.display()))?;

    let import_file: ExportFile = serde_json::from_str(&import_data)
        .with_context(|| format!("invalid JSON in import file: {}", import_path.display()))?;

    // バリデーション
    validate_import_file(&import_file)?;

    // 既存のコマンドを読み込み
    let path = commands_path()?;
    ensure_commands_file(&path)?;
    let existing = load_commands(&path)?;

    // マージ
    let merged = merge_commands(existing, import_file.commands, strategy)?;

    // 保存
    save_commands(&path, &merged)?;

    Ok(())
}

/* ------------ entry ------------ */

fn print_usage() {
    eprintln!(
        "usage:\n  clipper run <partial-name>\n  clipper add [name] [cmd]\n  clipper delete\n  clipper export [--output <path>]\n  clipper import <path> [--overwrite|--merge|--append-only]\n\nexamples:\n  clipper run bu\n  clipper add serve \"python -m http.server\"\n  clipper delete\n  clipper export\n  clipper export --output ./commands.json\n  clipper import commands-export-20250105-120000.json\n  clipper import commands.json --overwrite\n  clipper import commands.json --merge"
    );
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    match args[1].as_str() {
        "run" => cmd_run(&args[1..])?,
        "add" => cmd_add(&args[1..])?,
        "delete" => {
            if args.len() != 2 {
                print_usage();
                return Ok(());
            }
            cmd_delete()?
        }
        "export" => cmd_export(&args[1..])?,
        "import" => cmd_import(&args[1..])?,
        _ => {
            print_usage();
        }
    }

    Ok(())
}
