"""Read-only Docker health + database/proxy error logs; no article history export."""

import http.client
import json
import os
from pathlib import Path
import re
import signal
import socket
import struct
import threading
import time
import urllib.request

KEY = os.environ["POSTHOG_PROJECT_TOKEN"]
HOST = os.environ.get("POSTHOG_HOST", "https://eu.i.posthog.com").rstrip("/")
assert KEY.startswith("phc_") and HOST.startswith("https://"), (
    "Expected HTTPS and a public PostHog project token"
)
PROJECTS = {"news-scraper", "gusnews", "lmenews", "proxy"}
STOP = threading.Event()
STATE = Path("/state/cursors.json")
signal.signal(signal.SIGTERM, lambda *_: STOP.set())
signal.signal(signal.SIGINT, lambda *_: STOP.set())


class Docker(http.client.HTTPConnection):
    def __init__(self):
        super().__init__("localhost", timeout=10)

    def connect(self):
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.sock.settimeout(self.timeout)
        self.sock.connect("/var/run/docker.sock")


def docker_get(path):
    connection = Docker()
    try:
        connection.request("GET", path)
        response = connection.getresponse()
        body = response.read()
        if response.status != 200:
            raise RuntimeError(f"Docker GET failed: HTTP {response.status}")
        return json.loads(body)
    finally:
        connection.close()


def post(path, payload):
    request = urllib.request.Request(
        HOST + path,
        json.dumps(payload).encode(),
        {"Content-Type": "application/json", "Authorization": "Bearer " + KEY},
    )
    try:
        with urllib.request.urlopen(request, timeout=10) as response:
            response.read()
        return True
    except Exception as error:
        print(
            "PostHog delivery failed:",
            type(error).__name__,
            getattr(error, "code", ""),
            flush=True,
        )
        return False


def event(name, service, **properties):
    return {
        "event": name,
        "properties": {
            "distinct_id": "service:infrastructure-monitor",
            "$process_person_profile": False,
            "service": service,
            "environment": "production",
            **properties,
        },
    }


def log_record(body, severity=9, **attributes):
    return {
        "timeUnixNano": str(time.time_ns()),
        "severityNumber": severity,
        "severityText": "ERROR"
        if severity >= 17
        else "WARN"
        if severity >= 13
        else "INFO",
        "body": {"stringValue": body},
        "attributes": [
            {"key": key, "value": {"stringValue": str(value)}}
            for key, value in attributes.items()
        ],
    }


def unpack_logs(stream):
    pending = b""
    while header := stream.read(8):
        if len(header) != 8 or header[0] not in (1, 2) or header[1:4] != b"\0\0\0":
            raise ValueError("Invalid Docker log frame")
        size = struct.unpack(">I", header[4:])[0]
        if size > 16 * 1024 * 1024:
            raise ValueError("Docker log frame exceeds 16 MiB")
        data = stream.read(size)
        if len(data) != size:
            raise ValueError("Truncated Docker log frame")
        lines = (pending + data).split(b"\n")
        pending = lines.pop()
        if len(pending) > 16 * 1024 * 1024:
            raise ValueError("Docker log line exceeds 16 MiB")
        for line in lines:
            yield line.decode(errors="replace")
    if pending:
        yield pending.decode(errors="replace")


def docker_logs(path):
    connection = Docker()
    try:
        connection.request("GET", path)
        response = connection.getresponse()
        if response.status != 200:
            raise RuntimeError(f"Docker logs failed: HTTP {response.status}")
        yield from unpack_logs(response)
    finally:
        connection.close()


def log_severity(line):
    if re.search(r"(?i)\b(error|err|panic|fatal)\b", line):
        return 17
    if re.search(r"(?i)\b(warn|warning|wrn)\b", line):
        return 13
    return None


def deliver(service, events, logs):
    delivered = not events or post("/batch/", {"api_key": KEY, "batch": events})
    if logs:
        resource = {
            "resource": {
                "attributes": [
                    {"key": "service.name", "value": {"stringValue": service}},
                    {
                        "key": "deployment.environment",
                        "value": {"stringValue": "production"},
                    },
                ]
            },
            "scopeLogs": [{"scope": {"name": "docker-monitor"}, "logRecords": logs}],
        }
        delivered = post("/i/v1/logs", {"resourceLogs": [resource]}) and delivered
    if not delivered:
        raise RuntimeError("PostHog delivery failed; retaining log cursors")


