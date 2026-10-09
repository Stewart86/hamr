#!/usr/bin/env python3

import fcntl
import hashlib
import json
import math
import os
import socket
import sys
import tempfile
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

API_URL = "https://api.livetennisapi.com/api/public/v1/matches?status=live&limit=200"
# 24 * 60 * 60 / 900 = 96 attempts per day, below the free limit of 100.
REQUEST_INTERVAL = 900
MAX_RESPONSE_BYTES = 2 * 1024 * 1024

MOCK_SNAPSHOT = {
    "matches": [
        {
            "id": 1,
            "tournament": "Example tournament",
            "players": {"p1": {"name": "Alex Example"}, "p2": {"name": "Sam Example"}},
            "score": {
                "games": [[6, 3], [4, 4]],
                "points": ["40", "30"],
                "server": 1,
                "is_tiebreak": False,
            },
        }
    ],
    "has_more": False,
    "total": 1,
}


class SnapshotError(Exception):
    pass


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def clean_text(value, fallback=""):
    if not isinstance(value, str):
        return fallback
    return " ".join("".join(c for c in value if c.isprintable() or c.isspace()).split())


def parse_snapshot(payload):
    if not isinstance(payload, dict) or not isinstance(payload.get("data"), list):
        raise SnapshotError("The service returned an invalid snapshot.")
    meta = payload.get("meta")
    if not isinstance(meta, dict) or not isinstance(meta.get("has_more"), bool):
        raise SnapshotError("The service returned invalid pagination information.")
    total = meta.get("total")
    if total is not None and (type(total) is not int or total < 0):
        raise SnapshotError("The service returned invalid pagination information.")
    if len(payload["data"]) > 200:
        raise SnapshotError("The service returned an oversized snapshot.")
    matches = []
    for match in payload["data"]:
        if not isinstance(match, dict) or type(match.get("id")) is not int:
            raise SnapshotError("The service returned an invalid match.")
        players = match.get("players")
        if not isinstance(players, dict):
            raise SnapshotError("The service returned an invalid match.")
        names = {}
        for side in ("p1", "p2"):
            player = players.get(side)
            if not isinstance(player, dict) or not isinstance(player.get("name"), str):
                raise SnapshotError("The service returned an invalid player.")
            names[side] = {"name": clean_text(player["name"], "Unknown player") or "Unknown player"}
        score = match.get("score")
        if score is not None and not isinstance(score, dict):
            raise SnapshotError("The service returned an invalid score.")
        matches.append({
            "id": match["id"],
            "tournament": clean_text(match.get("tournament"), "Unknown tournament"),
            "players": names,
            "score": score,
        })
    return {"matches": matches, "has_more": meta["has_more"], "total": total}


def fetch_snapshot(api_key):
    request = urllib.request.Request(
        API_URL, headers={"X-API-Key": api_key, "Accept": "application/json"}
    )
    try:
        with urllib.request.build_opener(NoRedirect()).open(request, timeout=5) as response:
            if response.status != 200:
                raise SnapshotError("The service refused the snapshot request.")
            body = response.read(MAX_RESPONSE_BYTES + 1)
        if len(body) > MAX_RESPONSE_BYTES:
            raise SnapshotError("The service returned an oversized snapshot.")
        return parse_snapshot(json.loads(body))
    except urllib.error.HTTPError as error:
        code = error.code
        error.close()
        if code == 401:
            raise SnapshotError("The API credential was rejected.") from None
        if code == 429:
            raise SnapshotError("The service request limit was reached.") from None
        raise SnapshotError("The service refused the snapshot request.") from None
    except (urllib.error.URLError, TimeoutError, socket.timeout, OSError, ValueError):
        raise SnapshotError("The snapshot request failed.") from None


def cache_paths(api_key):
    base = Path(os.environ.get("XDG_CACHE_HOME") or Path.home() / ".cache")
    directory = base / "hamr" / "livetennis"
    digest = hashlib.sha256(api_key.encode()).hexdigest()
    return directory, directory / (digest + ".json"), directory / (digest + ".lock")


