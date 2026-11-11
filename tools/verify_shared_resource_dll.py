#!/usr/bin/env python3
"""Validate renamed DLLs, distinct node types, per-node state and one exclusive resource.

Build both framework examples with --features lease,instance. Run with paths to
shared_resource_dll and other_resource_dll. Uses a real exclusive file lock, no hardware.
"""
import argparse
import ctypes
import json
import shutil
import tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path


def load(path):
    library = ctypes.CDLL(str(path.resolve()))
    library.miresidian_abi_version.restype = ctypes.c_uint32
    assert library.miresidian_abi_version() == 10, "此脚本需要 ABI v10 单 JSON 入口"
    invoke = library.miresidian_invoke
    invoke.argtypes = [ctypes.c_char_p]
    invoke.restype = ctypes.c_void_p

    def call(operation, config=(), inputs=()):
        pointer = invoke(json.dumps({"platform": 0, "function": operation, "config": list(config), "input": list(inputs)}).encode())
        response = json.loads(ctypes.string_at(pointer))
        assert response['code'] == 'OK', response
        return response['output']

    return library, call


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('first', type=Path)
    parser.add_argument('other', type=Path)
    args = parser.parse_args()
    # Windows PIN keeps loaded DLL files mapped until process exit; retain copied
    # library files in the system temporary directory for subsequent OS cleanup.
    directory = Path(tempfile.mkdtemp(prefix='miresidian-shared-dll-'))
    renamed = directory / ('renamed_node' + args.first.suffix)
    shutil.copy2(args.first, renamed)
    resource = directory / 'exclusive-port'
    resource.touch()
    path = str(resource)
    libraries, calls = zip(*(load(p) for p in (args.first, renamed, args.other)))
    first, renamed_call, other = calls
    for call in (first, renamed_call):
        assert call('InitDevice', [path, 'h1']) == []
    first('InitDevice', [path, 'h2'])
    other('InitDevice', [path])
    assert '复用' in first('InitDevice', [path, 'h1'])[0]
    with ThreadPoolExecutor(max_workers=3) as pool:
        jobs = [pool.submit(first, '查询', [path, 'h1'], ['one']),
                pool.submit(renamed_call, '查询', [path, 'h1'], ['two']),
                pool.submit(other, '查询', [path], ['three'])]
        assert [j.result() for j in jobs] == [['A:1:one'], ['A:1:two'], ['B:1:three']]
    first('CloseDevice', [path, 'h1'])
    assert first('查询', [path, 'h2'], ['next']) == ['A:1:next']
    first('CloseAllDevices')
    assert renamed_call('查询', [path, 'h1'], ['alive']) == ['A:2:alive']
    renamed_call('CloseAllDevices')
    assert other('查询', [path], ['last']) == ['B:2:last']
    other('CloseAllDevices')
    events = resource.with_suffix('.events').read_text().splitlines()
    assert events[0] == 'open' and events[-1] == 'close', events
    assert events.count('open') == 1 and events.count('close') == 1, events
    transactions = events[1:-1]
    assert len(transactions) == 12, transactions
    for begin, end in zip(transactions[::2], transactions[1::2]):
        assert begin.startswith('begin:') and end == 'end:' + begin[6:], transactions
    print('PASS: renamed DLLs and distinct node types, independent state, one open/close, six serialized transactions')
    assert len(libraries) == 3  # Keep all explicit loader references throughout verification.


if __name__ == '__main__':
    main()