def redact(line, environment):
    for entry in environment:
        name, _, value = entry.partition("=")
        if len(value) > 5 and name.endswith(
            ("KEY", "TOKEN", "PASSWORD", "PASS", "SECRET")
        ):
            line = line.replace(value, "[redacted]")
    return re.sub(
        r"(?i)https?://\S+|bearer\s+\S+|\b(?:sk-|phc_|phx_)[\w-]+|eyJ[\w.-]+|[\w.+-]+@[\w.-]+\.[a-z]{2,}",
        "[redacted]",
        line,
    )[:1500]


def main():
    state = json.loads(STATE.read_text()) if STATE.exists() else {}
    while not STOP.is_set():
        next_state = dict(state)
        try:
            for container in docker_get("/containers/json?all=1"):
                labels = container.get("Labels", {})
                project = labels.get("com.docker.compose.project")
                if project not in PROJECTS:
                    continue
                events, logs = [], []
                identifier = container["Id"]
                info = docker_get(f"/containers/{identifier}/json")
                service = labels.get("com.docker.compose.service", "unknown").replace(
                    "_", "-"
                )
                if project in ("gusnews", "lmenews"):
                    service = project + "-server"
                if project == "proxy":
                    service = "traefik"
                status = info["State"]["Status"]
                health = info["State"].get("Health", {}).get("Status", "not-configured")
                restarts = info["RestartCount"]
                previous = state.get(identifier, {})
                changed = any(
                    previous.get(key) != value
                    for key, value in (
                        ("status", status),
                        ("health", health),
                        ("restarts", restarts),
                    )
                )
                properties = dict(
                    container_id=identifier[:12],
                    status=status,
                    health=health,
                    restart_count=restarts,
                    oom_killed=info["State"].get("OOMKilled", False),
                )
                events.append(event("container_health", service, **properties))
                if changed:
                    failed = (
                        status != "running"
                        or health == "unhealthy"
                        or (previous and restarts > previous.get("restarts", 0))
                    )
                    events.append(
                        event("container_state_changed", service, **properties)
                    )
                    logs.append(
                        log_record(
                            "Container state changed", 17 if failed else 9, **properties
                        )
                    )
                    if failed:
                        events.append(
                            event(
                                "$exception",
                                service,
                                **{
                                    "$exception_list": [
                                        {
                                            "type": "ContainerHealthError",
                                            "value": f"{service}: status={status}, health={health}, restarts={restarts}",
                                            "mechanism": {
                                                "type": "monitor",
                                                "handled": True,
                                            },
                                        }
                                    ]
                                },
                            )
                        )
                now = int(time.time())
                if service in ("surrealdb", "traefik"):
                    since = previous.get("since", now - 300)
                    if info["Config"].get("Tty"):
                        raise ValueError(
                            "Infrastructure log collection requires non-TTY containers"
                        )
                    for line in docker_logs(
                        f"/containers/{identifier}/logs?stdout=1&stderr=1&timestamps=1&since={since}&until={now}"
                    ):
                        severity = log_severity(line)
                        if severity is None:
                            continue
                        body = redact(line, info["Config"].get("Env", []))
                        logs.append(log_record(body, severity))
                        if severity == 17:
                            events.append(
                                event(
                                    "$exception",
                                    service,
                                    **{
                                        "$exception_list": [
                                            {
                                                "type": "InfrastructureError",
                                                "value": body,
                                                "mechanism": {
                                                    "type": "log",
                                                    "handled": True,
                                                },
                                            }
                                        ]
                                    },
                                )
                            )
                        if len(logs) >= 100:
                            deliver(service, events, logs)
                            events, logs = [], []
                deliver(service, events, logs)
                next_state[identifier] = {
                    "status": status,
                    "health": health,
                    "restarts": restarts,
                    "since": now,
                }
            deliver(
                "infrastructure-monitor",
                [event("service_heartbeat", "infrastructure-monitor")],
                [],
            )
            state = next_state
            STATE.write_text(json.dumps(state))
        except Exception as error:
            print("Monitor cycle failed:", type(error).__name__, flush=True)
        STOP.wait(300)


if __name__ == "__main__":
    main()