def read_state(path):
    try:
        with path.open() as stream:
            state = json.load(stream)
    except FileNotFoundError:
        return {"version": 1, "attempted_at": 0, "fetched_at": None, "snapshot": None, "error": ""}
    except (OSError, ValueError):
        raise SnapshotError("The stored snapshot cannot be read. No request was sent.") from None
    if not isinstance(state, dict) or type(state.get("version")) is not int or state["version"] != 1:
        raise SnapshotError("The stored snapshot is invalid. No request was sent.")
    if any(name not in state for name in ("attempted_at", "fetched_at", "snapshot", "error")):
        raise SnapshotError("The stored snapshot is invalid. No request was sent.")
    for name in ("attempted_at", "fetched_at"):
        value = state.get(name)
        if name == "fetched_at" and value is None:
            continue
        if type(value) not in (int, float) or not math.isfinite(value) or value < 0:
            raise SnapshotError("The stored request time is invalid. No request was sent.")
        try:
            datetime.fromtimestamp(value, timezone.utc)
        except (OverflowError, OSError, ValueError):
            raise SnapshotError("The stored request time is invalid. No request was sent.") from None
    if state["attempted_at"] == 0:
        raise SnapshotError("The stored request time is invalid. No request was sent.")
    snapshot = state.get("snapshot")
    if (snapshot is None) != (state["fetched_at"] is None):
        raise SnapshotError("The stored snapshot is invalid. No request was sent.")
    if snapshot is not None:
        if not isinstance(snapshot, dict):
            raise SnapshotError("The stored snapshot is invalid. No request was sent.")
        state["snapshot"] = parse_snapshot({
            "data": snapshot.get("matches"),
            "meta": {"has_more": snapshot.get("has_more"), "total": snapshot.get("total")},
        })
    if not isinstance(state.get("error"), str):
        raise SnapshotError("The stored snapshot is invalid. No request was sent.")
    return state


