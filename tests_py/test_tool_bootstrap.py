"""
Integration tests for clipper tool bootstrap feature.

Tests are organized according to TDD_prompt.md requirements:
A. tool add → config stores tools
B. bootstrap → shims are generated
C. tool install → install command executes
D. shim execution → __shim__ is called with args
E. export/import → tools and shim.dir are preserved
"""

import json
import os
import stat
from pathlib import Path

import pytest

from conftest import run_clipper, get_shim_extension, is_windows


def get_config_path(isolated_env: dict) -> Path:
    """Get the expected path to the clipper config file."""
    config_dir = isolated_env["config_dir"]
    if is_windows():
        # Windows: APPDATA/clipper/commands.json
        return config_dir / "clipper" / "commands.json"
    else:
        # Linux/macOS: XDG_CONFIG_HOME/clipper/commands.json
        return config_dir / "clipper" / "commands.json"


def load_config(isolated_env: dict) -> dict:
    """Load and parse the clipper config file."""
    config_path = get_config_path(isolated_env)
    if not config_path.exists():
        pytest.fail(f"Config file not found at {config_path}")

    with open(config_path, "r", encoding="utf-8") as f:
        return json.load(f)


# ============================================================================
# Test Case A: tool add stores tools in config
# ============================================================================

def test_tool_add_stores_in_config(clipper_bin, isolated_env):
    """
    Test Case A: tool add command stores tool definition in config.

    Steps:
    1. Run: clipper tool add rg --install "sudo apt install ripgrep" --exec "rg"
    2. Load config file
    3. Verify:
       - schemaVersion is 2 (or higher)
       - tools section exists
       - tools["rg"] exists with correct install and exec
    """
    result = run_clipper(
        clipper_bin,
        ["tool", "add", "rg", "--install", "sudo apt install ripgrep", "--exec", "rg"]
    )

    # Should succeed
    assert result.returncode == 0, f"tool add failed:\nstdout: {result.stdout}\nstderr: {result.stderr}"

    # Load config
    config = load_config(isolated_env)

    # Verify schema version
    assert "schemaVersion" in config, "schemaVersion missing in config"
    assert config["schemaVersion"] >= 2, f"Expected schemaVersion >= 2, got {config['schemaVersion']}"

    # Verify tools section exists
    assert "tools" in config, "tools section missing in config"
    assert isinstance(config["tools"], dict), "tools should be a dict"

    # Verify the tool entry
    assert "rg" in config["tools"], "rg tool not found in tools"
    tool = config["tools"]["rg"]

    assert "install" in tool, "install field missing in tool"
    assert tool["install"]["cmd"] == "sudo apt install ripgrep", "install cmd mismatch"

    assert "exec" in tool, "exec field missing in tool"
    # exec can be string or object with type/value
    if isinstance(tool["exec"], dict):
        assert tool["exec"]["value"] == "rg", "exec value mismatch"
    else:
        assert tool["exec"] == "rg", "exec mismatch"


# ============================================================================
# Test Case B: bootstrap generates shims
# ============================================================================

def test_bootstrap_generates_shims(clipper_bin, isolated_env):
    """
    Test Case B: bootstrap command generates shims for all tools.

    Steps:
    1. Add a tool: clipper tool add mytool --install "echo install" --exec "echo mytool"
    2. Run: clipper bootstrap
    3. Verify:
       - Shim file exists at shim_dir/mytool (or mytool.cmd on Windows)
       - Shim has correct permissions (executable on POSIX)
       - Shim content calls: clipper __shim__ mytool -- "$@"
    """
    shim_dir = isolated_env["shim_dir"]

    # Add a tool
    result = run_clipper(
        clipper_bin,
        ["tool", "add", "mytool", "--install", "echo install", "--exec", "echo mytool"]
    )
    assert result.returncode == 0, f"tool add failed: {result.stderr}"

    # Run bootstrap
    result = run_clipper(clipper_bin, ["bootstrap"])
    assert result.returncode == 0, f"bootstrap failed:\nstdout: {result.stdout}\nstderr: {result.stderr}"

    # Check shim file exists
    shim_name = "mytool" + get_shim_extension()
    shim_path = shim_dir / shim_name
    assert shim_path.exists(), f"Shim file not found at {shim_path}"

    # Check shim content
    shim_content = shim_path.read_text(encoding="utf-8")
    assert "__shim__" in shim_content, "Shim should contain __shim__ command"
    assert "mytool" in shim_content, "Shim should reference tool name"

    # Check executable permissions on POSIX
    if not is_windows():
        file_stat = shim_path.stat()
        is_executable = bool(file_stat.st_mode & stat.S_IXUSR)
        assert is_executable, f"Shim {shim_path} should be executable on POSIX"


