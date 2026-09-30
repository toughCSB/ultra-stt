"""Real loopback HTTP tests; the GPU test uses the production subprocess."""

from __future__ import annotations

import io
import json
import os
import secrets
import signal
import socket
import subprocess
import sys
import wave
from collections.abc import Iterator
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from threading import Event, Thread

import av
import httpx
import numpy as np
import pytest
import uvicorn

from local_stt.server import HOST, MAX_BODY, MODEL_NAME, ReadyServer, create_app

ROOT = Path(__file__).resolve().parents[2]
APP = ROOT / "opentypeless"
URL = "/v1/audio/transcriptions"
SECRET = "fixture-" + secrets.token_hex(32)


def wav(pcm: bytes, *, rate: int = 16000, channels: int = 1) -> bytes:
    output = io.BytesIO()
    with wave.open(output, "wb") as audio:
        audio.setnchannels(channels)
        audio.setsampwidth(2)
        audio.setframerate(rate)
        audio.writeframes(pcm)
    return output.getvalue()


def payload(audio: bytes) -> dict[str, tuple[str, bytes, str]]:
    return {"file": ("audio.wav", audio, "audio/wav")}


class FixtureRecognizer:
    def __init__(self) -> None:
        self.started = Event()
        self.release = Event()
        self.calls = 0
        self.fail = False

    def transcribe(self, samples: np.ndarray, language: str) -> str:
        self.calls += 1
        self.started.set()
        if self.fail:
            raise RuntimeError("GPU failed")
        if not self.release.wait(10):
            raise RuntimeError("Fixture inference was not released")
        return "" if not samples.any() else "한국어 2026"


class SignaledServer(ReadyServer):
    def __init__(self, config: uvicorn.Config, ready: Event) -> None:
        super().__init__(config)
        self.ready = ready

    async def startup(self, sockets: list[socket.socket] | None = None) -> None:
        # Tests subscribe to the exact server-start signal before starting it.
        await uvicorn.Server.startup(self, sockets=sockets)
        if self.started:
            self.ready.set()


@pytest.fixture
def service() -> Iterator[tuple[httpx.Client, FixtureRecognizer]]:
    recognizer = FixtureRecognizer()
    ready = Event()
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.bind(("127.0.0.1", 0))
        listener.listen(16)
        port = listener.getsockname()[1]
        config = uvicorn.Config(create_app(SECRET, recognizer, host=f"127.0.0.1:{port}"),
                                log_level="error", access_log=False)
        server = SignaledServer(config, ready)
        thread = Thread(target=lambda: __import__("asyncio").run(server.serve(sockets=[listener])), daemon=True)
        thread.start()
        assert ready.wait(10), "Server did not start"
        with httpx.Client(base_url=f"http://127.0.0.1:{port}", trust_env=False,
                          follow_redirects=False, timeout=10,
                          headers={"X-Ultra-STT-Session": SECRET}) as client:
            try:
                yield client, recognizer
            finally:
                recognizer.release.set()
                server.should_exit = True
                thread.join(10)
                assert not thread.is_alive(), "Fixture listener leaked"


def test_guards_validation_and_silence(service: tuple[httpx.Client, FixtureRecognizer]) -> None:
    client, recognizer = service
    valid = payload(wav(b"\0\0" * 16000))
    fields = {"model": MODEL_NAME, "language": "ko"}
    assert client.post(URL, files=valid, data=fields, headers={"X-Ultra-STT-Session": "wrong"}).status_code == 403
    assert client.post(URL, files=valid, data=fields, headers={"Host": "evil.test"}).status_code == 403
    for header in ({"Origin": "http://evil.test"}, {"Sec-Fetch-Site": "same-origin"}):
        assert client.post(URL, files=valid, data=fields, headers=header).status_code == 403
    assert client.options(URL).status_code == 403
    assert recognizer.calls == 0
    assert client.post(URL, files=valid, data={"model": "wrong"}).status_code == 422
    assert client.post(URL, files=payload(wav(b"\0\0" * 100, rate=8000)), data=fields).status_code == 422
    assert client.post(URL, files=payload(b"garbage"), data=fields).status_code == 422
    assert client.post(URL, files=payload(wav(b"\0\0" * 100, channels=2)), data=fields).status_code == 422
    assert client.post(URL, files=payload(wav(b"\0\0" * (12 * 1024 * 1024 + 1))),
                       data=fields).status_code == 422
    assert client.post(URL, content=b"x" * (MAX_BODY + 1),
                       headers={"Content-Type": "multipart/form-data; boundary=x"}).status_code == 413
    assert client.post(URL, content=(chunk for chunk in (b"x" * MAX_BODY, b"x")),
                       headers={"Content-Type": "multipart/form-data; boundary=x"}).status_code == 413
    recognizer.release.set()
    assert client.post(URL, files=valid, data=fields).json() == {"text": ""}
    assert recognizer.calls == 1


