/* Private native host for examples/module_lifetime; not a public node ABI. */
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#ifdef _WIN32
#include <windows.h>
typedef HMODULE module_t;
typedef uint32_t (__cdecl *probe_t)(void);
static wchar_t library_path[32768];
static module_t open_module(void) { return LoadLibraryW(library_path); }
static module_t retain_existing(void) {
    HMODULE module = NULL;
    if (!GetModuleHandleExW(0, library_path, &module)) return NULL;
    return module;
}
static void close_module(module_t module) { if (!FreeLibrary(module)) abort(); }
static int is_loaded(void) { return GetModuleHandleW(library_path) != NULL; }
static probe_t symbol(module_t module, const char *name) { return (probe_t)GetProcAddress(module, name); }
static void delay_ms(unsigned milliseconds) { Sleep(milliseconds); }
struct call { probe_t function; uint32_t result; };
static DWORD WINAPI call_thread(LPVOID parameter) {
    struct call *call = parameter;
    call->result = call->function(); return 0;
}
static uint32_t isolated_call(probe_t function) {
    struct call call = {function, 1};
    HANDLE thread = CreateThread(NULL, 0, call_thread, &call, 0, NULL);
    if (!thread) abort();
    if (WaitForSingleObject(thread, 10000) != WAIT_OBJECT_0) abort();
    if (!CloseHandle(thread)) abort();
    return call.result;
}
#else
#include <dlfcn.h>
#include <pthread.h>
#include <time.h>
typedef void *module_t;
typedef uint32_t (*probe_t)(void);
static const char *library_path;
static module_t open_module(void) { return dlopen(library_path, RTLD_LAZY | RTLD_LOCAL); }
static module_t retain_existing(void) { return dlopen(library_path, RTLD_LAZY | RTLD_NOLOAD); }
static void close_module(module_t module) { if (dlclose(module) != 0) abort(); }
static int is_loaded(void) {
    module_t module = retain_existing();
    if (!module) return 0;
    close_module(module); return 1;
}
static probe_t symbol(module_t module, const char *name) { return (probe_t)dlsym(module, name); }
static void delay_ms(unsigned milliseconds) {
    struct timespec duration = {milliseconds / 1000, (long)(milliseconds % 1000) * 1000000};
    nanosleep(&duration, NULL);
}
struct call { probe_t function; uint32_t result; };
static void *call_thread(void *parameter) {
    struct call *call = parameter;
    call->result = call->function(); return NULL;
}
static uint32_t isolated_call(probe_t function) {
    struct call call = {function, 1}; pthread_t thread;
    if (pthread_create(&thread, NULL, call_thread, &call) != 0) abort();
    if (pthread_join(thread, NULL) != 0) abort();
    return call.result;
}
#endif
static void check(int condition, const char *message) {
    if (!condition) { fprintf(stderr, "%s\n", message); exit(1); }
}
int main(int argc, char **argv) {
    check(argc == 2, "usage: module_lifetime_host <absolute-library-path>");
#ifdef _WIN32
    check(MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, argv[1], -1, library_path,
        sizeof library_path / sizeof *library_path) > 0, "invalid library path");
#else
    library_path = argv[1];
#endif
    for (unsigned iteration = 0; iteration < 4; ++iteration) {
        module_t module = open_module(); check(module != NULL, "load failed");
        probe_t start = symbol(module, "module_probe_start");
        probe_t status = symbol(module, "module_probe_status");
        probe_t stop = symbol(module, "module_probe_stop");
        check(start && status && stop, "fixture probe symbol missing");
        check(status() == 0, "previous library mapping was not unloaded");
        check(isolated_call(start) == 0, "executor start failed");
        close_module(module); /* No host reference while background code runs. */
        check(is_loaded(), "executor failed to retain its module");
        delay_ms(iteration % 2 ? 20 : 250);
        module = retain_existing(); check(module != NULL, "idle executor lost module");
        if (iteration % 2 == 0) check(status() == 2, "background operation did not finish");
        /* Short-lived call threads release loader-owned TLS destructors too. */
        check(isolated_call(stop) == 0, "executor shutdown failed");
        check(status() == 2, "shutdown failed to join blocking work");
        close_module(module);
        check(!is_loaded(), "shutdown left a module reference or irreversible pin");
    }
    puts("module retention, shutdown, unload and reload: 4 cycles passed");
    return 0;
}
