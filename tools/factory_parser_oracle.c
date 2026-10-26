/* Isolated offline file-parser probe with optional private file export.
 * No scanner SDK/device/control APIs. Must run with hardware devices absent. */
typedef void *P; typedef unsigned int U; typedef int I;
__declspec(dllimport) P LoadLibraryA(const char*);
__declspec(dllimport) P GetProcAddress(P,const char*);
__declspec(dllimport) U GetEnvironmentVariableA(const char*,char*,U);
__declspec(dllimport) U GetFileAttributesA(const char*);
__declspec(dllimport) P VirtualAlloc(P,unsigned long long,U,U);
__declspec(dllimport) P CreateFileA(const char*,U,U,P,U,U,P);
__declspec(dllimport) I WriteFile(P,const void*,U,U*,P);
__declspec(dllimport) I CloseHandle(P);
__declspec(dllimport) void ExitProcess(U);
static char dllpath[4096],input[4096],output[4096],textpath[4096];
void mainCRTStartup(void){
 if(!GetEnvironmentVariableA("PARSER_DLL",dllpath,sizeof dllpath)||!GetEnvironmentVariableA("PARSER_INPUT",input,sizeof input)||!GetEnvironmentVariableA("PARSER_OUTPUT",output,sizeof output))ExitProcess(20);
 P lib=LoadLibraryA(dllpath); if(!lib)ExitProcess(21);
 I (*load)(const char*,P,I)=GetProcAddress(lib,"LoadCalibrationParamsMulti");if(!load)ExitProcess(22);
 P buffer=VirtualAlloc(0,131072,0x3000,4);if(!buffer)ExitProcess(23);
 I status=load(input,buffer,0);
 P file=CreateFileA(output,0x40000000,0,0,1,0x80,0);if(file==(P)-1)ExitProcess(24);
 U written;if(!WriteFile(file,&status,4,&written,0)||written!=4)ExitProcess(25);
 if(!WriteFile(file,buffer,131072,&written,0)||written!=131072)ExitProcess(26);
 CloseHandle(file);
 if(!status && GetEnvironmentVariableA("PARSER_TEXT",textpath,sizeof textpath)) {
   if(GetFileAttributesA(textpath)!=(U)-1)ExitProcess(29);
   I (*save)(const char*,P,I,I)=GetProcAddress(lib,"WriteCalibrationParamsMulti");
   if(!save || save(textpath,buffer,3,0))ExitProcess(28);
 }
 ExitProcess(status?27:0);
}
