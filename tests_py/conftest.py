"""
pytest configuration and shared fixtures for clipper integration tests.

All tests run in isolation using temporary directories for config and shim directories.
Environment variables are set to avoid polluting the user's actual configuration.
"""

import os
import platform
import subprocess
from pathlib import Path
from typing import Dict

import pytest


def is_windows() -> bool:
    """Check if running on Windows."""
    return platform.system() == "Windows"


def get_shim_extension() -> str:
    """Get the expected shim file extension for the current OS."""
    return ".cmd" if is_windows() else ""


@pytest.fixture(scope="session")
def clipper_bin(tmp_path_factory) -> Path:
    """
    Build clipper binary once per test session.

    Returns the path to the clipper executable.
    """
    # Find the project root (parent of tests_py/)
    project_root = Path(__file__).parent.parent

    # Build the binary
    print("\nBuilding clipper binary...")
    result = subprocess.run(
        ["cargo", "build"],
        cwd=project_root,
        capture_output=True,
        text=True
    )

    if result.returncode != 0:
        pytest.fail(f"Failed to build clipper:\nstdout: {result.stdout}\nstderr: {result.stderr}")

    # Return path to the debug binary
    binary_name = "clipper.exe" if is_windows() else "clipper"
    binary_path = project_root / "target" / "debug" / binary_name

    if not binary_path.exists():
        pytest.fail(f"Binary not found at {binary_path}")

    print(f"Built clipper at: {binary_path}")
    return binary_path


@pytest.fixture
def isolated_env(tmp_path, monkeypatch) -> Dict[str, Path]:
    """
    Create an isolated environment for each test.

    Returns a dict with:
    - config_dir: temporary config directory
    - shim_dir: temporary shim directory
    - home_dir: temporary home directory

    Environment variables are set to redirect all config/data to tmp_path.
    """
    # Create temporary directories
    config_dir = tmp_path / "config"
    shim_dir = tmp_path / "shims"
    home_dir = tmp_path / "home"

    config_dir.mkdir()
    shim_dir.mkdir()
    home_dir.mkdir()

    # Set environment variables to isolate the test
    # Override config directory location
    if is_windows():
        # Windows uses APPDATA for config
        monkeypatch.setenv("APPDATA", str(config_dir))
    else:
        # Linux/macOS can use XDG_CONFIG_HOME
        monkeypatch.setenv("XDG_CONFIG_HOME", str(config_dir))

    # Override HOME to prevent any accidental pollution
    monkeypatch.setenv("HOME", str(home_dir))
    if is_windows():
        monkeypatch.setenv("USERPROFILE", str(home_dir))

    # Set CLIPPER_SHIM_DIR environment variable (to be used by implementation)
    monkeypatch.setenv("CLIPPER_SHIM_DIR", str(shim_dir))

    return {
        "config_dir": config_dir,
        "shim_dir": shim_dir,
        "home_dir": home_dir,
    }


def run_clipper(clipper_bin: Path, args: list, env: dict = None, cwd: Path = None, input_text: str = None) -> subprocess.CompletedProcess:
    """
    Run clipper with the given arguments.

    Args:
        clipper_bin: Path to the clipper binary
        args: List of command-line arguments
        env: Optional environment variables (merged with os.environ)
        cwd: Optional working directory
        input_text: Optional stdin input

    Returns:
        CompletedProcess with stdout, stderr, and returncode
    """
    cmd = [str(clipper_bin)] + args

    # Merge environment variables
    full_env = os.environ.copy()
    if env:
        full_env.update(env)

    result = subprocess.run(
        cmd,
        capture_output=True,
        text=True,
        env=full_env,
        cwd=cwd,
        input=input_text
    )

    return result
