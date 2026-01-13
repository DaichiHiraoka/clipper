use anyhow::{bail, Context, Result};
use dialoguer::{theme::ColorfulTheme, Confirm, FuzzySelect, Input};
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use chrono::Utc;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CmdEntry {
    name: String,
    cmd: String,
}

// --- Schema Version 2 Structures ---

#[derive(Debug, Serialize, Deserialize, Clone)]
struct InstallConfig {
    cmd: String,
    #[serde(default = "default_shell")]
    shell: String,
}

fn default_shell() -> String {
    if cfg!(target_os = "windows") {
        "cmd".to_string()
    } else {
        "sh".to_string()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ExecConfig {
    String { value: String },
    Array { value: Vec<String> },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Tool {
    name: String,
    install: InstallConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    exec: Option<ExecConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<String>>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    created_at: Option<String>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ShimConfig {
    dir: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ConfigFileV2 {
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    #[serde(default)]
    commands: Vec<CmdEntry>,
    #[serde(default)]
    tools: HashMap<String, Tool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shim: Option<ShimConfig>,
}

// Export file structure (used for import/export)
#[derive(Debug, Serialize, Deserialize)]
struct ExportFile {
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    #[serde(rename = "exportedAt")]
    exported_at: String,
    commands: Vec<CmdEntry>,
    #[serde(default)]
    tools: HashMap<String, Tool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shim: Option<ShimConfig>,
}

const CURRENT_SCHEMA_VERSION: u32 = 2;

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

fn default_shim_dir() -> Result<String> {
    // Check environment variable first
    if let Ok(dir) = std::env::var("CLIPPER_SHIM_DIR") {
        return Ok(dir);
    }

    // Otherwise use platform defaults
    if cfg!(target_os = "windows") {
        let local_app_data = std::env::var("LOCALAPPDATA")
            .or_else(|_| std::env::var("APPDATA"))
            .context("cannot locate LOCALAPPDATA or APPDATA")?;
        Ok(format!("{}\\clipper\\bin", local_app_data))
    } else {
        let home = std::env::var("HOME").context("cannot locate HOME")?;
        Ok(format!("{}/.local/bin", home))
    }
}

fn ensure_config_file(path: &Path) -> Result<ConfigFileV2> {
    if path.exists() {
        return load_config(path);
    }

    // Create new v2 config with sample commands
    let sample_commands = vec![
        CmdEntry {
            name: "build".to_string(),
            cmd: "cargo build".to_string(),
        },
        CmdEntry {
            name: "serve".to_string(),
            cmd: "python -m http.server".to_string(),
        },
        CmdEntry {
            name: "test-all".to_string(),
            cmd: "cargo test --all".to_string(),
        },
    ];

    let config = ConfigFileV2 {
        schema_version: CURRENT_SCHEMA_VERSION,
        commands: sample_commands,
        tools: HashMap::new(),
        shim: Some(ShimConfig {
            dir: default_shim_dir()?,
        }),
    };

    save_config(path, &config)?;
    Ok(config)
}

fn load_config(path: &Path) -> Result<ConfigFileV2> {
    let data = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    // Try to parse as v2 first
    if let Ok(config) = serde_json::from_str::<ConfigFileV2>(&data) {
        return Ok(config);
    }

    // Try to parse as v1 (array of CmdEntry)
    if let Ok(commands) = serde_json::from_str::<Vec<CmdEntry>>(&data) {
        // Migrate v1 to v2
        let config = ConfigFileV2 {
            schema_version: CURRENT_SCHEMA_VERSION,
            commands,
            tools: HashMap::new(),
            shim: Some(ShimConfig {
                dir: default_shim_dir()?,
            }),
        };
        // Save migrated config
        save_config(path, &config)?;
        return Ok(config);
    }

    bail!("invalid config file format: {}", path.display())
}

fn save_config(path: &Path, config: &ConfigFileV2) -> Result<()> {
    let data = serde_json::to_string_pretty(config)?;
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
    let config = ensure_config_file(&path)?;
    let cmds = &config.commands;

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
    let mut config = ensure_config_file(&path)?;

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

    if let Some(i) = config.commands.iter().position(|c| c.name == name) {
        if !Confirm::new()
            .with_prompt(format!("'{}' は既に存在します。上書きしますか？", name))
            .interact()?
        {
            println!("キャンセルしました");
            return Ok(());
        }
        config.commands[i] = CmdEntry { name, cmd };
    } else {
        config.commands.push(CmdEntry { name, cmd });
    }

    save_config(&path, &config)?;
    println!("コマンドを追加しました");
    Ok(())
}

fn cmd_delete() -> Result<()> {
    let path = commands_path()?;
    let mut config = ensure_config_file(&path)?;

    if config.commands.is_empty() {
        println!("登録済みのコマンドがありません");
        return Ok(());
    }

    let theme = ColorfulTheme::default();
    let items: Vec<String> = config
        .commands
        .iter()
        .map(|c| format!("{}  →  {}", c.name, c.cmd))
        .collect();

    let sel = FuzzySelect::with_theme(&theme)
        .with_prompt("削除するコマンドを選択（タイプで絞込）")
        .items(&items)
        .default(0)
        .interact()?;

    let target = config.commands[sel].clone();
    let confirmed = Confirm::new()
        .with_prompt(format!("'{}' を削除しますか？ (Y/n)", target.name))
        .default(true)
        .interact()?;

    if !confirmed {
        println!("キャンセルしました");
        return Ok(());
    }

    config.commands.remove(sel);
    save_config(&path, &config)?;
    println!("削除しました");
    Ok(())
}

fn cmd_export(args: &[String]) -> Result<()> {
    let path = commands_path()?;
    let config = ensure_config_file(&path)?;

    let output = match args.len() {
        1 => default_export_path()?,
        3 if args[1] == "--output" || args[1] == "-o" => PathBuf::from(&args[2]),
        _ => {
            bail!("invalid args for export. use: clipper export [--output <path>]");
        }
    };

    let export_file = ExportFile {
        schema_version: CURRENT_SCHEMA_VERSION,
        exported_at: Utc::now().to_rfc3339(),
        commands: config.commands,
        tools: config.tools,
        shim: config.shim,
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
    // Support both v1 and v2
    if file.schema_version < 1 || file.schema_version > CURRENT_SCHEMA_VERSION {
        bail!(
            "Unsupported schema version: {}. This version of clipper supports v1-v{} only.",
            file.schema_version,
            CURRENT_SCHEMA_VERSION
        );
    }
    Ok(())
}

fn merge_config(
    existing: ConfigFileV2,
    imported: ExportFile,
    strategy: MergeStrategy,
) -> Result<ConfigFileV2> {
    let mut result = existing;
    let mut added = 0;
    let mut skipped = 0;
    let mut overwritten = 0;

    // Merge commands
    for import_cmd in imported.commands {
        if let Some(pos) = result.commands.iter().position(|c| c.name == import_cmd.name) {
            match strategy {
                MergeStrategy::Skip => {
                    skipped += 1;
                }
                MergeStrategy::Overwrite => {
                    result.commands[pos] = import_cmd;
                    overwritten += 1;
                }
                MergeStrategy::Interactive => {
                    let theme = ColorfulTheme::default();
                    let choices = vec![
                        format!("[上書き] インポートするコマンドで置き換える ({})", import_cmd.cmd),
                        format!("[スキップ] 既存のコマンドを保持 ({})", result.commands[pos].cmd),
                        format!("[リネーム] 新しい名前でインポート ({}_imported)", import_cmd.name),
                    ];

                    let sel = FuzzySelect::with_theme(&theme)
                        .with_prompt(format!("'{}' は既に存在します。どうしますか？", import_cmd.name))
                        .items(&choices)
                        .default(0)
                        .interact()?;

                    match sel {
                        0 => {
                            result.commands[pos] = import_cmd;
                            overwritten += 1;
                        }
                        1 => {
                            skipped += 1;
                        }
                        2 => {
                            let mut new_name = format!("{}_imported", import_cmd.name);
                            let mut counter = 2;
                            while result.commands.iter().any(|c| c.name == new_name) {
                                new_name = format!("{}_imported_{}", import_cmd.name, counter);
                                counter += 1;
                            }
                            result.commands.push(CmdEntry {
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
            result.commands.push(import_cmd);
            added += 1;
        }
    }

    // Merge tools (always overwrite by name for simplicity)
    for (name, tool) in imported.tools {
        result.tools.insert(name.clone(), tool);
    }

    // Update shim config if provided
    if let Some(shim) = imported.shim {
        result.shim = Some(shim);
    }

    // Update schema version
    result.schema_version = CURRENT_SCHEMA_VERSION;

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
    if args.len() > 3 {
        bail!("too many arguments. usage: clipper import <path> [--overwrite|--merge|--append-only]");
    }

    let strategy = if args.len() == 3 {
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

    // 既存の設定を読み込み
    let path = commands_path()?;
    let existing = ensure_config_file(&path)?;

    // マージ
    let merged = merge_config(existing, import_file, strategy)?;

    // 保存
    save_config(&path, &merged)?;

    Ok(())
}

/* ------------ new tool commands ------------ */

fn cmd_tool_add(args: &[String]) -> Result<()> {
    // Parse: tool add <name> --install "<cmd>" [--exec "<cmd>"] [--shell <sh>]
    if args.len() < 2 {
        bail!("usage: clipper tool add <name> --install \"<cmd>\" [--exec \"<cmd>\"] [--shell <sh>]");
    }

    let name = args[1].clone();
    let mut install_cmd: Option<String> = None;
    let mut exec_cmd: Option<String> = None;
    let mut shell: Option<String> = None;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--install" => {
                if i + 1 >= args.len() {
                    bail!("--install requires an argument");
                }
                install_cmd = Some(args[i + 1].clone());
                i += 2;
            }
            "--exec" => {
                if i + 1 >= args.len() {
                    bail!("--exec requires an argument");
                }
                exec_cmd = Some(args[i + 1].clone());
                i += 2;
            }
            "--shell" => {
                if i + 1 >= args.len() {
                    bail!("--shell requires an argument");
                }
                shell = Some(args[i + 1].clone());
                i += 2;
            }
            _ => {
                bail!("unknown option: {}", args[i]);
            }
        }
    }

    let install_cmd = install_cmd.context("--install is required")?;

    let path = commands_path()?;
    let mut config = ensure_config_file(&path)?;

    let tool = Tool {
        name: name.clone(),
        install: InstallConfig {
            cmd: install_cmd,
            shell: shell.unwrap_or_else(default_shell),
        },
        exec: exec_cmd.map(|cmd| ExecConfig::String { value: cmd }),
        tags: None,
        created_at: Some(Utc::now().to_rfc3339()),
        updated_at: Some(Utc::now().to_rfc3339()),
    };

    config.tools.insert(name.clone(), tool);
    save_config(&path, &config)?;

    println!("Tool '{}' added successfully", name);
    Ok(())
}

fn cmd_bootstrap(args: &[String]) -> Result<()> {
    // Parse: bootstrap [--force] [--print-path]
    let mut force = false;
    let mut print_path = false;

    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "--force" => force = true,
            "--print-path" => print_path = true,
            _ => bail!("unknown option: {}", arg),
        }
    }

    let path = commands_path()?;
    let mut config = ensure_config_file(&path)?;

    // Ensure shim.dir is set
    // Priority: CLIPPER_SHIM_DIR env var > config.shim.dir > default
    let shim_dir_str = if let Ok(dir) = std::env::var("CLIPPER_SHIM_DIR") {
        // Update config with env var value
        config.shim = Some(ShimConfig { dir: dir.clone() });
        save_config(&path, &config)?;
        dir
    } else if let Some(ref shim) = config.shim {
        shim.dir.clone()
    } else {
        let dir = default_shim_dir()?;
        config.shim = Some(ShimConfig { dir: dir.clone() });
        save_config(&path, &config)?;
        dir
    };

    let shim_dir = PathBuf::from(&shim_dir_str);

    // Create shim directory if it doesn't exist
    if !shim_dir.exists() {
        fs::create_dir_all(&shim_dir)
            .with_context(|| format!("failed to create shim directory: {}", shim_dir.display()))?;
    }

    // Generate shims for all tools
    let mut generated = 0;
    for (name, _tool) in &config.tools {
        let shim_path = if cfg!(target_os = "windows") {
            shim_dir.join(format!("{}.cmd", name))
        } else {
            shim_dir.join(name)
        };

        // Check if shim already exists
        if shim_path.exists() && !force {
            eprintln!("Warning: shim already exists: {}. Use --force to overwrite.", shim_path.display());
            continue;
        }

        // Generate shim content
        let shim_content = if cfg!(target_os = "windows") {
            format!("@echo off\nclipper __shim__ \"{}\" -- %*\n", name)
        } else {
            format!("#!/usr/bin/env sh\nexec clipper __shim__ \"{}\" -- \"$@\"\n", name)
        };

        fs::write(&shim_path, shim_content)
            .with_context(|| format!("failed to write shim: {}", shim_path.display()))?;

        // Set executable permission on POSIX
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&shim_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&shim_path, perms)?;
        }

        generated += 1;
    }

    println!("Generated {} shim(s) in: {}", generated, shim_dir.display());

    if print_path {
        println!("\nAdd this directory to your PATH:");
        if cfg!(target_os = "windows") {
            println!("  setx PATH \"%PATH%;{}\"", shim_dir.display());
        } else {
            println!("  export PATH=\"{}:$PATH\"", shim_dir.display());
            println!("\nAdd the above line to your shell profile (~/.bashrc, ~/.zshrc, etc.)");
        }
    }

    Ok(())
}

fn cmd_tool_install(args: &[String]) -> Result<()> {
    // Parse: tool install <name> or tool install --all
    if args.len() < 2 {
        bail!("usage: clipper tool install <name> or clipper tool install --all");
    }

    let path = commands_path()?;
    let config = ensure_config_file(&path)?;

    let install_all = args[1] == "--all";

    if install_all {
        // Install all tools
        for (name, tool) in &config.tools {
            println!("Installing tool: {}", name);
            execute_install_command(&tool.install)?;
        }
        println!("All tools installed successfully");
    } else {
        // Install specific tool
        let name = &args[1];
        let tool = config
            .tools
            .get(name)
            .with_context(|| format!("tool '{}' not found", name))?;

        println!("Installing tool: {}", name);
        execute_install_command(&tool.install)?;
        println!("Tool '{}' installed successfully", name);
    }

    Ok(())
}

fn execute_install_command(install: &InstallConfig) -> Result<()> {
    let status = if cfg!(target_os = "windows") {
        if install.shell == "cmd" {
            Command::new("cmd").args(["/C", &install.cmd]).status()?
        } else {
            Command::new(&install.shell).arg(&install.cmd).status()?
        }
    } else {
        if install.shell == "sh" {
            Command::new("sh").arg("-c").arg(&install.cmd).status()?
        } else {
            Command::new(&install.shell).arg("-c").arg(&install.cmd).status()?
        }
    };

    if !status.success() {
        bail!("install command failed with exit code: {:?}", status.code());
    }

    Ok(())
}

fn cmd_shim(args: &[String]) -> Result<()> {
    // Parse: __shim__ <name> -- <args...>
    // Find the tool and execute it
    if args.len() < 2 {
        bail!("usage: clipper __shim__ <name> -- <args...>");
    }

    let name = &args[1];

    // Find the "--" separator
    let separator_pos = args.iter().position(|arg| arg == "--");
    let tool_args = if let Some(pos) = separator_pos {
        &args[pos + 1..]
    } else {
        &[]
    };

    let path = commands_path()?;
    let config = ensure_config_file(&path)?;

    let tool = config
        .tools
        .get(name)
        .with_context(|| format!("tool '{}' not found", name))?;

    // Determine what to execute
    let exec_result = if let Some(ref exec) = tool.exec {
        match exec {
            ExecConfig::String { value } => {
                // Execute the exec command with args
                let mut cmd = Command::new(value);
                cmd.args(tool_args);
                cmd.status()
            }
            ExecConfig::Array { value } => {
                if value.is_empty() {
                    bail!("exec array is empty for tool '{}'", name);
                }
                let mut cmd = Command::new(&value[0]);
                cmd.args(&value[1..]);
                cmd.args(tool_args);
                cmd.status()
            }
        }
    } else {
        // No exec specified - fail with helpful message
        bail!(
            "Tool '{}' has no exec configured. Please run:\n  clipper tool add {} --install \"{}\" --exec \"<executable>\"",
            name,
            name,
            tool.install.cmd
        );
    };

    match exec_result {
        Ok(status) => {
            std::process::exit(status.code().unwrap_or(1));
        }
        Err(e) => {
            bail!("failed to execute tool '{}': {}", name, e);
        }
    }
}

/* ------------ entry ------------ */

fn print_usage() {
    eprintln!(r#"clipper - コマンドランナー & ツール環境ブートストラップ

使い方:
  clipper <コマンド> [オプション]

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📋 コマンドランナー機能
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  run [部分文字列]      登録したコマンドを検索・実行
  add [name] [cmd]      新しいコマンドを追加
  delete                コマンドを削除（対話式）

  例:
    clipper run bu                      # "bu"を含むコマンドを検索
    clipper add serve "python -m http.server"
    clipper delete

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🛠️  Tool Bootstrap機能
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  tool add <名前> --install "<インストールコマンド>" [--exec "<実行コマンド>"]
      ツールのインストール方法を登録

  tool install <名前>   登録したツールをインストール
  tool install --all    すべてのツールをインストール

  bootstrap [--force] [--print-path]
      登録したツール用のshimファイルを生成
      --force       既存のshimを上書き
      --print-path  PATHの設定方法を表示

  例:
    # ツールを登録
    clipper tool add rg --install "cargo install ripgrep" --exec "rg"

    # shimを生成してPATH設定を確認
    clipper bootstrap --print-path

    # ツールをインストール
    clipper tool install rg
    clipper tool install --all

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
💾 エクスポート・インポート
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  export [--output <パス>]
      コマンドとツールをJSONファイルに出力

  import <パス> [--overwrite|--merge|--append-only]
      JSONファイルからコマンドとツールをインポート
      --overwrite    重複時に上書き
      --merge        重複時に対話式で選択
      --append-only  重複をスキップ（デフォルト）

  例:
    clipper export
    clipper export --output backup.json
    clipper import backup.json --overwrite

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🚀 新しいマシンでの環境構築フロー
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  1. 元のマシン:    clipper export
  2. ファイル転送:  JSONファイルを新マシンにコピー
  3. 新マシン:      clipper import <ファイル> --overwrite
  4. 新マシン:      clipper bootstrap --print-path
  5. PATH設定:      表示された手順に従う
  6. 新マシン:      clipper tool install --all

詳細: https://github.com/DaichiHiraoka/clipper
"#);
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
        "tool" => {
            if args.len() < 3 {
                eprintln!("clipper tool - ツール管理");
                eprintln!();
                eprintln!("使い方:");
                eprintln!("  clipper tool <サブコマンド> [オプション]");
                eprintln!();
                eprintln!("サブコマンド:");
                eprintln!("  add <名前> --install \"<コマンド>\" [--exec \"<実行コマンド>\"]");
                eprintln!("      ツールのインストール方法を登録");
                eprintln!();
                eprintln!("  install <名前>");
                eprintln!("      指定したツールをインストール");
                eprintln!();
                eprintln!("  install --all");
                eprintln!("      すべてのツールをインストール");
                eprintln!();
                eprintln!("例:");
                eprintln!("  clipper tool add rg --install \"cargo install ripgrep\" --exec \"rg\"");
                eprintln!("  clipper tool add fd --install \"brew install fd\" --exec \"fd\"");
                eprintln!("  clipper tool install rg");
                eprintln!("  clipper tool install --all");
                return Ok(());
            }
            match args[2].as_str() {
                "add" => cmd_tool_add(&args[2..])?,
                "install" => cmd_tool_install(&args[2..])?,
                _ => {
                    eprintln!("エラー: 未知のサブコマンド '{}'", args[2]);
                    eprintln!();
                    eprintln!("使用可能なサブコマンド: add, install");
                    eprintln!("詳細は 'clipper tool' を実行してください");
                }
            }
        }
        "bootstrap" => cmd_bootstrap(&args[1..])?,
        "__shim__" => cmd_shim(&args[1..])?,
        _ => {
            print_usage();
        }
    }

    Ok(())
}
