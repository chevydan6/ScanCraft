#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Exercise actual candidate callback code with explicit userspace kernel stubs.

This checks dispatch, copy bounds, references and cancellation; it does not run
the kernel allocator, real USB submission, V4L2 concurrency or IRQ timing.
Usage: python3 test-policy.py CANDIDATE_SOURCE_DIRECTORY
"""
from pathlib import Path
import os
import subprocess
import sys
import tempfile


def function(source, signature):
    start = source.index(signature)
    opening = source.index("{", start)
    depth = 1
    end = opening + 1
    while depth:
        depth += (source[end] == "{") - (source[end] == "}")
        end += 1
    return source[start:end]


root = Path(sys.argv[1])
video = (root / "uvc_video.c").read_text()
driver = (root / "uvc_driver.c").read_text()
init = function(driver, "static int __init uvc_init(void)")
assert init.index("uvc_scancraft_inline_copy_frozen = true;") < init.index("usb_register(")
assert "&uvc_scancraft_inline_copy, 0444)" in driver
assert "bool uvc_scancraft_inline_copy;" in driver
parts = [function(video, "static void uvc_video_copy_data_work("),
         function(video, "static void uvc_video_try_inline_copy("),
         function(video, "static void uvc_video_complete("),
         function(driver, "static int uvc_scancraft_inline_copy_set(")]

stub = r'''
#include <assert.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <errno.h>
typedef uint8_t u8;
#define V4L2_BUF_TYPE_VIDEO_CAPTURE 1
#define GFP_ATOMIC 1
#define GFP_KERNEL 2
#define fallthrough __attribute__((fallthrough))
#define le16_to_cpu(v) (v)
#define container_of(p,t,m) ((t *)((char *)(p)-offsetof(t,m)))
#define dev_warn(...) (++warnings)
#define dev_err(...) (++errors)
struct work_struct { int unused; };
struct vb2_queue { int unused; };
struct uvc_buffer { int refs; bool error; };
struct uvc_video_queue {
    struct { struct vb2_queue *queue; } vdev;
    struct uvc_buffer *current;
};
struct usb_device { struct { uint16_t idVendor, idProduct; } descriptor; };
struct usb_host_interface { struct { u8 bInterfaceNumber; } desc; };
struct usb_interface { struct usb_host_interface *cur_altsetting; int dev; };
struct uvc_device { struct usb_device *udev; };
struct urb { void *context; int status; unsigned pipe, actual_length, transfer_buffer_length; };
struct uvc_streaming;
struct uvc_copy_op { struct uvc_buffer *buf; void *dst; const void *src; unsigned len; };
struct uvc_urb {
    struct uvc_streaming *stream; struct urb *urb; unsigned async_operations;
    struct uvc_copy_op copy_operations[2]; struct work_struct work;
};
struct uvc_streaming {
    struct uvc_device *dev; struct usb_interface *intf; unsigned type; bool frozen;
    struct uvc_video_queue queue; struct { struct uvc_video_queue queue; } meta;
    void (*decode)(struct uvc_urb *,struct uvc_buffer *,struct uvc_buffer *);
    void *async_wq;
};
static bool uvc_scancraft_inline_copy;
static bool uvc_scancraft_inline_copy_frozen;
struct kernel_param { void *arg; };
static int setter_calls, warnings, errors, submissions, submitted_flags;
static int submit_result, releases, queued, cancels, disconnect_cancels, decodes;
static unsigned planned_ops, planned_len;
static unsigned char src[2][16416], dst[2][16416];
static bool usb_pipebulk(unsigned p) { return (p & 1) != 0; }
static bool usb_pipein(unsigned p) { return (p & 2) != 0; }
static int param_set_bool(const char *s, const struct kernel_param *kp) {
    ++setter_calls;
    if (!strcmp(s,"1") || !strcmp(s,"y")) *(bool *)kp->arg=true;
    else if (!strcmp(s,"0") || !strcmp(s,"n")) *(bool *)kp->arg=false;
    else return -EINVAL;
    return 0;
}
static int usb_submit_urb(struct urb *u, int flags) {
    (void)u; ++submissions; submitted_flags=flags; return submit_result;
}
static void uvc_queue_buffer_release(struct uvc_buffer *b) {
    assert(b->refs > 0);
    assert(releases < (int)planned_ops);
    assert(!memcmp(dst[releases]+8,src[releases]+8,planned_len));
    --b->refs; ++releases;
}
static void uvc_queue_cancel(struct uvc_video_queue *q,bool disconnected) {
    (void)q; ++cancels; disconnect_cancels += disconnected;
}
static struct uvc_buffer *uvc_queue_get_current_buffer(struct uvc_video_queue *q) { return q->current; }
static void queue_work(void *wq,struct work_struct *work) { (void)wq; (void)work; ++queued; }
static void decode(struct uvc_urb *u,struct uvc_buffer *b,struct uvc_buffer *m) {
    (void)m; ++decodes; assert(u->async_operations==0);
    for(unsigned i=0;i<planned_ops;++i) {
        ++b->refs;
        u->copy_operations[i]=(struct uvc_copy_op){b,dst[i]+8,src[i]+8,planned_len};
    }
    u->async_operations=planned_ops;
}
'''

checks = r'''
static struct usb_device device;
static struct uvc_device udev;
static struct usb_host_interface setting;
static struct usb_interface interface;
static struct uvc_buffer buffer;
static struct vb2_queue metadata;
static struct uvc_streaming stream;
static struct urb urb;
static struct uvc_urb uvc;
static unsigned cases;
static void reset(void) {
    warnings=errors=submissions=submitted_flags=submit_result=releases=queued=cancels=disconnect_cancels=decodes=0;
    planned_ops=1; planned_len=12276;
    memset(&device,0,sizeof device); device.descriptor.idVendor=0x2bc5; device.descriptor.idProduct=0x06a8;
    udev.udev=&device; setting.desc.bInterfaceNumber=1; interface.cur_altsetting=&setting;
    buffer=(struct uvc_buffer){0,true};
    memset(&stream,0,sizeof stream); stream.dev=&udev; stream.intf=&interface;
    stream.type=V4L2_BUF_TYPE_VIDEO_CAPTURE; stream.decode=decode;
    stream.queue.current=&buffer; stream.meta.queue.vdev.queue=&metadata;
    urb=(struct urb){&uvc,0,3,12288,12288};
    memset(&uvc,0,sizeof uvc); uvc.stream=&stream; uvc.urb=&urb;
    for(unsigned i=0;i<2;++i) for(unsigned j=0;j<sizeof src[i];++j) src[i][j]=(unsigned char)(j*17+i);
    memset(dst,0xa5,sizeof dst);
    uvc_scancraft_inline_copy=true;
}
static void verify_bytes(void) {
    for(unsigned i=0;i<2;++i) for(unsigned j=0;j<sizeof dst[i];++j) {
        bool copied=i<planned_ops && j>=8 && j<8+planned_len;
        assert(dst[i][j]==(copied ? src[i][j] : 0xa5));
    }
    assert(buffer.refs==0 && buffer.error);
}
static void run(bool inlined) {
    uvc_video_complete(&urb);
    assert(decodes==1 && cancels==0);
    if(inlined || !planned_ops) {
        assert(queued==0 && submissions==1 && submitted_flags==GFP_ATOMIC);
        assert(releases==(int)planned_ops && uvc.async_operations==0);
    } else {
        assert(queued==1 && submissions==0 && releases==0);
        assert(buffer.refs==(int)planned_ops);
        for(unsigned i=0;i<sizeof dst;++i) assert(((unsigned char *)dst)[i]==0xa5);
        uvc_video_copy_data_work(&uvc.work);
        assert(submissions==1 && submitted_flags==GFP_KERNEL && releases==(int)planned_ops);
    }
    assert(errors==(submit_result<0)); verify_bytes(); ++cases;
}
int main(void) {
    /* Default opt-out must preserve the original deferred path. */
    assert(!uvc_scancraft_inline_copy);
    reset(); uvc_scancraft_inline_copy=false; run(false);
    unsigned sizes[]={0,1,12,12276,12288};
    for(unsigned i=0;i<2;++i) for(unsigned j=0;j<5;++j) {
        reset(); setting.desc.bInterfaceNumber=i ? 3 : 1; planned_len=sizes[j]; run(true);
    }
    reset(); planned_ops=0; run(false);
    reset(); planned_ops=2; run(false);
    reset(); urb.actual_length=urb.transfer_buffer_length=12289; planned_len=12289; run(false);
    reset(); urb.transfer_buffer_length=12289; run(false);
    reset(); urb.transfer_buffer_length=12287; run(false);
    reset(); urb.actual_length=0; planned_len=0; run(false);
    reset(); planned_len=12289; run(false);
    reset(); urb.actual_length=100; planned_len=101; run(false);
    for(unsigned i=0;i<9;++i) if(i!=1 && i!=3) { reset(); setting.desc.bInterfaceNumber=i; run(false); }
    reset(); device.descriptor.idVendor=0x1234; run(false);
    reset(); device.descriptor.idProduct=0x06a9; run(false);
    reset(); stream.type=2; run(false);
    for(unsigned pipe=0;pipe<3;++pipe) { reset(); urb.pipe=pipe; run(false); }
    reset(); submit_result=-ENODEV; run(true);
    reset(); uvc_scancraft_inline_copy=false; submit_result=-ENODEV; run(false);
    int statuses[]={-ENOENT,-ECONNRESET,-ESHUTDOWN,-EIO};
    for(unsigned m=0;m<2;++m) for(unsigned f=0;f<2;++f) for(unsigned i=0;i<4;++i) {
        reset(); urb.status=statuses[i]; stream.frozen=f;
        if(!m) stream.meta.queue.vdev.queue=NULL;
        uvc_video_complete(&urb);
        int expected=(urb.status==-ENOENT || urb.status==-EIO) && f ? 0 : 1+(int)m;
        assert(cancels==expected && disconnect_cancels==(urb.status==-ESHUTDOWN ? expected : 0));
        assert(!decodes && !queued && !submissions && !releases && !buffer.refs);
        assert(warnings==(urb.status==-EIO)); ++cases;
    }
    struct kernel_param kp={&uvc_scancraft_inline_copy};
    uvc_scancraft_inline_copy_frozen=false; setter_calls=0;
    assert(!uvc_scancraft_inline_copy_set("1",&kp) && uvc_scancraft_inline_copy);
    assert(uvc_scancraft_inline_copy_set("invalid",&kp)==-EINVAL && uvc_scancraft_inline_copy);
    assert(!uvc_scancraft_inline_copy_set("0",&kp) && !uvc_scancraft_inline_copy);
    uvc_scancraft_inline_copy_frozen=true;
    int before=setter_calls;
    assert(uvc_scancraft_inline_copy_set("1",&kp)==-EPERM && !uvc_scancraft_inline_copy);
    assert(setter_calls==before); cases+=4;
    printf("%u callback/copy/cancellation/parameter cases passed\n",cases);
    return 0;
}
'''

with tempfile.TemporaryDirectory(prefix="scancraft-inline-policy-") as temp:
    path = Path(temp)
    (path / "test.c").write_text(stub + "\n".join(parts) + checks)
    command = [os.environ.get("CC", "cc"), "-std=c11", "-O2", "-Wall", "-Wextra", "-Werror",
               str(path / "test.c"), "-o", str(path / "test")]
    subprocess.run(command, check=True)
    subprocess.run([str(path / "test")], check=True)
