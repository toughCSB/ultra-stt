"""Typed subset of faster-whisper 1.2.1 used by the recognition probe."""
from collections.abc import Iterable

import numpy as np

class Segment:
    text: str

class TranscriptionInfo:
    language: str
    duration: float

class WhisperModel:
    def __init__(
        self,
        model_size_or_path: str,
        *,
        device: str,
        compute_type: str,
        cpu_threads: int,
        local_files_only: bool,
    ) -> None: ...
    def transcribe(
        self,
        audio: str | np.ndarray,
        *,
        language: str,
        task: str,
        beam_size: int,
        vad_filter: bool,
        condition_on_previous_text: bool = ...,
        initial_prompt: str | None,
        prefix: str | None,
        hotwords: str | None,
    ) -> tuple[Iterable[Segment], TranscriptionInfo]: ...
