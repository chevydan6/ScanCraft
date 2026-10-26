#!/usr/bin/env python3
"""Compile actual candidate policy functions in a small userspace test harness.
Generated C incorporates GPL-2.0 kernel functions; no hardware is accessed.
"""
import argparse
from pathlib import Path
import re
import subprocess


def function(source, name):
    start = source.index(name)
    opening = source.index('{', start)
    depth = 1
    end = opening+1
    while depth:
        depth += (source[end] == '{')-(source[end] == '}')
        end += 1
    return source[start:end]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('candidate', type=Path)
    parser.add_argument('--container', default='kernel-6900hs-lto-build')
    args = parser.parse_args()
    root = args.candidate.resolve()
    assert root == Path('/home/developer/kernel-6900hs-lto/scancraft-uvc-queue'), 'review container mount mapping for another path'
    header = (root/'uvcvideo.h').read_text()
    default = re.search(r'#define UVC_URBS\s+(\d+)', header).group(1)
    maximum = re.search(r'#define UVC_MAX_URBS\s+(\d+)', header).group(1)
    policy = function((root/'uvc_video.c').read_text(), 'unsigned int uvc_num_urbs(')
    setter = function((root/'uvc_driver.c').read_text(), 'static int uvc_scancraft_urbs_set(')
    scaffold = r'''
// SPDX-License-Identifier: GPL-2.0
#include <assert.h>
#include <stdbool.h>
#include <errno.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <limits.h>
typedef uint8_t u8;
#define le16_to_cpu(x) (x)
struct usb_device { struct { uint16_t idVendor, idProduct; } descriptor; };
struct uvc_device { struct usb_device *udev; };
struct usb_host_interface { struct { u8 bInterfaceNumber; } desc; };
struct usb_interface { struct usb_host_interface *cur_altsetting; };
struct uvc_streaming { struct uvc_device *dev; struct usb_interface *intf; };
struct kernel_param { void *arg; };
static int kstrtouint(const char *s, unsigned int base, unsigned int *out) {
    char *end; unsigned long value = strtoul(s, &end, base);
    if (*end || value > UINT_MAX) return -EINVAL;
    *out = (unsigned int)value; return 0;
}
'''
    main_c = r'''
int main(void) {
    struct usb_device usb = {0}; struct uvc_device dev = {&usb};
    struct usb_host_interface alt = {0}; struct usb_interface intf = {&alt};
    struct uvc_streaming stream = {&dev,&intf};
    unsigned int depths[] = {5,8,32}; unsigned int vendors[] = {0x2bc5,0x1234};
    unsigned int products[] = {0x06a8,0x0c0b}; unsigned int cases=0;
    for (unsigned int d=0;d<3;d++) for(unsigned int v=0;v<2;v++) for(unsigned int p=0;p<2;p++) for(unsigned int i=0;i<6;i++) {
        uvc_scancraft_urbs=depths[d];usb.descriptor.idVendor=vendors[v];usb.descriptor.idProduct=products[p];alt.desc.bInterfaceNumber=i;
        unsigned int expected=(vendors[v]==0x2bc5 && products[p]==0x06a8 && (i==1 || i==3)) ? depths[d] : UVC_URBS;
        assert(uvc_num_urbs(&stream)==expected);cases++;
    }
    struct kernel_param param={&uvc_scancraft_urbs}; const char *values[] = {"0","4","5","8","32","33"};
    for(unsigned int i=0;i<6;i++) {
        unsigned int old=uvc_scancraft_urbs; unsigned int value=(unsigned int)atoi(values[i]);
        int result=uvc_scancraft_urbs_set(values[i],&param);
        if(value<5 || value>32) {assert(result==-EINVAL);assert(uvc_scancraft_urbs==old);}
        else {assert(result==0);assert(uvc_scancraft_urbs==value);} cases++;
    }
    uvc_scancraft_urbs_frozen=true;
    unsigned int old=uvc_scancraft_urbs;
    assert(uvc_scancraft_urbs_set("8",&param)==-EPERM);assert(uvc_scancraft_urbs==old);cases++;
    printf("{\"scope\":\"compiled candidate policy only; not kernel runtime or USB qualification\",\"cases\":%u,\"pass\":true}\n",cases);
    return 0;
}
'''
    (root/'policy-test.c').write_text(scaffold+f'\n#define UVC_URBS {default}\n#define UVC_MAX_URBS {maximum}\nunsigned int uvc_scancraft_urbs=UVC_URBS;\nstatic bool uvc_scancraft_urbs_frozen;\n'+policy+'\n'+setter+main_c)
    prefix = ['podman', 'exec', args.container]
    subprocess.run(prefix+['clang', '-std=c11', '-Wall', '-Wextra', '-Werror', '/work/scancraft-uvc-queue/policy-test.c', '-o', '/work/scancraft-uvc-queue/policy-test'], check=True)
    subprocess.run(prefix+['/work/scancraft-uvc-queue/policy-test'], check=True)


if __name__ == '__main__':
    main()
