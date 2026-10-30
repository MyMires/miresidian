#!/usr/bin/env python3
"""Load the framework execution_dll example in a native host and prove overlap.

Build: cargo build --example execution_dll --features instances --locked
Run: python3 tools/verify_runtime_dll.py target/debug/examples/libexecution_dll.so
Windows accepts the corresponding x64/x86 execution_dll.dll in a matching host.
"""
import argparse
import ctypes
import json
import statistics
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('library', type=Path)
    args = parser.parse_args()
    library = ctypes.CDLL(str(args.library.resolve()))
    library.miresidian_abi_version.restype = ctypes.c_uint32
    invoke = library.miresidian_invoke
    invoke.argtypes = [ctypes.c_char_p] * 3
    invoke.restype = ctypes.c_void_p

    def call(operation, config=(), inputs=()):
        pointer = invoke(operation.encode(), json.dumps(list(config)).encode(), json.dumps(list(inputs)).encode())
        result = json.loads(ctypes.string_at(pointer))
        assert result['code'] == 'OK', result
        return result['output']

    for key in ('a', 'b'):
        call('InitDevice', [key, 'Sequential'])
    with ThreadPoolExecutor(max_workers=2) as workers:
        for operation in ('Pair', 'PairAsync'):
            outputs = list(workers.map(lambda key: call(operation, [key]), ('a', 'b')))
            assert len(outputs) == 2
    timings = {}
    with ThreadPoolExecutor(max_workers=2) as workers:
        for operation in ('Delay', 'DelayAsync'):
            for label, keys in [('same_instance', ('a', 'a')), ('different_instances', ('a', 'b'))]:
                samples = []
                for _ in range(5):
                    started = time.perf_counter()
                    list(workers.map(lambda key: call(operation, [key], ['30']), keys))
                    samples.append((time.perf_counter() - started) * 1000)
                timings[f'{operation}/{label}'] = round(statistics.median(samples), 2)
    print('observed median pair latency in ms:', json.dumps(timings))
    for operation in ('Record', 'RecordAsync'):
        key = f'broadcast-{operation}'
        call('InitDevice', [key, 'Broadcast'])
        assert call(operation, [key], ['共享', '10', '20', '', '30']) == ['1,10', '1,10', '', '1,10']
        assert call(operation, [key], ['共享', '', '20']) == ['', '']
        assert call(operation, [key], ['11']) == ['2', '11']
    print('broadcast channel 1 executes once; differing inputs and skipped destinations passed')
    call('CloseAllDevices')
    print(f'native DLL overlap passed; ABI {library.miresidian_abi_version()}')


if __name__ == '__main__':
    main()
