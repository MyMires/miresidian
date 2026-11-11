#!/usr/bin/env python3
"""Exercise automatic lease ownership using unchanged node macros and direct open.

Run against leased_resource_dll and leased_other_dll. Both initialize a real
exclusive file handle. No SharedResource API is used by the fixture nodes.
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

    def call(operation, config=(), inputs=(), code='OK'):
        response = json.loads(ctypes.string_at(invoke(json.dumps({"platform": 0, "function": operation, "config": list(config), "input": list(inputs)}).encode())))
        assert response['code'] == code, response
        return response['output']
    return library, call


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('first', type=Path)
    parser.add_argument('second', type=Path)
    args = parser.parse_args()
    # The runtime intentionally retains the owner module while callbacks exist.
    directory = Path(tempfile.mkdtemp(prefix='miresidian-lease-node-dll-'))
    renamed = directory / ('renamed' + args.first.suffix)
    shutil.copy2(args.first, renamed)
    resource = directory / 'exclusive-port'
    resource.touch()
    config = lambda handle, setting='1': [str(resource), setting, handle, 'Lease']
    libraries, calls = zip(*(load(path) for path in (args.first, renamed, args.second)))
    first, copied, second = calls
    # Failures here reproduce the original second-open bug before the fix.
    with ThreadPoolExecutor(max_workers=3) as pool:
        jobs = [pool.submit(call, 'InitDevice', config('h1')) for call in calls]
        assert all(job.result() == ['connected:1'] for job in jobs)
    assert '复用' in first('InitDevice', config('h1'))[0]
    first('InitDevice', config('h2'))
    with ThreadPoolExecutor(max_workers=3) as pool:
        jobs = [pool.submit(call, '查询', config('h1'), [f'message-{i}']) for i, call in enumerate(calls)]
        outputs = [job.result()[0] for job in jobs]
    assert sorted(int(output.split(':')[0]) for output in outputs) == [1, 2, 3], outputs
    second('参数错误', config('h1'), code='INVALID_ARGUMENT')
    assert second('异步查询', config('h1')) == ['4']
    copied('InitDevice', config('conflict', '2'), code='OPERATION_FAILED')
    first('CloseDevice', config('h1'))
    first('CloseAllDevices')
    assert copied('查询', config('h1'), ['after-owner-close']) == ['5:after-owner-close']
    copied('CloseAllDevices')
    assert second('查询', config('h1'), ['last-user']) == ['6:last-user']
    second('CloseAllDevices')
    events = Path(str(resource) + '.events').read_text().splitlines()
    assert events.count('open:1') == 1 and events[-2:] == ['close', 'drop'], events
    assert events.count('close') == events.count('drop') == 1, events
    for begin, end in zip(events[1:-2:2], events[2:-2:2]):
        assert begin.startswith('begin:') and end == 'end:' + begin[6:], events
    # A new setting after final close proves both connection and lease are released.
    second('InitDevice', config('reopened', '2'))
    assert second('查询', config('reopened', '2'), ['fresh']) == ['1:fresh']
    second('CloseAllDevices')
    events = Path(str(resource) + '.events').read_text().splitlines()
    assert events.count('open:2') == 1 and events.count('close') == 2 and events.count('drop') == 2, events
    assert len(libraries) == 3
    print('PASS: original/renamed/independently compiled DLLs; concurrent init, one open, shared calls, serialized transactions, last close, reopen, async and typed errors')


if __name__ == '__main__':
    main()