def write_state(path, state):
    descriptor, temporary = tempfile.mkstemp(prefix="snapshot-", dir=path.parent)
    try:
        with os.fdopen(descriptor, "w") as stream:
            json.dump(state, stream, allow_nan=False)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
        directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def get_state(api_key, refresh=True):
    if os.environ.get("HAMR_TEST_MODE") == "1":
        return {"snapshot": MOCK_SNAPSHOT, "fetched_at": time.time(), "error": "Test mode. No API request was sent."}
    if not api_key:
        raise SnapshotError("Set LIVETENNIS_API_KEY in the Hamr daemon environment, then restart the daemon.")
    directory, path, lock_path = cache_paths(api_key)
    if not refresh:
        return read_state(path)
    try:
        directory.mkdir(parents=True, exist_ok=True, mode=0o700)
        descriptor = os.open(lock_path, os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
        with os.fdopen(descriptor, "a") as lock:
            try:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError:
                state = read_state(path)
                state["error"] = "A snapshot request is already running."
                return state
            state = read_state(path)
            now = time.time()
            if now - state["attempted_at"] < REQUEST_INTERVAL:
                return state
            state["attempted_at"] = now
            state["error"] = "The last snapshot request did not finish."
            write_state(path, state)
            try:
                snapshot = fetch_snapshot(api_key)
            except SnapshotError as error:
                state["error"] = str(error)
            else:
                state["snapshot"] = snapshot
                state["fetched_at"] = time.time()
                state["error"] = ""
            write_state(path, state)
            return state
    except OSError:
        raise SnapshotError("The request cache cannot be saved. No further request was sent.") from None


def score_text(score):
    if not isinstance(score, dict):
        return "Score unavailable"
    games = score.get("games")
    if games is None:
        return "Score withheld"
    pairs = []
    if (isinstance(games, list) and len(games) == 2
            and all(isinstance(side, list) for side in games)
            and len(games[0]) == len(games[1])):
        for left, right in zip(*games):
            if type(left) is not int or type(right) is not int or min(left, right) < 0:
                return "Score unavailable"
            pairs.append(f"{left}-{right}")
    if not pairs:
        return "Score unavailable"
    text = " ".join(pairs)
    points = score.get("points")
    if isinstance(points, list) and len(points) == 2:
        if all(value is None or isinstance(value, str) for value in points):
            label = "Tiebreak points" if score.get("is_tiebreak") is True else "Points"
            text += " | " + label + ": " + "-".join(clean_text(value, "?") or "?" for value in points)
    server = score.get("server")
    if type(server) is int and server in (1, 2):
        text += f" | Player {server} serving"
    return text


def match_text(match):
    players = match["players"]
    name = players["p1"]["name"] + " vs " + players["p2"]["name"]
    return name, match["tournament"] + " | " + score_text(match["score"])


def status_text(state):
    text = state.get("error", "")
    fetched = state.get("fetched_at")
    if fetched is not None:
        timestamp = time.strftime("%Y-%m-%d %H:%M UTC", time.gmtime(fetched))
        text = (text + " " if text else "") + "Snapshot from " + timestamp + "."
    else:
        text = (text + " " if text else "") + "No snapshot is available yet."
    snapshot = state.get("snapshot")
    if snapshot is not None and snapshot["has_more"]:
        total = snapshot["total"]
        count = len(snapshot["matches"])
        text += f" Showing the first {count}" + (f" of {total}" if total is not None else "") + " matches."
    text += " Requests are at least 15 minutes apart."
    return text


def handle_request(request):
    step = request.get("step", "initial")
    api_key = os.environ.get("LIVETENNIS_API_KEY", "").strip()
    if step == "action":
        selected = request.get("selected") or {}
        match_id = selected.get("id", "")
        if match_id == "__status__":
            return {"type": "noop"}
        state = get_state(api_key, refresh=False)
        snapshot = state.get("snapshot")
        for match in snapshot["matches"] if snapshot else []:
            if str(match["id"]) == str(match_id):
                name, description = match_text(match)
                timestamp = time.strftime("%Y-%m-%d %H:%M UTC", time.gmtime(state["fetched_at"]))
                return {"type": "execute", "copy": name + "\n" + description + "\nSnapshot from " + timestamp + ".", "close": True}
        return {"type": "error", "message": "This match is no longer in the stored snapshot."}
    if step not in ("initial", "search"):
        return {"type": "noop"}
    state = get_state(api_key)
    snapshot = state.get("snapshot")
    query = clean_text(request.get("query", "")).casefold()
    results = []
    for match in snapshot["matches"] if snapshot else []:
        name, description = match_text(match)
        if query in (name + " " + match["tournament"]).casefold():
            results.append({"id": str(match["id"]), "name": name, "description": description,
                            "icon": "sports_tennis", "verb": "Copy score"})
    if not results:
        if snapshot is None:
            name = "Snapshot unavailable"
        elif query:
            name = "No matching matches in this snapshot"
        else:
            name = "No live matches"
        results.append({"id": "__status__", "name": name,
                        "description": status_text(state), "icon": "sports_tennis"})
    else:
        results.append({"id": "__status__", "name": "Score snapshot", "description": status_text(state), "icon": "info"})
    return {"type": "results", "results": results, "inputMode": "realtime", "placeholder": "Search players or tournaments"}


def main():
    try:
        request = json.load(sys.stdin)
        if not isinstance(request, dict):
            raise ValueError
        response = handle_request(request)
    except SnapshotError as error:
        response = {"type": "error", "message": str(error)}
    except (ValueError, TypeError, KeyError, AttributeError):
        response = {"type": "error", "message": "The plugin request is invalid."}
    print(json.dumps(response))


if __name__ == "__main__":
    main()
