"""Miresidian C ABI v5 的共享 Python 调用代码。"""

from __future__ import annotations

import ctypes
import json
from pathlib import Path


def invoke(
    dll_path: Path,
    function: str,
    config: list[str],
    input_values: list[str],
) -> str:
    """调用 DLL，并返回已复制的 UTF-8 JSON 文本 result_json。"""
    if not dll_path.is_file():
        raise RuntimeError(f"找不到 DLL：{dll_path}")
    if not function:
        raise ValueError("function 不能为空")
    if not all(isinstance(value, str) for value in config):
        raise ValueError("config 必须是字符串数组")
    if not all(isinstance(value, str) for value in input_values):
        raise ValueError("input 必须是字符串数组")

    try:
        library = ctypes.CDLL(str(dll_path))
        library.miresidian_invoke.argtypes = [
            ctypes.c_char_p,
            ctypes.c_char_p,
            ctypes.c_char_p,
        ]
        library.miresidian_invoke.restype = ctypes.c_char_p
    except (AttributeError, OSError) as error:
        raise RuntimeError(f"无法加载 Miresidian C ABI：{error}") from error

    config_json = json.dumps(config, ensure_ascii=False).encode("utf-8")
    input_json = json.dumps(input_values, ensure_ascii=False).encode("utf-8")
    result = library.miresidian_invoke(function.encode("utf-8"), config_json, input_json)
    if result is None:
        raise RuntimeError("miresidian_invoke 返回了空指针")

    # c_char_p 在函数返回时会复制 DLL 持有的文本，满足 result_json 的生命周期约束。
    return result.decode("utf-8")
