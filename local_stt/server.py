"""Launch with `python -m local_stt.server` from the app directory.

The native owner kills this process on cancellation; abandoning an inference thread
is not a GPU cancellation mechanism.
"""

from __future__ import annotations

import asyncio
import ctypes
import io
import json
import os
import socket
import sys
import wave
from contextlib import ExitStack
from pathlib import Path
from threading import Lock
from typing import Annotated, Protocol

import anyio
import numpy as np
import uvicorn
from fastapi import FastAPI, HTTPException, Request
from fastapi.responses import JSONResponse
from pydantic import BaseModel, ConfigDict, Field
from starlette.datastructures import UploadFile
from starlette.middleware.base import RequestResponseEndpoint
from starlette.responses import Response
from starlette.types import Message

MODEL_NAME = "Systran/faster-whisper-large-v3"
MAX_BODY = 25 * 1024 * 1024
MAX_PCM = 24 * 1024 * 1024
HOST = "127.0.0.1:17845"


class Bootstrap(BaseModel):
    model_config = ConfigDict(extra="forbid", strict=True)
    protocol: Annotated[int, Field(strict=True)]
    session_secret: Annotated[str, Field(min_length=32)]


class Recognizer(Protocol):
    def transcribe(self, samples: np.ndarray, language: str) -> str: ...


class WhisperRecognizer:
    def __init__(self, root: Path) -> None:
        model_path = root / "models" / "large-v3"
        for name in ("model.bin", "config.json", "tokenizer.json", "preprocessor_config.json"):
            if not (model_path / name).is_file():
                raise FileNotFoundError(model_path / name)

        # Same process-local Windows DLL setup as qa/check_recognition.py.
        self._dll_handles = ExitStack()
        site_packages = root / ".venv" / "Lib" / "site-packages"
        dll_dirs = [site_packages / "nvidia" / package / "bin"
                    for package in ("cublas", "cudnn", "cuda_nvrtc")]
        for directory in dll_dirs:
            if not directory.is_dir():
                raise FileNotFoundError(directory)
            self._dll_handles.enter_context(os.add_dll_directory(str(directory)))
        self._dll_modules = [ctypes.WinDLL(str(dll_dirs[0] / name))
                             for name in ("cublasLt64_12.dll", "cublas64_12.dll")]
        self._dll_modules.append(ctypes.WinDLL(str(dll_dirs[1] / "cudnn64_9.dll")))
        import ctranslate2
        from faster_whisper import WhisperModel

        if ctranslate2.get_cuda_device_count() < 1 or "float16" not in ctranslate2.get_supported_compute_types("cuda"):
            raise RuntimeError("CUDA device with float16 support is required")
        self._model = WhisperModel(str(model_path), device="cuda", compute_type="float16",
                                   cpu_threads=6, local_files_only=True)

    def transcribe(self, samples: np.ndarray, language: str) -> str:
        segments, _info = self._model.transcribe(samples, language=language, task="transcribe",
                                                 beam_size=5, vad_filter=True,
                                                 initial_prompt=None, prefix=None, hotwords=None)
        return "".join(segment.text for segment in segments)  # Lazy inference completes here.


def error(status: int, message: str) -> JSONResponse:
    return JSONResponse({"error": message}, status_code=status)


def decode_wav(raw: bytes) -> np.ndarray:
    try:
        with wave.open(io.BytesIO(raw), "rb") as wav:
            if (wav.getnchannels() != 1 or wav.getsampwidth() != 2 or
                    wav.getframerate() != 16000 or wav.getcomptype() != "NONE"):
                raise ValueError("Expected mono PCM16 16 kHz WAV")
            frames = wav.getnframes()
            if frames < 1 or frames * 2 > MAX_PCM:
                raise ValueError("PCM length out of range")
            pcm = wav.readframes(frames)
            if len(pcm) != frames * 2:
                raise ValueError("Truncated WAV")
    except (EOFError, wave.Error) as exc:
        raise ValueError("Invalid WAV") from exc
    return np.frombuffer(pcm, dtype="<i2").astype(np.float32) / 32768.0


