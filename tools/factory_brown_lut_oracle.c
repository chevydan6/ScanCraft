/* Synthetic-only offline Brown LUT probe. Run without hardware/network.
 * Uses installed DLL exports; no proprietary instructions are copied. */
#include <windows.h>
static char path[4096],output[4096],which[16];
static FARPROC api(HMODULE dll,WORD ordinal) {
    FARPROC p=GetProcAddress(dll,MAKEINTRESOURCEA(ordinal));
    if(!p)ExitProcess(22);return p;
}
void mainCRTStartup(void) {
    if(!GetEnvironmentVariableA("BROWN_DLL",path,sizeof path)||
       !GetEnvironmentVariableA("BROWN_OUTPUT",output,sizeof output)||
       !GetEnvironmentVariableA("BROWN_CASE",which,sizeof which))ExitProcess(20);
    int index=which[0]-'0';if(index<0||index>5||which[1])ExitProcess(21);
    HMODULE dll=LoadLibraryA(path);if(!dll)ExitProcess(23);
    BYTE *object=VirtualAlloc(0,65536,MEM_COMMIT|MEM_RESERVE,PAGE_READWRITE);
    if(!object)ExitProcess(24);
    void *(*ctor)(void*)=(void*)api(dll,2);
    void (*image)(void*,int,int)=(void*)api(dll,50);
    void (*intrinsic)(void*,const float*)=(void*)api(dll,48);
    void (*distortion)(void*,const float*,int)=(void*)api(dll,49);
    int (*config)(void*,int,int,float,BOOL)=(void*)api(dll,51);
    int (*calculate)(void*)=(void*)api(dll,27);
    ctor(object);
    float intr[4]={20.f,22.f,15.f,11.f},coeff[8]={0};
    if(index==1 || index==3){coeff[0]=-0.08f;coeff[1]=0.12f;coeff[2]=-0.04f;}
    if(index==2 || index==3){coeff[6]=0.01f;coeff[7]=-0.02f;}
    if(index==3 || index==4)coeff[3]=0.07f;
    if(index==3 || index==5){coeff[4]=-0.03f;coeff[5]=0.02f;}
    image(object,32,24);intrinsic(object,intr);distortion(object,coeff,8);
    if(config(object,0,0,2.f,FALSE))ExitProcess(25);
    int status=calculate(object);if(status)ExitProcess(26);
    unsigned short *shape=(void*)(object+0x90);
    unsigned count=(unsigned)shape[0]*shape[1];
    if(!count||count>20000||!*(void**)(object+0x88)||!*(void**)(object+0x98))ExitProcess(27);
    HANDLE file=CreateFileA(output,GENERIC_WRITE,0,0,CREATE_NEW,FILE_ATTRIBUTE_NORMAL,0);
    if(file==INVALID_HANDLE_VALUE)ExitProcess(28);
    DWORD written;int header[4]={32,24,shape[0],shape[1]};
    if(!WriteFile(file,header,16,&written,0)||written!=16)ExitProcess(29);
    if(!WriteFile(file,intr,16,&written,0)||written!=16)ExitProcess(29);
    if(!WriteFile(file,coeff,32,&written,0)||written!=32)ExitProcess(29);
    if(!WriteFile(file,object+0x60,64,&written,0)||written!=64)ExitProcess(29);
    if(!WriteFile(file,*(void**)(object+0x88),count*8,&written,0)||written!=count*8)ExitProcess(29);
    if(!WriteFile(file,*(void**)(object+0x98),count*8,&written,0)||written!=count*8)ExitProcess(29);
    CloseHandle(file);ExitProcess(0);
}