def test_bootstrap_print_path(clipper_bin, isolated_env):
    """
    Test Case B (continued): bootstrap --print-path outputs shim directory path.

    Steps:
    1. Run: clipper bootstrap --print-path
    2. Verify:
       - stdout contains the shim directory path
       - Should show instructions for adding to PATH
    """
    shim_dir = isolated_env["shim_dir"]

    result = run_clipper(clipper_bin, ["bootstrap", "--print-path"])
    assert result.returncode == 0, f"bootstrap --print-path failed: {result.stderr}"

    # Output should contain shim directory path
    assert str(shim_dir) in result.stdout, f"Expected shim directory {shim_dir} in output:\n{result.stdout}"


# ============================================================================
# Test Case C: tool install executes install command
# ============================================================================

def test_tool_install_executes_command(clipper_bin, isolated_env, tmp_path):
    """
    Test Case C: tool install command executes the stored install recipe.

    Uses a dummy install command that creates a marker file to verify execution.

    Steps:
    1. Add a tool with a dummy install command (creates a file)
    2. Run: clipper tool install <name>
    3. Verify:
       - Install command was executed (marker file exists)
       - Exit code is 0
    """
    # Create a marker file path in tmp
    marker_file = tmp_path / "install_marker.txt"

    # Install command: create the marker file
    if is_windows():
        install_cmd = f'cmd /C "echo installed > {marker_file}"'
    else:
        install_cmd = f'sh -c "echo installed > {marker_file}"'

    # Add tool with this install command
    result = run_clipper(
        clipper_bin,
        ["tool", "add", "dummytool", "--install", install_cmd, "--exec", "echo dummy"]
    )
    assert result.returncode == 0, f"tool add failed: {result.stderr}"

    # Run tool install
    result = run_clipper(clipper_bin, ["tool", "install", "dummytool"])
    assert result.returncode == 0, f"tool install failed:\nstdout: {result.stdout}\nstderr: {result.stderr}"

    # Verify marker file was created
    assert marker_file.exists(), f"Install command did not create marker file at {marker_file}"


def test_tool_install_all(clipper_bin, isolated_env, tmp_path):
    """
    Test Case C (continued): tool install --all installs all tools.

    Steps:
    1. Add multiple tools with dummy install commands
    2. Run: clipper tool install --all
    3. Verify all tools were installed
    """
    marker1 = tmp_path / "marker1.txt"
    marker2 = tmp_path / "marker2.txt"

    if is_windows():
        cmd1 = f'cmd /C "echo tool1 > {marker1}"'
        cmd2 = f'cmd /C "echo tool2 > {marker2}"'
    else:
        cmd1 = f'sh -c "echo tool1 > {marker1}"'
        cmd2 = f'sh -c "echo tool2 > {marker2}"'

    # Add two tools
    run_clipper(clipper_bin, ["tool", "add", "tool1", "--install", cmd1, "--exec", "echo t1"])
    run_clipper(clipper_bin, ["tool", "add", "tool2", "--install", cmd2, "--exec", "echo t2"])

    # Install all
    result = run_clipper(clipper_bin, ["tool", "install", "--all"])
    assert result.returncode == 0, f"tool install --all failed: {result.stderr}"

    # Verify both were installed
    assert marker1.exists(), "tool1 was not installed"
    assert marker2.exists(), "tool2 was not installed"


# ============================================================================
# Test Case D: shim execution calls __shim__ with args
# ============================================================================

def test_shim_executes_via_shim_command(clipper_bin, isolated_env, tmp_path):
    """
    Test Case D: Shim execution flows through __shim__ internal command.

    Steps:
    1. Add a tool with exec pointing to a dummy script that echoes args to a file
    2. Run bootstrap to create shim
    3. Execute the shim with arguments
    4. Verify:
       - __shim__ command was invoked
       - Arguments were passed through
       - exec command was executed
    """
    output_file = tmp_path / "shim_output.txt"

    # Create a dummy executable script
    if is_windows():
        dummy_script = tmp_path / "dummy.cmd"
        dummy_script.write_text(f'@echo off\necho %* > {output_file}\n', encoding="utf-8")
        exec_cmd = str(dummy_script)
    else:
        dummy_script = tmp_path / "dummy.sh"
        dummy_script.write_text(f'#!/bin/sh\necho "$@" > {output_file}\n', encoding="utf-8")
        dummy_script.chmod(0o755)
        exec_cmd = str(dummy_script)

    # Add tool
    result = run_clipper(
        clipper_bin,
        ["tool", "add", "shimtest", "--install", "echo install", "--exec", exec_cmd]
    )
    assert result.returncode == 0, f"tool add failed: {result.stderr}"

    # Bootstrap to create shim
    result = run_clipper(clipper_bin, ["bootstrap"])
    assert result.returncode == 0, f"bootstrap failed: {result.stderr}"

    # Test: Call __shim__ directly (simulating what the shim would do)
    result = run_clipper(clipper_bin, ["__shim__", "shimtest", "--", "arg1", "arg2"])
    assert result.returncode == 0, f"__shim__ failed:\nstdout: {result.stdout}\nstderr: {result.stderr}"

    # Verify output file was created with arguments
    assert output_file.exists(), f"Output file not created at {output_file}"
    content = output_file.read_text(encoding="utf-8").strip()
    assert "arg1" in content, f"arg1 not found in output: {content}"
    assert "arg2" in content, f"arg2 not found in output: {content}"


