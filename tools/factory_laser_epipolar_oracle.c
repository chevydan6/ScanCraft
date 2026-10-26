/* Synthetic-only epipolar numerical probe for the hashed preprocessing DLL.
 * Run without scanner nodes/network, in a private prefix. Stops before return.
 */
#include <windows.h>
static char dllpath[4096], output[4096], which[16];
static float values[20];
static BYTE input[65536];
static DWORD64 stop_address;
static LONG CALLBACK capture(EXCEPTION_POINTERS *info) {
    if (info->ExceptionRecord->ExceptionCode != EXCEPTION_BREAKPOINT ||
        (DWORD64)info->ExceptionRecord->ExceptionAddress != stop_address)
        return EXCEPTION_CONTINUE_SEARCH;
    HANDLE file = CreateFileA(output, GENERIC_WRITE, 0, 0, CREATE_NEW,
                              FILE_ATTRIBUTE_NORMAL, 0);
    if (file == INVALID_HANDLE_VALUE) ExitProcess(31);
    DWORD written;
    if (!WriteFile(file, values, sizeof values, &written, 0)
        || written != sizeof values) ExitProcess(32);
    if (!WriteFile(file, input + 0x5fc, 72, &written, 0)
        || written != 72) ExitProcess(33);
    CloseHandle(file); ExitProcess(0);
    return EXCEPTION_CONTINUE_SEARCH;
}
void mainCRTStartup(void) {
    if (!GetEnvironmentVariableA("EPIPOLAR_DLL", dllpath, sizeof dllpath)
        || !GetEnvironmentVariableA("EPIPOLAR_OUTPUT", output, sizeof output)
        || !GetEnvironmentVariableA("EPIPOLAR_CASE", which, sizeof which)) ExitProcess(20);
    int index = which[0] - '0';
    if (index < 0 || index > 5 || which[1]) ExitProcess(21);
    values[0] = 800.f + index * 17.f; values[1] = 900.f + index * 11.f;
    values[2] = 320.f; values[3] = 240.f;
    values[4] = 850.f + index * 7.f; values[5] = 950.f + index * 13.f;
    values[6] = 300.f; values[7] = 250.f;
    values[8] = values[12] = values[16] = 1.f;
    if (index) values[17] = -100.f;
    if (index >= 2) { values[18] = 2.f; values[19] = 5.f; }
    if (index == 3) {
        values[8] = values[12] = 0.f; values[9] = -1.f; values[11] = 1.f;
    }
    if (index == 4) {
        values[8] = values[16] = 0.8f; values[10] = 0.6f; values[14] = -0.6f;
        values[17] = 7.f; values[18] = -80.f; values[19] = 13.f;
    }
    if (index == 5) {
        values[8] = values[12] = values[16] = 0.f;
        values[10] = values[11] = values[15] = 1.f;
        values[17] = 15.f; values[18] = -7.f; values[19] = 9.f;
    }
    for (int i = 0; i < 4; ++i) {
        ((float *)input)[i] = values[i];
        ((float *)(input + 0x34))[i] = values[4 + i];
    }
    for (int i = 0; i < 12; ++i) ((float *)(input + 0x68))[i] = values[8 + i];
    HMODULE module = LoadLibraryA(dllpath);
    if (!module) ExitProcess(22);
    BYTE *entry = (BYTE *)module + 0x75f0, *stop = (BYTE *)module + 0x7ba1;
    if (entry[0] != 0x48 || entry[1] != 0x8b || entry[2] != 0xc4 ||
        stop[0] != 0x48 || stop[1] != 0x8b || stop[2] != 0x8d) ExitProcess(24);
    stop_address = (DWORD64)stop;
    if (!AddVectoredExceptionHandler(1, capture)) ExitProcess(25);
    DWORD old, ignored;
    if (!VirtualProtect(stop, 1, PAGE_EXECUTE_READWRITE, &old)) ExitProcess(26);
    *stop = 0xcc; FlushInstructionCache(GetCurrentProcess(), stop, 1);
    VirtualProtect(stop, 1, old, &ignored);
    ((void (*)(void *))entry)(input);
    ExitProcess(27);
}
