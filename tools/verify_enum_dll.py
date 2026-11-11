#!/usr/bin/env python3
"""Load enum_platform_dll and verify ABI v10's sole JSON entry and paired platforms."""
import argparse
import ctypes
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('library', type=Path)
    args = parser.parse_args()
    library = ctypes.CDLL(str(args.library.resolve()))
    library.miresidian_abi_version.restype = ctypes.c_uint32
    assert library.miresidian_abi_version() == 10
    for removed in ('miresidian_invoke_with', 'miresidian_invoke_request'):
        assert getattr(library, removed, None) is None, removed
    invoke = library.miresidian_invoke
    invoke.argtypes = [ctypes.c_char_p]
    invoke.restype = ctypes.c_void_p

    def call(function, config=(), inputs=(), platform=None, code='OK'):
        request = {'function': function, 'config': list(config), 'input': list(inputs)}
        if platform is not None:
            request['platform'] = platform
        response = json.loads(ctypes.string_at(invoke(json.dumps(request).encode())))
        assert response['code'] == code, response
        return response['output']

    for platform in (None, 0, 1):
        assert call('GetInitConfig', platform=platform) == ['状态:Idle/Running']
    alternate = json.loads(call('GetInitConfig', platform=65538)[0])
    assert alternate == {'name': '状态', 'choices': ['Idle', 'Running'], 'inputPrefix': 'pick:'}
    assert call('GetInitConfig') == ['状态:Idle/Running']
    call('InitDevice', ['Idle', '平台适配:alternate_enum'], code='INVALID_ARGUMENT')
    call('InitDevice', ['Idle'], platform=1)
    assert call('Echo', ['Idle'], ['Running'], platform=0) == ['Running']
    call('Echo', ['Idle'], ['unknown'], code='INVALID_ARGUMENT')
    call('CloseAllDevices')
    call('InitDevice', ['pick:Idle'], platform=65538)
    assert call('Echo', ['pick:Idle'], ['pick:Running'], platform=65538) == ['Running']
    call('GetInitConfig', platform=0, code='INVALID_ARGUMENT')
    call('CloseAllDevices', platform=65538)
    for raw in (None, b'[]', b'{', b'\xff', b'{"function":"GetFunctions","platform":null}'):
        response = json.loads(ctypes.string_at(invoke(raw)))
        assert response['code'] == 'INVALID_ARGUMENT', response
    print('PASS: ABI v10 sole JSON export; removed symbols; default/explicit platforms; enum candidates and paired conversion')


if __name__ == '__main__':
    main()