def test_shim_recursion_guard(clipper_bin, isolated_env):
    """
    Test Case D (continued): __shim__ detects and prevents infinite recursion.

    Steps:
    1. Add a tool without exec (only name)
    2. Run: clipper __shim__ <name> --
    3. Verify:
       - Should fail with clear error message
       - Error should suggest setting exec
    """
    # Add tool without exec
    result = run_clipper(
        clipper_bin,
        ["tool", "add", "notool", "--install", "echo install"]
    )
    assert result.returncode == 0, f"tool add failed: {result.stderr}"

    # Try to run via __shim__
    result = run_clipper(clipper_bin, ["__shim__", "notool", "--"])

    # Should fail with helpful error
    assert result.returncode != 0, "__shim__ should fail when exec is not set"
    # Error message should mention exec or configuration
    stderr_lower = result.stderr.lower()
    assert "exec" in stderr_lower or "configure" in stderr_lower or "set" in stderr_lower, \
        f"Error message should mention exec configuration:\n{result.stderr}"


# ============================================================================
# Test Case E: export/import preserves tools and shim.dir
# ============================================================================

def test_export_import_preserves_tools(clipper_bin, isolated_env, tmp_path):
    """
    Test Case E: export/import preserves tools and shim.dir.

    Steps:
    1. Add tools in first environment
    2. Export to file
    3. Create new isolated environment
    4. Import the file
    5. Verify:
       - tools are preserved
       - shim.dir is preserved
       - Can run bootstrap in new environment
    """
    export_file = tmp_path / "export.json"

    # Add tools
    run_clipper(clipper_bin, ["tool", "add", "tool1", "--install", "echo i1", "--exec", "echo e1"])
    run_clipper(clipper_bin, ["tool", "add", "tool2", "--install", "echo i2", "--exec", "echo e2"])

    # Run bootstrap to set shim.dir in config
    run_clipper(clipper_bin, ["bootstrap"])

    # Export
    result = run_clipper(clipper_bin, ["export", "--output", str(export_file)])
    assert result.returncode == 0, f"export failed: {result.stderr}"
    assert export_file.exists(), "Export file not created"

    # Load export file and verify structure
    with open(export_file, "r", encoding="utf-8") as f:
        export_data = json.load(f)

    assert "schemaVersion" in export_data, "schemaVersion missing in export"
    assert export_data["schemaVersion"] >= 2, f"Expected schemaVersion >= 2 in export"
    assert "tools" in export_data, "tools missing in export"
    assert len(export_data["tools"]) == 2, "Expected 2 tools in export"
    assert "shim" in export_data, "shim section missing in export"
    assert "dir" in export_data["shim"], "shim.dir missing in export"

    # Create a new isolated environment for import test
    new_config_dir = tmp_path / "new_config"
    new_shim_dir = tmp_path / "new_shims"
    new_config_dir.mkdir()
    new_shim_dir.mkdir()

    import_env = os.environ.copy()
    if is_windows():
        import_env["APPDATA"] = str(new_config_dir)
    else:
        import_env["XDG_CONFIG_HOME"] = str(new_config_dir)
    import_env["CLIPPER_SHIM_DIR"] = str(new_shim_dir)

    # Import in new environment
    result = run_clipper(clipper_bin, ["import", str(export_file), "--overwrite"], env=import_env)
    assert result.returncode == 0, f"import failed:\nstdout: {result.stdout}\nstderr: {result.stderr}"

    # Load config in new environment and verify
    if is_windows():
        new_config_path = new_config_dir / "clipper" / "commands.json"
    else:
        new_config_path = new_config_dir / "clipper" / "commands.json"

    assert new_config_path.exists(), f"Config not created after import at {new_config_path}"

    with open(new_config_path, "r", encoding="utf-8") as f:
        imported_config = json.load(f)

    assert "tools" in imported_config, "tools not imported"
    assert "tool1" in imported_config["tools"], "tool1 not imported"
    assert "tool2" in imported_config["tools"], "tool2 not imported"
    assert "shim" in imported_config, "shim not imported"

    # Verify bootstrap works in new environment
    result = run_clipper(clipper_bin, ["bootstrap"], env=import_env)
    assert result.returncode == 0, f"bootstrap after import failed: {result.stderr}"

    # Verify shims were created
    shim_ext = get_shim_extension()
    assert (new_shim_dir / f"tool1{shim_ext}").exists(), "tool1 shim not created after import+bootstrap"
    assert (new_shim_dir / f"tool2{shim_ext}").exists(), "tool2 shim not created after import+bootstrap"
