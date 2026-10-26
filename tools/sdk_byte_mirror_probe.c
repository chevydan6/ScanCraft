/* Synthetic-only diagnostic for an externally hash-verified, self-contained
 * private byte-copy leaf. No vendor instructions are embedded in this source. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <unistd.h>
typedef void (__attribute__((ms_abi)) *copy_fn)(const uint8_t *,uint8_t *,uint32_t,uint32_t);
static void *allocate(size_t n,int prot) {
 void *p=mmap(NULL,n,prot,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);
 if(p==MAP_FAILED)exit(20);
 return p;
}
int main(int argc,char **argv) {
 if(argc!=2)return 2;
 FILE *f=fopen(argv[1],"rb");if(!f)return 3;
 long page=sysconf(_SC_PAGESIZE);if(page<4096)return 4;
 unsigned char *code=allocate(page,PROT_READ|PROT_WRITE);
 if(fread(code,1,93,f)!=93||fgetc(f)!=EOF||fclose(f))return 5;
 if(mprotect(code,page,PROT_READ|PROT_EXEC))return 6;
 copy_fn copy=(copy_fn)(void*)code;
 const uint32_t cases[][2]={{1,1},{1,7},{2,3},{7,3},{8,5},{1920,1200}};
 uint64_t tested=0;
 for(unsigned k=0;k<sizeof(cases)/sizeof(cases[0]);k++) {
  uint32_t w=cases[k][0],h=cases[k][1];size_t n=(size_t)w*h;
  size_t span=(n+page-1)/page*page;
  unsigned char *a=allocate(span+2*page,PROT_NONE),*b=allocate(span+2*page,PROT_NONE);
  if(mprotect(a+page,span,PROT_READ|PROT_WRITE)||mprotect(b+page,span,PROT_READ|PROT_WRITE))return 7;
  unsigned char *src=a+page+span-n,*dst=b+page+span-n;
  for(size_t i=0;i<n;i++)src[i]=(unsigned char)((i*73+(i/w)*19+(i%w)*11)^((i*7)>>8));
  memset(dst,0xa5,n);
  if(mprotect(a+page,span,PROT_READ))return 8;
  copy(src,dst,w,h);
  for(uint32_t y=0;y<h;y++)for(uint32_t x=0;x<w;x++) {
   size_t i=(size_t)y*w+x;
   if(dst[i]!=src[(size_t)(y+1)*w-1-x])return 9;
  }
  copy_fn same=copy;
  if(mprotect(a+page,span,PROT_READ|PROT_WRITE))return 10;
  same(dst,src,w,h);
  for(size_t i=0;i<n;i++) {
   unsigned char expected=(unsigned char)((i*73+(i/w)*19+(i%w)*11)^((i*7)>>8));
   if(src[i]!=expected)return 11;
  }
  munmap(a,span+2*page);munmap(b,span+2*page);tested+=n;
 }
 printf("{\"synthetic_shapes\":6,\"pixels_checked_each_direction\":%llu,\"row_reversal_exact\":true,\"involution_exact\":true}\n",(unsigned long long)tested);
 munmap(code,page);return 0;
}
