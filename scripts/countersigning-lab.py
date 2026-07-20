#!/usr/bin/env python3
"""Create and supervise the canonical Holochain countersigning lab.

The lab uses hc sandbox for deterministic app installation, then starts each
sandbox in its own process group so restart and availability faults are
addressable per clinical actor.
"""
from __future__ import annotations

import argparse
import json
import os
import pathlib
import shutil
import signal
import socket
import subprocess
import sys
import time
from typing import Any

ROOT = pathlib.Path(__file__).resolve().parents[1]
TOPOLOGY_PATH = ROOT / "tests/countersigning/lab-topology.json"
DEFAULT_STATE = ROOT / "target/countersigning-lab"


def fail(message: str, code: int = 1) -> "NoReturn":
    print(f"countersigning lab: {message}", file=sys.stderr)
    raise SystemExit(code)


def topology() -> dict[str, Any]:
    data = json.loads(TOPOLOGY_PATH.read_text())
    if data.get("schema_version") != 1:
        fail("unsupported topology schema")
    return data


def conductor(data: dict[str, Any], name: str) -> tuple[int, dict[str, Any]]:
    for index, item in enumerate(data["conductors"]):
        if item["name"] == name:
            return index, item
    fail(f"unknown conductor {name!r}")


def state_paths(state: pathlib.Path) -> dict[str, pathlib.Path]:
    return {
        "root": state,
        "control": state / "control",
        "sandboxes": state / "sandboxes",
        "run": state / "run",
        "logs": state / "logs",
        "runtime": state / "runtime",
    }


def require(command: str) -> str:
    resolved = shutil.which(command)
    if resolved is None:
        fail(f"required executable is missing: {command}", 2)
    return resolved


def generate_command(data: dict[str, Any], paths: dict[str, pathlib.Path], happ: pathlib.Path) -> list[str]:
    admin_ports = ",".join(str(item["admin_port"]) for item in data["conductors"])
    names = ",".join(item["name"] for item in data["conductors"])
    return [
        "hc", "sandbox", "--force-admin-ports", admin_ports,
        "generate",
        "--app-id", data["installed_app_id"],
        "--num-sandboxes", str(len(data["conductors"])),
        "--root", str(paths["sandboxes"]),
        "--directories", names,
        "--in-process-lair",
        "--network-seed", data["network_seed"],
        str(happ),
    ]


def run_command(item: dict[str, Any], index: int) -> list[str]:
    return [
        "hc", "sandbox", "--force-admin-ports", str(item["admin_port"]),
        "run", "--ports", str(item["app_port"]), str(index),
    ]


def pid_path(paths: dict[str, pathlib.Path], name: str) -> pathlib.Path:
    return paths["run"] / f"{name}.pid"


def read_pid(paths: dict[str, pathlib.Path], name: str) -> int | None:
    path = pid_path(paths, name)
    try:
        value = int(path.read_text().strip())
    except (FileNotFoundError, ValueError):
        return None
    try:
        os.kill(value, 0)
    except ProcessLookupError:
        path.unlink(missing_ok=True)
        return None
    return value


def wait_port(port: int, timeout: float = 60.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=0.25):
                return
        except OSError:
            time.sleep(0.2)
    fail(f"port {port} did not become ready within {timeout:.0f}s")


def prepare(args: argparse.Namespace) -> None:
    data = topology()
    paths = state_paths(args.state)
    require("hc")
    require("holochain")
    happ = args.happ.resolve()
    if not happ.is_file():
        fail(f"packed hApp does not exist: {happ}", 2)
    if paths["root"].exists() and any(paths["root"].iterdir()):
        if not args.reset:
            fail(f"state directory is not empty: {paths['root']} (pass --reset)")
        stop_all(data, paths, tolerate_missing=True)
        shutil.rmtree(paths["root"])
    for key in ("control", "sandboxes", "run", "logs", "runtime"):
        paths[key].mkdir(parents=True, exist_ok=True)
    command = generate_command(data, paths, happ)
    (paths["runtime"] / "generate-command.json").write_text(json.dumps(command, indent=2) + "\n")
    subprocess.run(command, cwd=paths["control"], check=True)
    hc_file = paths["control"] / ".hc"
    if not hc_file.is_file():
        fail("hc sandbox did not create the expected .hc inventory")
    os.chmod(hc_file, 0o600)
    print(f"prepared {len(data['conductors'])} sandboxes under {paths['root']}")


