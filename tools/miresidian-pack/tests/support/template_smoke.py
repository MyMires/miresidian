"""加载真实 registry 模板产物，先核对 ABI 再绑定对应函数签名。"""
import ctypes
import json
import sys

library = ctypes.CDLL(sys.argv[1])
library.miresidian_abi_version.argtypes = []
library.miresidian_abi_version.restype = ctypes.c_uint32
abi = library.miresidian_abi_version()
assert abi == int(sys.argv[2]), abi
invoke = library.miresidian_invoke
invoke.restype = ctypes.c_char_p  # ctypes 在下一次调用前将返回文本复制为 bytes。
invoke.argtypes = [ctypes.c_char_p] if abi == 10 else [ctypes.c_char_p] * 3


def call(function, inputs=()):
    if abi == 10:
        raw = invoke(json.dumps({"platform": 0, "function": function, "input": list(inputs)}).encode())
    else:
        raw = invoke(function.encode(), b"[]", json.dumps(list(inputs)).encode())
    payload = json.loads(raw)
    assert payload["status"] == 0, payload
    return payload["output"]


assert any(item.startswith("Echo(") for item in call("GetFunctions"))
call("GetInitConfig")
try:
    assert call("InitDevice") == []
    assert call("Echo", ["hello,中文"]) == ["hello,中文"]
finally:
    call("CloseAllDevices")
print(f"ABI v{abi}: query/init/echo/close passed")
