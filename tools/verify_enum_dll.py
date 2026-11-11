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
        assert call('GetInitConfig', platform=platform) == ['状态:Idle-空闲/Running-运行中']
    alternate = json.loads(call('GetInitConfig', platform=65538)[0])
    assert alternate == {'name': '状态', 'choices': [
        {'value': 'Idle', 'label': '空闲'},
        {'value': 'Running', 'label': '运行中'},
    ], 'inputPrefix': 'pick:'}
    assert call('GetInitConfig') == ['状态:Idle-空闲/Running-运行中']
    for platform in (None, 0, 1, 65538):
        assert call('GetPortNO', platform=platform) == ['0', '当前未实现端口发现能力']
    call('InitDevice', ['Idle', '平台适配:alternate_enum'], code='INVALID_ARGUMENT')
    call('InitDevice', ['空闲'], platform=1)
    assert call('Echo', ['Idle'], ['运行中'], platform=0) == ['Running']
    call('Echo', ['Idle'], ['unknown'], code='INVALID_ARGUMENT')
    call('CloseAllDevices')
    call('InitDevice', ['pick:空闲'], platform=65538)
    assert call('Echo', ['pick:Idle'], ['pick:运行中'], platform=65538) == ['Running']
    call('GetInitConfig', platform=0, code='INVALID_ARGUMENT')
    call('CloseAllDevices', platform=65538)
    for raw in (None, b'[]', b'{', b'\xff', b'{"function":"GetFunctions","platform":null}'):
        response = json.loads(ctypes.string_at(invoke(raw)))
        assert response['code'] == 'INVALID_ARGUMENT', response
    print('PASS: ABI v10 sole JSON export; default/explicit platforms; derived labels; unimplemented port discovery reason')


if __name__ == '__main__':
    main()