def start_one(data: dict[str, Any], paths: dict[str, pathlib.Path], name: str, wait: bool = True) -> None:
    require("hc")
    index, item = conductor(data, name)
    if read_pid(paths, name) is not None:
        fail(f"{name} is already running")
    if not (paths["control"] / ".hc").is_file():
        fail("lab is not prepared; run prepare first")
    paths["run"].mkdir(parents=True, exist_ok=True)
    paths["logs"].mkdir(parents=True, exist_ok=True)
    log = open(paths["logs"] / f"{name}.log", "ab", buffering=0)
    process = subprocess.Popen(
        run_command(item, index),
        cwd=paths["control"],
        stdout=log,
        stderr=subprocess.STDOUT,
        start_new_session=True,
    )
    pid_path(paths, name).write_text(f"{process.pid}\n")
    os.chmod(pid_path(paths, name), 0o600)
    if wait:
        try:
            wait_port(item["admin_port"])
            wait_port(item["app_port"])
        except BaseException:
            stop_one(data, paths, name, tolerate_missing=True)
            raise
    print(f"started {name}: pid={process.pid} admin={item['admin_port']} app={item['app_port']}")


def stop_one(data: dict[str, Any], paths: dict[str, pathlib.Path], name: str, tolerate_missing: bool = False) -> None:
    conductor(data, name)
    pid = read_pid(paths, name)
    if pid is None:
        if tolerate_missing:
            return
        fail(f"{name} is not running")
    try:
        os.killpg(pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        try:
            os.kill(pid, 0)
        except ProcessLookupError:
            break
        time.sleep(0.1)
    else:
        try:
            os.killpg(pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    pid_path(paths, name).unlink(missing_ok=True)
    print(f"stopped {name}")


def stop_all(data: dict[str, Any], paths: dict[str, pathlib.Path], tolerate_missing: bool = False) -> None:
    for item in reversed(data["conductors"]):
        stop_one(data, paths, item["name"], tolerate_missing=tolerate_missing)


def signal_one(data: dict[str, Any], paths: dict[str, pathlib.Path], name: str, sig: signal.Signals) -> None:
    conductor(data, name)
    pid = read_pid(paths, name)
    if pid is None:
        fail(f"{name} is not running")
    os.killpg(pid, sig)
    print(f"sent {sig.name} to {name}")


def status(data: dict[str, Any], paths: dict[str, pathlib.Path]) -> None:
    result = []
    for item in data["conductors"]:
        pid = read_pid(paths, item["name"])
        result.append({
            "name": item["name"],
            "pid": pid,
            "running": pid is not None,
            "admin_port": item["admin_port"],
            "app_port": item["app_port"],
        })
    print(json.dumps({"schema_version": 1, "conductors": result}, indent=2))


def plan(args: argparse.Namespace) -> None:
    data = topology()
    paths = state_paths(args.state)
    output = {
        "generate": generate_command(data, paths, args.happ.resolve()),
        "run": {item["name"]: run_command(item, index) for index, item in enumerate(data["conductors"])},
    }
    print(json.dumps(output, indent=2))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--state", type=pathlib.Path, default=DEFAULT_STATE)
    parser.add_argument("--happ", type=pathlib.Path, default=ROOT / "mycelix-health.happ")
    sub = parser.add_subparsers(dest="command", required=True)
    prepare_parser = sub.add_parser("prepare")
    prepare_parser.add_argument("--reset", action="store_true")
    sub.add_parser("plan")
    sub.add_parser("start-all")
    sub.add_parser("stop-all")
    sub.add_parser("status")
    for command in ("start", "stop", "restart", "suspend", "resume"):
        command_parser = sub.add_parser(command)
        command_parser.add_argument("name")
    args = parser.parse_args()
    args.state = args.state.resolve()
    data = topology()
    paths = state_paths(args.state)

    if args.command == "prepare":
        prepare(args)
    elif args.command == "plan":
        plan(args)
    elif args.command == "start-all":
        started: list[str] = []
        try:
            for item in data["conductors"]:
                start_one(data, paths, item["name"])
                started.append(item["name"])
        except BaseException:
            for name in reversed(started):
                stop_one(data, paths, name, tolerate_missing=True)
            raise
    elif args.command == "stop-all":
        stop_all(data, paths, tolerate_missing=True)
    elif args.command == "status":
        status(data, paths)
    elif args.command == "start":
        start_one(data, paths, args.name)
    elif args.command == "stop":
        stop_one(data, paths, args.name)
    elif args.command == "restart":
        stop_one(data, paths, args.name, tolerate_missing=True)
        start_one(data, paths, args.name)
    elif args.command == "suspend":
        signal_one(data, paths, args.name, signal.SIGSTOP)
    elif args.command == "resume":
        signal_one(data, paths, args.name, signal.SIGCONT)


if __name__ == "__main__":
    main()