def test_busy_no_queue(service: tuple[httpx.Client, FixtureRecognizer]) -> None:
    client, recognizer = service
    with ThreadPoolExecutor(max_workers=1) as pool:
        first = pool.submit(client.post, URL, files=payload(wav(b"\x01\0" * 16000)),
                            data={"model": MODEL_NAME})
        assert recognizer.started.wait(10), "Inference never started"
        assert client.post(URL, files=payload(wav(b"\0\0" * 100)),
                           data={"model": MODEL_NAME}).status_code == 429
        recognizer.release.set()
        assert first.result(timeout=10).json() == {"text": "한국어 2026"}
    assert recognizer.calls == 1


def test_explicit_inference_failure(service: tuple[httpx.Client, FixtureRecognizer]) -> None:
    client, recognizer = service
    recognizer.release.set()

    recognizer.fail = True
    response = client.post(URL, files=payload(wav(b"\0\0" * 100)), data={"model": MODEL_NAME})
    assert response.status_code == 500
    assert response.json() == {"detail": "Local inference failed"}


def test_real_gpu_http_and_silence() -> None:
    secret = secrets.token_hex(32)
    env = {**os.environ, "ULTRA_STT_ROOT": str(ROOT), "HF_HUB_OFFLINE": "1"}
    process = subprocess.Popen([sys.executable, "-m", "local_stt.server"], cwd=APP,
                               env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, text=True)
    worker_pid: int | None = None
    try:
        assert process.stdin is not None and process.stdout is not None
        process.stdin.write(json.dumps({"protocol": 1, "session_secret": secret}) + "\n")
        process.stdin.close()
        with ThreadPoolExecutor(max_workers=1) as pool:
            ready = json.loads(pool.submit(process.stdout.readline).result(timeout=120))
        worker_pid = ready["pid"]
        assert ready == {"event": "ready", "protocol": 1, "pid": worker_pid,
                         "endpoint": f"http://{HOST}/v1"}
        with httpx.Client(base_url=f"http://{HOST}", trust_env=False, follow_redirects=False,
                          timeout=120, headers={"X-Ultra-STT-Session": secret}) as client:
            corpus = json.loads((ROOT / "qa" / "fleurs-ko-smoke.json").read_text(encoding="utf-8"))
            fixture = ROOT / "qa" / corpus["cases"][0]["audio"]
            with av.open(str(fixture)) as container:
                resampler = av.AudioResampler(format="s16", layout="mono", rate=16000)
                pcm = b"".join(frame.to_ndarray().tobytes()
                               for decoded in container.decode(audio=0)
                               for frame in resampler.resample(decoded))
            response = client.post(URL, files=payload(wav(pcm)),
                                   data={"model": MODEL_NAME, "language": "ko"})
            assert response.status_code == 200, response.text
            text = response.json()["text"]
            assert isinstance(text, str) and text.strip(), response.text
            print(f"GPU fixture {fixture.name}: {text!r}")
            response = client.post(URL, files=payload(wav(b"\0\0" * 32000)),
                                   data={"model": MODEL_NAME, "language": "ko"})
            assert response.status_code == 200, response.text
            assert response.json() == {"text": ""}
            print("GPU 2s silence: empty")
    finally:
        # Windows venv's python.exe launcher may be a parent of the actual
        # interpreter. The ready receipt identifies the listener process.
        if worker_pid is not None and worker_pid != process.pid:
            os.kill(worker_pid, signal.SIGTERM)
        if process.poll() is None:
            process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=10)
        assert process.stderr is not None
        stderr = process.stderr.read()
        assert not stderr, stderr
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as freed:
            freed.bind(("127.0.0.1", 17845))
        print(f"GPU worker pid={worker_pid}, launcher pid={process.pid} exit={process.returncode}; port bind released")