def create_app(secret: str, recognizer: Recognizer, *, host: str = HOST) -> FastAPI:
    app = FastAPI(docs_url=None, redoc_url=None, openapi_url=None)
    inference = Lock()

    @app.middleware("http")
    async def guard(request: Request, call_next: RequestResponseEndpoint) -> Response:
        headers = request.headers
        if (request.method == "OPTIONS" or "origin" in headers or
                any(key.startswith("sec-fetch-") for key in headers)):
            return error(403, "Browser requests are forbidden")
        if headers.get("host") != host or headers.get("x-ultra-stt-session") != secret:
            return error(403, "Unauthorized local request")
        if request.method != "POST" or request.url.path != "/v1/audio/transcriptions":
            return error(404, "Unknown endpoint")
        return await call_next(request)

    @app.post("/v1/audio/transcriptions")
    async def transcription(request: Request) -> JSONResponse:
        if not inference.acquire(blocking=False):
            return error(429, "Inference busy")
        try:
            length = request.headers.get("content-length")
            if length is not None and (not length.isdecimal() or int(length) > MAX_BODY):
                return error(413, "Request body too large")
            chunks: list[bytes] = []
            size = 0
            async for chunk in request.stream():
                size += len(chunk)
                if size > MAX_BODY:
                    return error(413, "Request body too large")
                chunks.append(chunk)
            body = b"".join(chunks)
            sent = False

            async def receive() -> Message:
                nonlocal sent
                if sent:
                    return {"type": "http.disconnect"}
                sent = True
                return {"type": "http.request", "body": body, "more_body": False}

            bounded = Request(request.scope, receive)
            try:
                async with bounded.form(max_files=1, max_fields=2, max_part_size=MAX_BODY) as form:
                    file = form.get("file")
                    model = form.get("model")
                    language = form.get("language", "ko")
                    if not isinstance(file, UploadFile) or model != MODEL_NAME or language != "ko":
                        return error(422, "Expected file, full model, and Korean language")
                    raw = await file.read(MAX_BODY + 1)
            except ValueError:
                return error(400, "Invalid multipart body")
            if len(raw) > MAX_BODY:
                return error(413, "File too large")
            try:
                samples = decode_wav(raw)
            except ValueError as exc:
                return error(422, str(exc))
            try:
                # Shielding keeps the lock owned until the worker finishes; the native
                # process owner must terminate the helper for actual cancellation.
                text = await anyio.to_thread.run_sync(recognizer.transcribe, samples, "ko",
                                                      abandon_on_cancel=False)
            except Exception as exc:
                raise HTTPException(status_code=500, detail="Local inference failed") from exc
            return JSONResponse({"text": text})
        finally:
            inference.release()

    return app


class ReadyServer(uvicorn.Server):
    async def startup(self, sockets: list[socket.socket] | None = None) -> None:
        await super().startup(sockets=sockets)
        if self.started:
            print(json.dumps({"event": "ready", "protocol": 1, "pid": os.getpid(),
                              "endpoint": "http://127.0.0.1:17845/v1"}), flush=True)


def main() -> None:
    bootstrap = Bootstrap.model_validate_json(sys.stdin.readline())
    if bootstrap.protocol != 1:
        raise ValueError("Unsupported bootstrap protocol")
    root = Path(os.environ["ULTRA_STT_ROOT"])
    if not root.is_absolute():
        raise ValueError("ULTRA_STT_ROOT must be absolute")
    # Reserve the fixed address before expensive model initialization; no adoption
    # or termination of another listener is ever attempted.
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.bind(("127.0.0.1", 17845))
        listener.listen(128)
        recognizer = WhisperRecognizer(root)
        config = uvicorn.Config(create_app(bootstrap.session_secret, recognizer),
                                host="127.0.0.1", port=17845, access_log=False,
                                log_level="warning")
        asyncio.run(ReadyServer(config).serve(sockets=[listener]))


if __name__ == "__main__":
    main()
