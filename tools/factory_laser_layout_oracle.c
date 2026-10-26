/* Synthetic copy-layout probe for one hashed reference DLL. No device data.
 * Enter only the straight-line serializer copy region, ending before calls.
 * Run in the private no-device/no-network sandbox described in the docs.
 */
#include <windows.h>
static char dllpath[4096], output[4096];
static BYTE memory[8192], stack_memory[2048];
static DWORD64 start_address, stop_address;
static LONG CALLBACK capture(EXCEPTION_POINTERS *info) {
    if (info->ExceptionRecord->ExceptionCode != EXCEPTION_BREAKPOINT)
        return EXCEPTION_CONTINUE_SEARCH;
    DWORD64 address = (DWORD64)info->ExceptionRecord->ExceptionAddress;
    if (address == start_address) {
        info->ContextRecord->Rbp = (DWORD64)(memory + 128);
        info->ContextRecord->Rsp = (DWORD64)(stack_memory + 128);
        /* Original first instruction copies this header value to xmm0. */
        for (int i = 0; i < 16; ++i)
            ((BYTE *)&info->ContextRecord->Xmm0)[i] = stack_memory[128 + 0x60 + i];
        info->ContextRecord->Rip = start_address + 6;
        return EXCEPTION_CONTINUE_EXECUTION;
    }
    if (address != stop_address) return EXCEPTION_CONTINUE_SEARCH;
    HANDLE file = CreateFileA(output, GENERIC_WRITE, 0, 0, CREATE_NEW,
                              FILE_ATTRIBUTE_NORMAL, 0);
    if (file == INVALID_HANDLE_VALUE) ExitProcess(31);
    DWORD written;
    if (!WriteFile(file, memory + 128 - 0x30, 1680, &written, 0)
        || written != 1680) ExitProcess(32);
    CloseHandle(file);
    ExitProcess(0);
    return EXCEPTION_CONTINUE_SEARCH;
}
static void breakpoint(BYTE *site) {
    DWORD old, ignored;
    if (!VirtualProtect(site, 1, PAGE_EXECUTE_READWRITE, &old)) ExitProcess(26);
    *site = 0xcc;
    FlushInstructionCache(GetCurrentProcess(), site, 1);
    VirtualProtect(site, 1, old, &ignored);
}
void mainCRTStartup(void) {
    if (!GetEnvironmentVariableA("LAYOUT_DLL", dllpath, sizeof dllpath)
        || !GetEnvironmentVariableA("LAYOUT_OUTPUT", output, sizeof output)) ExitProcess(20);
    HMODULE module = LoadLibraryA(dllpath);
    if (!module) ExitProcess(22);
    BYTE *start = (BYTE *)module + 0x44b4, *stop = (BYTE *)module + 0x4635;
    if (start[0] != 0xc5 || start[1] != 0xf8 || start[2] != 0x10
        || start[3] != 0x44 || start[4] != 0x24 || start[5] != 0x60
        || stop[0] != 0x48 || stop[1] != 0x8b || stop[2] != 0x7c) ExitProcess(24);
    for (int i = 0; i < 4096 / 4; ++i)
        ((float *)(memory + 128 + 0x660))[i] = (float)(1000 + i);
    for (int i = 0; i < 28; ++i) stack_memory[128 + 0x60 + i] = (BYTE)(i + 1);
    start_address = (DWORD64)start; stop_address = (DWORD64)stop;
    if (!AddVectoredExceptionHandler(1, capture)) ExitProcess(25);
    breakpoint(start); breakpoint(stop);
    ((void (*)(void))start)();
    ExitProcess(27);
}
