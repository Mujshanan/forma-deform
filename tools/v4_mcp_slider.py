#!/usr/bin/env python3
"""Experimental AniCraft MCP bone-to-global-playhead Slider prototype."""
from __future__ import annotations

import argparse
import csv
import json
import math
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

PROTOCOL = "2025-06-18"


class McpError(RuntimeError):
    pass


class AniCraftMcp:
    """Minimal MCP stdio client for the five tools documented by AniCraft."""

    def __init__(self, executable: str, port: int):
        self.proc = subprocess.Popen(
            [executable, "mcp", "--bridge", str(port)],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=None,
            text=True, encoding="utf-8", bufsize=1,
        )
        self.next_id = 1
        self.request("initialize", {
            "protocolVersion": PROTOCOL, "capabilities": {},
            "clientInfo": {"name": "forma-v4-mcp-slider", "version": "0.1.0"},
        })
        self.notify("notifications/initialized")

    def close(self):
        if self.proc.poll() is None:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=3)
            except subprocess.TimeoutExpired:
                self.proc.kill()
                self.proc.wait(timeout=3)

    def request(self, method: str, params: dict[str, Any] | None = None):
        if self.proc.poll() is not None:
            raise McpError(f"anicraft-cli exited with status {self.proc.returncode}")
        assert self.proc.stdin and self.proc.stdout
        request_id = self.next_id
        self.next_id += 1
        message = {"jsonrpc": "2.0", "id": request_id, "method": method, "params": params or {}}
        self.proc.stdin.write(json.dumps(message) + "\n")
        self.proc.stdin.flush()
        line = self.proc.stdout.readline()
        if not line:
            raise McpError("AniCraft MCP closed stdout before replying")
        try:
            response = json.loads(line)
        except json.JSONDecodeError as exc:
            raise McpError(f"invalid MCP response: {line[:200]!r}") from exc
        if response.get("id") != request_id:
            raise McpError(f"MCP response ID mismatch: {response.get('id')!r}")
        if "error" in response:
            err = response["error"]
            raise McpError(f"MCP {method}: {err.get('code')} {err.get('message')}")
        return response.get("result", {})

    def notify(self, method: str):
        assert self.proc.stdin
        self.proc.stdin.write(json.dumps({"jsonrpc": "2.0", "method": method}) + "\n")
        self.proc.stdin.flush()

    def tool(self, name: str, arguments: dict[str, Any] | None = None):
        result = self.request("tools/call", {"name": name, "arguments": arguments or {}})
        if result.get("isError"):
            detail = "\n".join(x.get("text", "") for x in result.get("content", []))
            raise McpError(f"{name}: {detail or result}")
        texts = [x.get("text", "") for x in result.get("content", []) if x.get("type") == "text"]
        if not texts:
            return result
        text = "\n".join(texts)
        try:
            return json.loads(text)
        except json.JSONDecodeError:
            return text

    def command(self, command: str, params: dict[str, Any] | None = None):
        return self.tool("anicraft_run", {"command": command, "params": params or {}})


def slider_frame(angle: float, min_angle: float, max_angle: float,
                 start: int, end: int, policy: str = "exclusive") -> int:
    if not all(math.isfinite(v) for v in (angle, min_angle, max_angle)):
        raise ValueError("angles must be finite")
    if min_angle == max_angle or start < 0 or end <= start:
        raise ValueError("invalid angle or frame range")
    if policy not in ("exclusive", "inclusive"):
        raise ValueError("policy must be exclusive or inclusive")
    t = min(1.0, max(0.0, (angle - min_angle) / (max_angle - min_angle)))
    span = end - start
    offset = min(math.floor(t * span), span - 1) if policy == "exclusive" else math.floor(t * span + 0.5)
    return start + offset


