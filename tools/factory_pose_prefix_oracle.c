/* Offline synthetic numerical-prefix probe for one hashed vendor DLL.
 * Run only in a private sandbox without scanner devices or network.
 * Stops at a breakpoint before the remainder of the engine can execute.
 * Does not copy or redistribute vendor instructions or accept device data.
 */
#include <windows.h>
static char dllpath[4096],output[4096],which[16];
static float values[6];
static DWORD64 breakpoint_address;
static LONG CALLBACK capture(EXCEPTION_POINTERS *info) {
    if(info->ExceptionRecord->ExceptionCode != EXCEPTION_BREAKPOINT ||
       (DWORD64)info->ExceptionRecord->ExceptionAddress != breakpoint_address)
        return EXCEPTION_CONTINUE_SEARCH;
    HANDLE file=CreateFileA(output,GENERIC_WRITE,0,0,CREATE_NEW,FILE_ATTRIBUTE_NORMAL,0);
    if(file==INVALID_HANDLE_VALUE)ExitProcess(31);
    DWORD written;
    if(!WriteFile(file,values,sizeof values,&written,0)||written!=sizeof values)ExitProcess(32);
    const BYTE *stack=(const BYTE*)info->ContextRecord->Rsp;
    if(!WriteFile(file,stack+0x1c0,72,&written,0)||written!=72)ExitProcess(33);
    if(!WriteFile(file,stack+0x310,128,&written,0)||written!=128)ExitProcess(34);
    CloseHandle(file);ExitProcess(0);
    return EXCEPTION_CONTINUE_SEARCH;
}
void mainCRTStartup(void) {
    if(!GetEnvironmentVariableA("POSE_DLL",dllpath,sizeof dllpath)||
       !GetEnvironmentVariableA("POSE_OUTPUT",output,sizeof output)||
       !GetEnvironmentVariableA("POSE_CASE",which,sizeof which))ExitProcess(20);
    int index=which[0]-'0';
    if(index<0 || index>5 || which[1])ExitProcess(21);
    if(index==1)values[0]=0.2f;
    if(index==2)values[1]=-0.3f;
    if(index==3)values[2]=0.4f;
    if(index>=4){values[0]=0.2f;values[1]=-0.3f;values[2]=0.4f;}
    if(index==5){values[3]=10.f;values[4]=-20.f;values[5]=30.f;}
    HMODULE module=LoadLibraryA(dllpath);if(!module)ExitProcess(22);
    BYTE *input=VirtualAlloc(0,131072,MEM_COMMIT|MEM_RESERVE,PAGE_READWRITE);
    void *a=VirtualAlloc(0,65536,MEM_COMMIT|MEM_RESERVE,PAGE_READWRITE);
    void *b=VirtualAlloc(0,65536,MEM_COMMIT|MEM_RESERVE,PAGE_READWRITE);
    void *c=VirtualAlloc(0,65536,MEM_COMMIT|MEM_RESERVE,PAGE_READWRITE);
    if(!input||!a||!b||!c)ExitProcess(23);
    for(int i=0;i<6;i++)((float*)(input+0x864))[i]=values[i];
    BYTE *entry=(BYTE*)module+0x127f0;
    BYTE *stop=(BYTE*)module+0x12b7b;
    /* Refuse a different instruction at the known stop location. Caller must
       additionally verify the exact local DLL hash recorded in evidence. */
    if(stop[0]!=0xc4 || stop[1]!=0xc1 || stop[2]!=0x7a || stop[3]!=0x10)ExitProcess(24);
    breakpoint_address=(DWORD64)stop;
    if(!AddVectoredExceptionHandler(1,capture))ExitProcess(25);
    DWORD old;
    if(!VirtualProtect(stop,1,PAGE_EXECUTE_READWRITE,&old))ExitProcess(26);
    stop[0]=0xcc;
    FlushInstructionCache(GetCurrentProcess(),stop,1);
    DWORD ignored;VirtualProtect(stop,1,old,&ignored);
    void (*prefix)(void*,void*,void*,void*)=(void*)entry;
    prefix(input,a,b,c);
    ExitProcess(27); /* Missing breakpoint is a failed experiment. */
}
