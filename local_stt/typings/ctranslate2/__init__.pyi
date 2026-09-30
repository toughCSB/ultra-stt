"""The native CTranslate2 capability APIs used by the recognition probe."""
from typing import Literal

def get_cuda_device_count() -> int: ...
def get_supported_compute_types(
    device: Literal["cpu", "cuda"], device_index: int = 0
) -> set[str]: ...