def require_object(value: Any, source: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise McpError(f"{source} did not return a JSON object")
    return value


def discover(client: AniCraftMcp) -> dict[str, dict[str, Any]]:
    found = {}
    for filter_text in ("bone info", "timeline goto", "bone pose"):
        result = client.tool("anicraft_commands", {"filter": filter_text})
        if not isinstance(result, list):
            raise McpError(f"command discovery for {filter_text!r} returned no list")
        for command in result:
            if isinstance(command, dict) and isinstance(command.get("id"), str):
                found[command["id"]] = command
    missing = [x for x in ("bone.info", "timeline.goto") if x not in found]
    if missing:
        raise McpError("missing required AniCraft command(s): " + ", ".join(missing))
    return found


def document_frame(client: AniCraftMcp) -> int:
    document = require_object(client.tool("anicraft_document"), "anicraft_document")
    frame = document.get("frame")
    if not isinstance(frame, int) or frame < 0:
        raise McpError(f"document returned invalid current frame: {frame!r}")
    return frame


def bone_angle(client: AniCraftMcp, layer: int, bone_id: int, frame: int) -> float:
    data = require_object(client.command("bone.info", {"layer": layer, "frame": frame}), "bone.info")
    if data.get("frame") != frame or not isinstance(data.get("armatures"), list):
        raise McpError("bone.info returned an unexpected frame/armature response")
    for armature in data["armatures"]:
        if armature.get("layer") == layer:
            for bone in armature.get("bones", []):
                if bone.get("id") == bone_id:
                    angle = bone.get("angle")
                    if isinstance(angle, bool) or not isinstance(angle, (int, float)) or not math.isfinite(angle):
                        raise McpError(f"bone {bone_id} has no finite angle")
                    # Match V1: keyed joint angle only; spring swing is not added.
                    return float(angle)
    raise McpError(f"bone {bone_id} not found on armature layer {layer}")


def append_log(path: Path, row: dict[str, Any]):
    fields = ["timestamp_utc", "source_frame", "layer_id", "bone_id", "angle_deg",
              "target_frame", "bone_query_ms", "goto_ms", "round_trip_ms", "status", "error"]
    path.parent.mkdir(parents=True, exist_ok=True)
    exists = path.exists()
    with path.open("a", newline="", encoding="utf-8") as stream:
        writer = csv.DictWriter(stream, fieldnames=fields)
        if not exists:
            writer.writeheader()
        writer.writerow(row)


def cycle(client: AniCraftMcp, args, source_frame: int) -> float:
    started = time.perf_counter()
    row = {
        "timestamp_utc": datetime.now(timezone.utc).isoformat(),
        "source_frame": source_frame, "layer_id": args.layer, "bone_id": args.bone,
        "angle_deg": "", "target_frame": "", "bone_query_ms": "", "goto_ms": "",
        "round_trip_ms": "", "status": "error", "error": "",
    }
    try:
        t0 = time.perf_counter()
        angle = bone_angle(client, args.layer, args.bone, source_frame)
        row["bone_query_ms"] = round((time.perf_counter() - t0) * 1000, 3)
        target = slider_frame(angle, args.min_angle, args.max_angle,
                              args.start_frame, args.end_frame, args.endpoint_policy)
        row["angle_deg"], row["target_frame"] = angle, target
        t0 = time.perf_counter()
        result = require_object(client.command("timeline.goto", {"frame": target}), "timeline.goto")
        row["goto_ms"] = round((time.perf_counter() - t0) * 1000, 3)
        if result.get("frame") != target:
            raise McpError(f"goto reported frame {result.get('frame')!r}, expected {target}")
        row["status"] = "ok"
        print(f"bone {args.bone}: {angle:.3f}° -> global playhead frame {target}")
        return angle
    except Exception as exc:
        row["error"] = str(exc)
        raise
    finally:
        row["round_trip_ms"] = round((time.perf_counter() - started) * 1000, 3)
        append_log(args.log, row)


def arguments():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--cli", default="anicraft-cli")
    p.add_argument("--port", type=int, default=9878)
    p.add_argument("--layer", type=int, required=True, help="armature layer ID")
    p.add_argument("--bone", type=int, required=True, help="numeric bone ID")
    p.add_argument("--source-frame", type=int, help="fixed driver frame; required by --watch")
    p.add_argument("--min-angle", type=float, default=-45.0)
    p.add_argument("--max-angle", type=float, default=45.0)
    p.add_argument("--start-frame", type=int, default=0)
    p.add_argument("--end-frame", type=int, default=20)
    p.add_argument("--endpoint-policy", choices=("exclusive", "inclusive"), default="exclusive")
    p.add_argument("--watch", action="store_true")
    p.add_argument("--interval", type=float, default=0.25)
    p.add_argument("--cycles", type=int, default=0, help="watch updates; 0 runs until Ctrl+C")
    p.add_argument("--log", type=Path, default=Path("v4_mcp_latency.csv"))
    return p.parse_args()


def main() -> int:
    args = arguments()
    if args.port <= 0 or args.layer < 0 or args.bone < 0:
        print("port must be positive; layer and bone IDs must be non-negative", file=sys.stderr)
        return 2
    if args.watch and (args.source_frame is None or args.source_frame < 0):
        print("--watch requires a non-negative --source-frame", file=sys.stderr)
        return 2
    if args.interval < 0.2 or args.interval > 5:
        print("--interval must be between 0.2 and 5 seconds", file=sys.stderr)
        return 2
    try:
        slider_frame(0, args.min_angle, args.max_angle, args.start_frame, args.end_frame, args.endpoint_policy)
        client = AniCraftMcp(args.cli, args.port)
    except (ValueError, OSError, McpError) as exc:
        print(f"startup failed: {exc}", file=sys.stderr)
        return 1
    try:
        commands = discover(client)
        print("Discovered commands: bone.info, timeline.goto")
        if "bone.pose" in commands:
            print("bone.pose is available but is intentionally not used (driver pose remains artist-authored).")
        if args.watch:
            print(f"Watching only while the UI playhead is at source frame {args.source_frame}.")
            print("After each mapped goto, return to the source frame to make the next driver adjustment.")
            last_angle = None
            candidate = None
            stable = 0
            updates = 0
            while args.cycles == 0 or updates < args.cycles:
                if document_frame(client) == args.source_frame:
                    current = bone_angle(client, args.layer, args.bone, args.source_frame)
                    if candidate is not None and abs(current - candidate) <= 1e-6:
                        stable += 1
                    else:
                        candidate, stable = current, 1
                    if stable >= 2 and (last_angle is None or abs(current - last_angle) > 1e-6):
                        last_angle = cycle(client, args, args.source_frame)
                        updates += 1
                        stable = 0
                time.sleep(args.interval)
        else:
            source = args.source_frame if args.source_frame is not None else document_frame(client)
            cycle(client, args, source)
        return 0
    except KeyboardInterrupt:
        print("Stopped by user.")
        return 0
    except (McpError, ValueError, OSError) as exc:
        print(f"V4 prototype stopped: {exc}", file=sys.stderr)
        return 1
    finally:
        client.close()


if __name__ == "__main__":
    raise SystemExit(main())
