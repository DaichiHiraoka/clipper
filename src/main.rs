use anyhow::{bail, Context, Result};
use dialoguer::{theme::ColorfulTheme, Confirm, FuzzySelect, Input};
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CmdEntry {
    name: String,
    cmd: String,
}

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

/* ------------ entry ------------ */

fn print_usage() {
    eprintln!(
        "usage:\n  clipper run <partial-name>\n  clipper add [name] [cmd]\n\nexamples:\n  clipper run bu\n  clipper add serve \"python -m http.server\""
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
        _ => {
            print_usage();
        }
    }

    Ok(())
}
