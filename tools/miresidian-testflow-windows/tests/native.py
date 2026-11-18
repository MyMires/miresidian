#!/usr/bin/env python3
"""Internal DLL integration checks; independent nodes use an exact registry framework.

Requires cargo and PowerShell. Builds only temporary fixtures. Linux native loading
checks the embedded runner; Windows additionally executes its generated BAT.
"""
import argparse
import ctypes
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import uuid
import time
import queue
import threading
from concurrent.futures import ThreadPoolExecutor

ROOT = Path(__file__).resolve().parents[3]
TOOL = ROOT / 'tools/miresidian-testflow-windows'
VERSION = '0.34.2'


def run(command, **kwargs):
    return subprocess.run([str(x) for x in command], check=True, timeout=kwargs.pop('timeout', 60), **kwargs)



def verify_events(events, opened, closed, transactions):
    assert events[0] == opened and events[-2:] == [closed, 'drop'], events
    assert events.count(opened) == events.count(closed) == events.count('drop') == 1, events
    body=events[1:-2]
    assert len(body) == transactions*2, events
    for start,end in zip(body[::2],body[1::2]):
        assert start.startswith('begin:') and end == 'end:'+start[6:],events


def verify_validator():
    good=['open','begin:x','end:x','close','drop']
    verify_events(good,'open','close',1)
    for bad,count in ((['open','begin:x','end:x','drop','close'],1),(good,2),(['open','begin:x','end:y','close','drop'],1)):
        try: verify_events(bad,'open','close',count)
        except AssertionError: continue
        raise AssertionError('event verifier accepted deliberately wrong count/order')


def build(directory):
    fixtures = {'execution': 'execution_dll.rs', 'leased': 'leased_resource_dll.rs',
                'other': 'leased_other_dll.rs', 'shared': 'shared_resource_dll.rs',
                'other_shared': 'other_resource_dll.rs', 'policy':'policy', 'policy_other':'policy', 'policy_incompatible':'incompatible'}
    members = list(fixtures)
    directory.mkdir(parents=True, exist_ok=True)
    (directory / 'Cargo.toml').write_text('[workspace]\nresolver="3"\nmembers=[]\n')
    for name, source in fixtures.items():
        package = directory / name
        (package / 'src/support').mkdir(parents=True, exist_ok=True)
        if source in ('policy', 'incompatible'):
            code = (TOOL / 'tests/support/policy_node.rs').read_text()
            if source == 'incompatible':
                code += '\nimpl PolicyNode { #[miresidian::operation] fn extra(&mut self) -> MiresidianResult<()> { Ok(()) } }\n'
        else:
            code = (ROOT / 'examples' / source).read_text()
        (package / 'src/lib.rs').write_text(code)
        for support in ('leased_node.rs', 'shared_resource.rs'):
            content=(ROOT / 'examples/support' / support).read_text()
            if support == 'shared_resource.rs':
                content=content.replace('                events,\n            })','                events,\n                marker: path.to_path_buf(),\n            })')
                content=content.replace('struct ExclusiveResource {','struct ExclusiveResource {\n    marker: std::path::PathBuf,')
                entry='        std::thread::sleep(Duration::from_millis(10));'
                gate="""        if request == b"hold" {
            std::fs::write(self.marker.with_extension("entered"), b"entered").map_err(io_error)?;
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            while !self.marker.with_extension("release").exists() {
                if std::time::Instant::now() > deadline { return Err(LeaseError::LocalIpc("test barrier deadline".into())); }
                std::thread::sleep(Duration::from_millis(1));
            }
        }
"""
                content=content.replace(entry,gate+entry)
                content=content.replace('        Ok(request.to_vec())','        if request == b"hold" { std::fs::write(self.marker.with_extension("completed"), b"completed").map_err(io_error)?; }\n        Ok(request.to_vec())')
            (package / 'src/support' / support).write_text(content)
        (package / 'Cargo.toml').write_text(f'''[package]
name="testflow_{name}"
version="0.1.0"
edition="2024"
[lib]
crate-type=["cdylib"]
[dependencies]
miresidian={{version="={VERSION}",features=["instance","lease"]}}
tokio={{version="~1.53.3",features=["time"]}}
[workspace]
''')
    for name in members:
        run(['cargo', 'build', '--manifest-path', directory / name / 'Cargo.toml', '--target-dir',directory / 'target'], timeout=300)
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--format-version', '1', '--manifest-path', str(directory / 'execution/Cargo.toml')]))
    framework = next(p for p in metadata['packages'] if p['name'] == 'miresidian')
    assert framework['version'] == VERSION and framework['source'].startswith('registry+'), framework
    suffix = '.dll' if os.name == 'nt' else '.so'
    prefix = '' if os.name == 'nt' else 'lib'
    return {n: directory / 'target/debug' / f'{prefix}testflow_{n}{suffix}' for n in members}


def load(path):
    dll = ctypes.CDLL(str(path.resolve()))
    dll.miresidian_abi_version.restype = ctypes.c_uint32
    assert dll.miresidian_abi_version() == 10
    invoke = dll.miresidian_invoke
    invoke.argtypes = [ctypes.c_char_p]
    invoke.restype = ctypes.c_void_p
    def call(method, config=(), inputs=(), code='OK'):
        request = dict(platform=1, function=method, config=list(config), input=list(inputs))
        response = json.loads(ctypes.string_at(invoke(json.dumps(request).encode())))
        assert response['code'] == code, response
        return response['output']
    return dll, call



class Host:
    """Bounded ready/response handshakes; an explicit kill is always reaped."""
    def __init__(self, dll, config):
        self.process = subprocess.Popen([sys.executable, __file__, '--host', str(dll), json.dumps(config)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        self.killed = False
        self.messages = queue.Queue()
        def read():
            for line in self.process.stdout:
                self.messages.put(line.rstrip())
            self.messages.put(None)
        threading.Thread(target=read, daemon=True).start()
        try: assert self.messages.get(timeout=10) == 'ready'
        except BaseException:
            self.kill(); raise
    def kill(self):
        self.killed = True
        self.process.kill()
        self.process.wait(timeout=10)
    def close(self, invalid=False):
        if self.process.poll() is None:
            self.process.stdin.write('close-invalid\n' if invalid else 'close\n'); self.process.stdin.flush()
            try: self.process.wait(timeout=10)
            except subprocess.TimeoutExpired: self.kill(); raise
        assert self.process.poll() == 0 or self.killed, self.process.stderr.read()


def host_main():
    module, call = load(Path(sys.argv[2]))
    call('InitDevice', json.loads(sys.argv[3]), ['token'])
    print('ready', flush=True)
    for line in sys.stdin:
        if line.strip() in ('close','close-invalid'):
            call('CloseDevice', json.loads(sys.argv[3]), ['reset'], code='LEASE_INVALIDATED' if line.strip() == 'close-invalid' else 'OK')
            return
    assert module


def await_file(path):
    deadline = time.monotonic() + 10
    while not path.exists():
        if time.monotonic() > deadline: raise TimeoutError(f'No entered handshake: {path}')
        time.sleep(0.005)  # Poll an explicit event; elapsed time is never concurrency evidence.


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--powershell', default='powershell.exe' if os.name == 'nt' else 'pwsh')
    parser.add_argument('--work', type=Path)
    options = parser.parse_args()
    directory = options.work or Path(tempfile.mkdtemp(prefix='testflow-native-'))
    verify_validator()
    dlls = build(directory)
    projects = directory
    directory = directory / ('run-' + uuid.uuid4().hex)
    directory.mkdir()
    os.environ['MIRESIDIAN_LEASE_SERVICE_NAMESPACE'] = 'testflow-' + uuid.uuid4().hex
    run([options.powershell, '-NoProfile', '-File', TOOL / 'tests/runner-tests.ps1'])
    generator = ROOT / 'target/debug/cargo-miresidian-testflow-windows'
    if os.name == 'nt': generator = generator.with_suffix('.exe')
    run(['cargo', 'build', '-p', 'miresidian_testflow_windows'], cwd=ROOT, timeout=120)

    def flow(name, text, dll='execution', success=True, observer=None):
        path = directory / dll / f'{name}.testflow'
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding='utf-8')
        # Every actual flow passes the Rust generator's check too.
        run([generator, '--check', path], cwd=projects / dll)
        run([generator,path],cwd=projects / dll)
        assert all(len(line)<8191 for line in path.with_suffix('.bat').read_text().splitlines())
        env = dict(os.environ, MIRESIDIAN_DLL_PATH=str(dlls[dll]), MIRESIDIAN_FLOW_FILE=str(path), MIRESIDIAN_TESTFLOW_NO_PAUSE='1')
        if os.name == 'nt':
            # BAT resolves ../dist by design; provide its normal architecture layout.
            architecture = 'windows-x64' if ctypes.sizeof(ctypes.c_void_p) == 8 else 'windows-x86'
            distribution = directory / 'dist' / architecture
            distribution.mkdir(parents=True, exist_ok=True)
            shutil.copy2(dlls[dll], distribution / dlls[dll].name)
            command = ['cmd.exe', '/c', str(path.with_suffix('.bat'))]
        else:
            command = [options.powershell, '-NoProfile', '-File', str(TOOL / 'src/runner.ps1')]
        if observer:
            # Test-only second caller; production parsing/dispatch functions stay intact.
            runner=(TOOL / 'src/runner.ps1').read_text()
            runner=runner.replace('function Invoke-NodeRequest(', 'function Invoke-OriginalNodeRequest(')
            extra=(TOOL / 'tests/support/concurrent_host.cs').read_text()
            hooks="\n$source += @'\n" + extra + "\n'@\n" + """
function Invoke-NodeRequest([string]$Function,[string]$ConfigJson,[string]$InputJson) {
    $response=Invoke-OriginalNodeRequest $Function $ConfigJson $InputJson
    if ($Function -ceq 'Record' -and $InputJson.Contains('共享')) { [TestFlowProbe]::Join() }
    return $response
}
"""
            location=runner.index('\ntry {\n    Add-Type')
            runner=runner[:location]+hooks+runner[location:]
            quote=lambda text: "'"+str(text).replace("'","''")+"'"
            args=[observer['resource'].with_suffix('.entered'),observer['resource'].with_suffix('.attempted'),observer['resource'].with_suffix('.response'),json.dumps(dict(platform=1,function='Record',config=observer['config'],input=['other']),ensure_ascii=False)]
            runner=runner.replace('    Add-Type -TypeDefinition $source -ErrorAction Stop', '    Add-Type -TypeDefinition $source -ErrorAction Stop\n    [TestFlowProbe]::Start('+','.join(map(quote,args))+')')
            script=directory / (name+'-host.ps1'); script.write_text(runner,encoding='utf-8-sig')
            command=[options.powershell,'-NoProfile','-ExecutionPolicy','Bypass','-File',str(script)]
        result = subprocess.run(command, env=env, capture_output=True, text=True, timeout=60)
        (directory / f'{name}.log').write_text(result.stdout + result.stderr)
        assert (result.returncode == 0) == success, (name, result.stdout, result.stderr)
        responses = [json.loads(line.split('原始响应：', 1)[1]) for line in result.stdout.splitlines() if line.startswith('原始响应：')]
        assert responses, (name, result.stdout, result.stderr)
        return responses

    for mode in ('Sequential', 'Parallel', 'Broadcast'):
        text = f'Config("A","{mode}")\nInitDevice()\nRecord({{"ch1":["7"],"ch3":["7"],"ch6":null}}) => {{"code":"OK"}}\nRecord("9") => {{"code":"OK","output":["{2 if mode == "Broadcast" else 3}","9"]}}'
        responses = flow(mode, text)
        batch = next(r['output'] for r in responses if len(r['output']) == 6)
        assert batch[1] == batch[3] == batch[4] == batch[5] == '', batch
        assert sorted([batch[0], batch[2]]) == (['1,7', '1,7'] if mode == 'Broadcast' else ['1,7', '2,7']), batch
    flow('broadcast-skipped-source', 'Config("A","Broadcast")\nInitDevice()\nRecord({"ch1":null,"ch3":["7"]}) => {"status":0,"output":{"ch1-ch3":""}}\nRecord("9") => {"code":"OK","output":["1","9"]}')
    flow('different-config', 'Config({"ch1,ch3":["A","Sequential"],"ch2":["B","Sequential"]})\nInitDevice()\nRecord({"ch1-ch3":["7"]}) => {"code":"OK","output":{"ch1,ch2":["1","7"],"ch3":["2","7"]}}\nRecord("9") => {"code":"OK","output":["3","9"]}\nCloseDevice()')
    flow('expected-error', 'Config("A","Sequential")\nInitDevice()\nRecord("bad") => {"code":"INVALID_ARGUMENT"}\nRecord("9") => {"code":"OK","output":["1","9"]}')
    flow('unexpected-error', 'Config("A","Sequential")\nInitDevice()\nRecord("bad")', success=False)
    flow('code-only-actual-output', 'Config("A","Sequential")\nInitDevice()\nRecord("7") => {"code":"OK"}')
    flow('batch-arity-skip', 'Config("A","Sequential")\nInitDevice()\nRecord({"ch1":["7","extra"],"ch2":["8"]}) => {"code":"OK","output":{"ch1":"","ch2":["1","8"]}}')

    # Keep another host's lease alive while TestFlow initializes, calls and finally cleans up.
    # Returned counters + exclusive-file events prove it did not open a second connection.
    resource = directory / 'leased-port'; resource.touch()
    module, other = load(dlls['other'])
    cfg = [str(resource), '1', 'other-host', 'Lease']
    other('InitDevice', cfg)
    config = json.dumps([str(resource), '1', 'flow-host', 'Lease'], ensure_ascii=False)[1:-1]
    flow('lease-through-testflow', f'Config({config})\nInitDevice()\n查询({{"ch1,ch3":["flow"]}}) => {{"code":"OK","output":{{"ch1":"1:flow","ch3":"2:flow"}}}}', 'leased')
    assert other('查询', cfg, ['after-flow']) == ['3:after-flow']
    conflict = json.dumps([str(resource), '2', 'conflict', 'Lease'])[1:-1]
    flow('lease-conflict', f'Config({conflict})\nInitDevice() => {{"code":"OPERATION_FAILED"}}', 'leased')
    assert other('查询', cfg, ['still-alive']) == ['4:still-alive']
    other('CloseAllDevices')
    events = Path(str(resource) + '.events').read_text().splitlines()
    assert events.count('open:1') == events.count('close') == events.count('drop') == 1, events
    cfg[1]='2'; other('InitDevice', cfg); other('CloseAllDevices')
    # Existing original/renamed/independent DLL baselines remain unchanged.
    run([sys.executable, ROOT / 'tools/verify_leased_node_dll.py', dlls['leased'], dlls['other']])
    run([sys.executable, ROOT / 'tools/verify_shared_resource_dll.py', dlls['shared'], dlls['other_shared']])
    assert module
    policy_module, policy = load(dlls['policy_other'])
    incompatible_module, incompatible = load(dlls['policy_incompatible'])
    definitions = policy('GetInitConfig')
    # These are generated by the published framework, not inferred from Handle position.
    assert len(definitions) == 5, definitions
    for activation in ('Direct','Lease'):
        for mode in ('Sequential','Parallel','Broadcast'):
            resource = directory / f'policy-{activation}-{mode}'; resource.touch()
            cfg = [str(resource), '1', 'h1', mode, activation]
            # Resolve framework-generated selection positions by their actual metadata.
            cfg = [str(resource), '1'] + [('h1' if d.startswith('Handle') else mode if '执行' in d else activation) for d in definitions[2:]]
            config = json.dumps(cfg,ensure_ascii=False)[1:-1]
            after = 2 if mode == 'Broadcast' else 3
            responses = flow(f'policy-{activation}-{mode}', f'Config({config})\nInitDevice("token") => {{"code":"OK","output":["token"]}}\nRecord({{"ch1":["x"],"ch3":["x"],"ch6":null}}) => {{"code":"OK"}}\nRecord("after") => {{"code":"OK","output":["{after}","after"]}}\nCloseDevice("reset") => {{"code":"OK"}}', 'policy')
            batch = next(r['output'] for r in responses if len(r['output']) == 6)
            assert sorted([batch[0],batch[2]]) == (['1,x','1,x'] if mode == 'Broadcast' else ['1,x','2,x']), batch
            events = Path(str(resource)+'.events').read_text().splitlines()
            verify_events(events,'open:1:token','close:reset',2 if mode == 'Broadcast' else 3)
            assert events.count('begin:x') == (1 if mode == 'Broadcast' else 2), events
    resource = directory / 'policy-shared'; resource.touch()
    cfg=[str(resource),'1'] + [('owner' if d.startswith('Handle') else 'Sequential' if '执行' in d else 'Lease') for d in definitions[2:]]
    policy('InitDevice',cfg,['token'])
    proxy=cfg.copy(); proxy[2]='flow'
    config=json.dumps(proxy,ensure_ascii=False)[1:-1]
    flow('init-input-conflict', f'Config({config})\nInitDevice("different") => {{"code":"INVALID_ARGUMENT"}}', 'policy')
    incompatible('InitDevice', proxy, ['token'], code='INVALID_ARGUMENT')
    assert policy('Record',cfg,['alive']) == ['1','alive']
    flow('close-nonlast-input', f'Config({config})\nInitDevice("token")\nCloseDevice("not-last")', 'policy')
    assert policy('Record',cfg,['still-alive']) == ['2','still-alive']
    policy('CloseDevice',cfg,['last-close'])
    events=Path(str(resource)+'.events').read_text().splitlines()
    assert 'close:not-last' not in events and events[-2:] == ['close:last-close','drop'], events
    for failure in ('fail','panic'):
        resource=directory / f'policy-init-{failure}'; resource.touch()
        cfg[0]=str(resource); config=json.dumps(cfg,ensure_ascii=False)[1:-1]
        # A failed initializer must release its physical file and configuration lease.
        failure_status = 3 if failure == 'panic' else 2
        flow(f'init-{failure}', f'Config({config})\nInitDevice("{failure}") => {{"status":{failure_status}}}\nInitDevice("recovered") => {{"code":"OK"}}\nCloseDevice("reset")', 'policy')
        events=Path(str(resource)+'.events').read_text().splitlines()
        assert events.count('drop') == 2 and events[-2:] == ['close:reset','drop'],events
    for failure in ('fail','panic'):
        resource=directory / f'policy-close-{failure}'; resource.touch()
        cfg[0]=str(resource); config=json.dumps(cfg,ensure_ascii=False)[1:-1]
        failure_status=3 if failure == 'panic' else 2
        flow(f'close-{failure}', f'Config({config})\nInitDevice("token")\nCloseDevice("{failure}") => {{"status":{failure_status}}}', 'policy')
        events=Path(str(resource)+'.events').read_text().splitlines()
        assert events[-2:] == [f'close:{failure}','drop'],events
        cfg[1]='2'; policy('InitDevice',cfg,['reopened']); policy('CloseDevice',cfg,['reset']); cfg[1]='1'
    resource=directory / 'policy-cleanup-failure'; resource.touch(); cfg[0]=str(resource)
    config=json.dumps(cfg,ensure_ascii=False)[1:-1]
    flow('cleanup-error-visible', f'Config({config})\nInitDevice("token")', 'policy', success=False)
    events=Path(str(resource)+'.events').read_text().splitlines()
    assert events[-1]=='drop',events
    for mode in ('Sequential','Parallel','Broadcast'):
        resource=directory / f'policy-business-error-{mode}'; resource.touch(); cfg[0]=str(resource)
        cfg=[str(resource),'1'] + [('h1' if d.startswith('Handle') else mode if '执行' in d else 'Lease') for d in definitions[2:]]
        config=json.dumps(cfg,ensure_ascii=False)[1:-1]
        after=4 if mode == 'Parallel' else 2
        flow(f'batch-failure-{mode}', f'Config({config})\nInitDevice("token")\nRecord({{"ch1":["fail"],"ch2":["x"],"ch3":["fail"]}}) => {{"code":"OPERATION_FAILED"}}\nRecord("after") => {{"code":"OK","output":["{after}","after"]}}\nCloseDevice("reset")', 'policy')
        events=Path(str(resource)+'.events').read_text().splitlines()
        assert events.count('begin:fail')==(2 if mode == 'Parallel' else 1),events
        assert events.count('begin:x')==(1 if mode == 'Parallel' else 0),events
    # The physical owner dies while the real TestFlow request is in flight.
    # The already-running parent's service stays alive, separating the two failures.
    resource=directory / 'owner-death'; resource.touch()
    cfg=[str(resource),'1'] + [('owner' if d.startswith('Handle') else 'Sequential' if '执行' in d else 'Lease') for d in definitions[2:]]
    owner=Host(dlls['policy_other'],cfg)
    proxy=cfg.copy(); proxy[2]='flow'; config=json.dumps(proxy,ensure_ascii=False)[1:-1]
    try:
        with ThreadPoolExecutor(max_workers=1) as pool:
            future=pool.submit(flow,'owner-dies-inflight',f'Config({config})\nInitDevice("token")\nRecord("hold") => {{"code":"LEASE_INVALIDATED"}}\nRecord("after") => {{"code":"LEASE_INVALIDATED"}}\nCloseDevice("reset") => {{"code":"OK"}}','policy')
            await_file(resource.with_suffix('.entered'))
            owner.kill()
            future.result(timeout=60)
        events=Path(str(resource)+'.events').read_text().splitlines()
        assert events == ['open:1:token','begin:hold'], events  # killed, no claim of finally/drop
        policy('InitDevice',cfg,['token'])
        assert policy('Record',cfg,['recovered']) == ['1','recovered']
        policy('CloseDevice',cfg,['reset'])
        events=Path(str(resource)+'.events').read_text().splitlines()
        assert events.count('begin:hold') == 1 and events.count('open:1:token') == 2, events
    finally:
        owner.close()
    # Separate service-host death from physical-owner death: four hosts at most.
    namespace=os.environ['MIRESIDIAN_LEASE_SERVICE_NAMESPACE']
    os.environ['MIRESIDIAN_LEASE_SERVICE_NAMESPACE']='testflow-service-' + uuid.uuid4().hex
    service_resource=directory / 'service-coordinator'; service_resource.touch()
    service_cfg=cfg.copy(); service_cfg[0]=str(service_resource)
    service=Host(dlls['policy_other'],service_cfg)
    resource=directory / 'service-death'; resource.touch(); cfg[0]=str(resource)
    owner=Host(dlls['policy_other'],cfg)
    proxy=cfg.copy(); proxy[2]='flow'; config=json.dumps(proxy,ensure_ascii=False)[1:-1]
    try:
        with ThreadPoolExecutor(max_workers=1) as pool:
            future=pool.submit(flow,'service-dies-owner-alive',f'Config({config})\nInitDevice("token")\nRecord("hold") => {{"code":"OK","output":["1","hold"]}}\nRecord("after") => {{"code":"LEASE_INVALIDATED"}}\nRecord("retry") => {{"code":"LEASE_INVALIDATED"}}\nCloseDevice("reset") => {{"code":"LEASE_INVALIDATED"}}','policy')
            await_file(resource.with_suffix('.entered'))
            service.kill()
            resource.with_suffix('.release').write_text('release')
            future.result(timeout=60)
        events=Path(str(resource)+'.events').read_text().splitlines()
        assert events == ['open:1:token','begin:hold','end:hold'],events
    finally:
        owner.close(invalid=True); service.close()
        os.environ['MIRESIDIAN_LEASE_SERVICE_NAMESPACE']=namespace
    for activation in ('Direct','Lease'):
        for mode in ('Sequential','Parallel','Broadcast'):
            resource=directory / f'concurrent-{activation}-{mode}'; resource.touch()
            cfg=[str(resource),'1'] + [('h1' if d.startswith('Handle') else mode if '执行' in d else activation) for d in definitions[2:]]
            config=json.dumps(cfg,ensure_ascii=False)[1:-1]
            with ThreadPoolExecutor(max_workers=1) as pool:
                future=pool.submit(flow,f'concurrent-{activation}-{mode}',f'Config({config})\nInitDevice("token")\nRecord({{"ch1":["hold"],"ch2":["x"]}}) => {{"code":"OK"}}\nCloseDevice("reset")','policy',True,dict(resource=resource,config=cfg))
                await_file(resource.with_suffix('.entered'))
                await_file(resource.with_suffix('.attempted'))
                resource.with_suffix('.release').write_text('release')
                future.result(timeout=60)
            result=json.loads(resource.with_suffix('.response').read_text())
            assert result['code']=='OK' and result['output']==[('2' if mode=='Broadcast' else '3'),'other'],result
            events=Path(str(resource)+'.events').read_text().splitlines()
            verify_events(events,'open:1:token','close:reset',2 if mode=='Broadcast' else 3)
            assert events[-4:-2]==['begin:other','end:other'],events
    for activation in ('Direct','Lease'):
        resource=directory / f'independent-{activation}'; resource.touch()
        second=directory / f'independent-second-{activation}'; second.touch()
        cfg=[str(resource),'1'] + [('h1' if d.startswith('Handle') else 'Parallel' if '执行' in d else activation) for d in definitions[2:]]
        cfg2=[str(second),'1'] + [('h2' if d.startswith('Handle') else 'Parallel' if '执行' in d else activation) for d in definitions[2:]]
        mapping=json.dumps({'ch1,ch3':cfg,'ch2':cfg2},ensure_ascii=False)
        with ThreadPoolExecutor(max_workers=1) as pool:
            future=pool.submit(flow,f'independent-{activation}',f'Config({mapping})\nInitDevice("token")\nRecord({{"ch1":["hold"],"ch3":["x"]}}) => {{"code":"OK"}}\nCloseDevice("reset")','policy',True,dict(resource=resource,config=cfg2))
            await_file(resource.with_suffix('.entered'))
            try: await_file(resource.with_suffix('.response'))
            finally: resource.with_suffix('.release').write_text('release')
            future.result(timeout=60)
        result=json.loads(resource.with_suffix('.response').read_text())
        assert result['code']=='OK' and result['output']==['1','other'],result
        verify_events(Path(str(second)+'.events').read_text().splitlines(),'open:1:token','close:reset',1)
    shared_module, shared_owner=load(dlls['other_shared'])
    resource=directory / 'communication-timeout'; resource.touch()
    shared_owner('InitDevice',[str(resource)])
    config=json.dumps([str(resource),'flow'],ensure_ascii=False)[1:-1]
    try:
        with ThreadPoolExecutor(max_workers=1) as pool:
            future=pool.submit(flow,'communication-timeout',f'Config({config})\nInitDevice()\n查询("hold") => {{"code":"OPERATION_FAILED"}}\n查询("retry") => {{"code":"OPERATION_FAILED"}}\nCloseDevice() => {{"code":"OK"}}','shared')
            await_file(resource.with_suffix('.entered'))
            try: future.result(timeout=60)  # Existing fixture transaction budget is 2s.
            finally: resource.with_suffix('.release').write_text('release')
        await_file(resource.with_suffix('.completed'))
        assert shared_owner('查询',[str(resource)],['after']) == ['B:1:after']
    finally:
        resource.with_suffix('.release').write_text('release')
        shared_owner('CloseAllDevices')
    events=resource.with_suffix('.events').read_text().splitlines()
    assert events == ['open','begin:hold','end:hold','begin:after','end:after','close'],events
    assert shared_module
    assert policy_module and incompatible_module
    print('PASS: native TestFlow multichannel strategies, independent Config, assertions, cleanup, cross-host leases; artifacts:', directory)


if __name__ == '__main__':
    if len(sys.argv) > 1 and sys.argv[1] == '--host': host_main()
    else: main()
