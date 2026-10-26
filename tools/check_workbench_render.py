#!/usr/bin/env python3
"""Capture this app on a separate authenticated X display; no scanner/UI automation.
Requires Xvfb, xauth and ImageMagick import. Does not access the user's desktop.
"""
import argparse
import json
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time


def stop(child):
    if child is not None and child.poll() is None:
        child.terminate()
        try:
            child.wait(timeout=3)
        except subprocess.TimeoutExpired:
            child.kill()
            child.wait(timeout=3)


def window_geometry(display):
    # Read only the task's isolated X display, never the user's desktop.
    import ctypes as c
    x = c.CDLL('libX11.so.6')
    x.XOpenDisplay.argtypes = [c.c_char_p]; x.XOpenDisplay.restype = c.c_void_p
    x.XDefaultRootWindow.argtypes = [c.c_void_p]; x.XDefaultRootWindow.restype = c.c_ulong
    x.XQueryTree.argtypes = [c.c_void_p,c.c_ulong,c.POINTER(c.c_ulong),c.POINTER(c.c_ulong),c.POINTER(c.POINTER(c.c_ulong)),c.POINTER(c.c_uint)]
    x.XGetGeometry.argtypes = [c.c_void_p,c.c_ulong,c.POINTER(c.c_ulong),c.POINTER(c.c_int),c.POINTER(c.c_int),c.POINTER(c.c_uint),c.POINTER(c.c_uint),c.POINTER(c.c_uint),c.POINTER(c.c_uint)]
    x.XFree.argtypes = [c.c_void_p]; x.XCloseDisplay.argtypes = [c.c_void_p]
    connection = x.XOpenDisplay(display.encode())
    if not connection:
        raise RuntimeError('cannot inspect private display geometry')
    children = c.POINTER(c.c_ulong)()
    try:
        root,parent,count = c.c_ulong(),c.c_ulong(),c.c_uint()
        if not x.XQueryTree(connection,x.XDefaultRootWindow(connection),c.byref(root),c.byref(parent),c.byref(children),c.byref(count)):
            raise RuntimeError('cannot query private display windows')
        windows=[]
        for window in children[:count.value]:
            gx,gy,w,h,border,depth = c.c_int(),c.c_int(),c.c_uint(),c.c_uint(),c.c_uint(),c.c_uint()
            if x.XGetGeometry(connection,window,c.byref(root),c.byref(gx),c.byref(gy),c.byref(w),c.byref(h),c.byref(border),c.byref(depth)):
                windows.append({'id':window,'x':gx.value,'y':gy.value,'width':w.value,'height':h.value})
        return windows
    finally:
        if children:
            x.XFree(children)
        x.XCloseDisplay(connection)


def process_cpu_ticks(pid):
    fields = Path(f'/proc/{pid}/stat').read_text().rsplit(')', 1)[1].split()
    return int(fields[11])+int(fields[12])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('project', type=Path)
    parser.add_argument('output', type=Path)
    modes = parser.add_mutually_exclusive_group()
    modes.add_argument('--statistical-preview', action='store_true', help='preview statistical candidates with k=4 and sigma=1')
    modes.add_argument('--open-scan-project', action='store_true', help='verify and open exported scan folder')
    modes.add_argument('--export-scan-project', action='store_true', help='export saved project, sources and full recordings from input directory')
    modes.add_argument('--archive-recordings', action='store_true', help='archive complete input left.raptor/right.raptor into result/')
    modes.add_argument('--export-project-bundle', action='store_true', help='export input bundle project and retained sources into result/')
    modes.add_argument('--verify-source-bundle', action='store_true', help='load project.orproj from the input bundle and explicitly verify its sources')
    modes.add_argument('--reconstruct-recordings', action='store_true', help='reconstruct bundled synthetic recorded stereo frame selections')
    modes.add_argument('--play-recording', action='store_true', help='open synthetic recording in the native offline playback panel')
    modes.add_argument('--recover-recording', action='store_true', help='recover synthetic interrupted recording into result/')
    modes.add_argument('--import-mesh', action='store_true', help='import synthetic triangle STL/OBJ in mm with exact welding into result/')
    modes.add_argument('--mesh-smooth-preview', action='store_true', help='preview constrained smoothing with explicit fixture bounds')
    modes.add_argument('--mesh-simplify-preview', action='store_true', help='preview 70-triangle simplification with explicit fixture bounds')
    modes.add_argument('--mesh-section', action='store_true', help='compute z=0.5 mesh section on startup')
    modes.add_argument('--mesh-plane-preview', action='store_true', help='open mesh with an unapplied x<=0 trim preview')
    modes.add_argument('--mesh', action='store_true', help='open mesh JSON in the native mesh viewer')
    modes.add_argument('--mesh-boundaries', action='store_true', help='open mesh and analyze indexed boundaries')
    modes.add_argument('--fit-cylinder', action='store_true', help='fit all input points with a 0.05 mm RMS gate')
    modes.add_argument('--frame-document', action='store_true', help='load reusable frame without applying it')
    modes.add_argument('--coordinate-frame', action='store_true', help='open manual coordinate frame controls')
    modes.add_argument('--measurement-outlines', action='store_true', help='show saved historical measurement fits')
    modes.add_argument('--cloud-normals', action='store_true', help='open point cloud with sampled normal arrows')
    modes.add_argument('--mesh-normals', action='store_true', help='open mesh with sampled face-normal arrows')
    modes.add_argument('--fuse-sequence', action='store_true', help='project argument names a dataset directory with sequence.json')
    modes.add_argument('--reconstruct', action='store_true', help='project argument names a synthetic-style dataset directory with calibration.rapcal, left.png, right.png and request.json')
    parser.add_argument('--captures', type=int, default=1, choices=(1,2,3), help='repeat private-display screenshots one second apart')
    parser.add_argument('--binary', type=Path, default=Path('target/release/scancraft'))
    parser.add_argument('--xvfb', type=Path, default=Path(shutil.which('Xvfb') or Path.home()/'.cache/scancraft-test-tools/usr/bin/Xvfb'))
    args = parser.parse_args()
    project, binary, output = args.project.resolve(), args.binary.resolve(), args.output.resolve()
    valid_input=project.is_dir() if args.reconstruct or args.reconstruct_recordings or args.verify_source_bundle or args.export_project_bundle or args.archive_recordings or args.export_scan_project or args.open_scan_project or args.fuse_sequence else project.is_file()
    if not valid_input or not binary.is_file() or not args.xvfb.is_file():
        raise SystemExit('input project/dataset, release binary and Xvfb must exist')
    display = next(n for n in range(90, 200) if not Path(f'/tmp/.X11-unix/X{n}').exists() and not Path(f'/tmp/.X{n}-lock').exists())
    output.mkdir()
    app, server = None, None
    with tempfile.TemporaryDirectory(prefix='scancraft-private-display-') as temp:
        authority = Path(temp)/'authority'
        authority.touch(mode=0o600)
        cookie = os.urandom(16).hex()
        subprocess.run(['xauth', '-f', str(authority), 'add', f':{display}', 'MIT-MAGIC-COOKIE-1', cookie], check=True, capture_output=True)
        env = os.environ.copy()
        env.update({'DISPLAY': f':{display}', 'XAUTHORITY': str(authority), 'WINIT_UNIX_BACKEND': 'x11',
                    'XDG_SESSION_TYPE': 'x11', 'LIBGL_ALWAYS_SOFTWARE': '1'})
        env.pop('WAYLAND_DISPLAY', None)
        try:
            with (output/'server.log').open('w') as server_log, (output/'app.log').open('w') as app_log:
                server = subprocess.Popen([str(args.xvfb), f':{display}', '-screen', '0', '1280x900x24', '-nolisten', 'tcp', '-auth', str(authority), '-noreset'], stdout=server_log, stderr=subprocess.STDOUT)
                for _ in range(50):
                    if server.poll() is not None:
                        raise RuntimeError('private display server exited; inspect server.log')
                    if Path(f'/tmp/.X11-unix/X{display}').exists():
                        break
                    time.sleep(0.1)
                else:
                    raise RuntimeError('private display did not become ready')
                command=[str(binary),str(project)]
                if args.reconstruct:
                    command=[str(binary),'--reconstruct',str(project/'calibration.rapcal'),str(project/'left.png'),str(project/'right.png'),str(project/'request.json'),str(output/'result')]
                if args.reconstruct_recordings:
                    command=[str(binary),'--reconstruct-recordings',str(project/'calibration.rapcal'),str(project/'left.raptor'),'1',str(project/'right.raptor'),'2',str(project/'request.json'),str(output/'result')]
                if args.statistical_preview:
                    command=[str(binary),'--statistical-preview',str(project),'4','1']
                if args.export_scan_project:
                    command=[str(binary),'--export-scan-project',str(project/'project.orproj'),str(project),str(project/'left.raptor'),str(project/'right.raptor'),str(output/'result')]
                if args.archive_recordings:
                    command=[str(binary),'--archive-recordings',str(project/'left.raptor'),str(project/'right.raptor'),str(output/'result')]
                if args.export_project_bundle:
                    command=[str(binary),'--export-project-bundle',str(project/'project.orproj'),str(project),str(output/'result')]
                if args.verify_source_bundle:
                    command=[str(binary),'--verify-source-bundle',str(project/'project.orproj'),str(project)]
                if args.fuse_sequence:
                    command=[str(binary),'--fuse-sequence',str(project/'sequence.json'),str(output/'result')]
                if args.mesh_plane_preview:
                    command=[str(binary),'--mesh-plane-preview',str(project),'1','0','0','0']
                if args.mesh:
                    command=[str(binary),'--mesh',str(project)]
                if args.play_recording:
                    command=[str(binary),'--play-recording',str(project)]
                if args.recover_recording:
                    command=[str(binary),'--recover-recording',str(project),'1000',str(output/'result')]
                if args.import_mesh:
                    command=[str(binary),'--import-mesh',str(project),project.suffix.lower().lstrip('.'),'mm','weld-exact',str(output/'result')]
                if args.mesh_smooth_preview:
                    command=[str(binary),'--mesh-smooth-preview',str(project),'5','0.5','0.1','30','180']
                if args.mesh_simplify_preview:
                    command=[str(binary),'--mesh-simplify-preview',str(project),'70','1.5','30','30','100']
                if args.mesh_section:
                    command=[str(binary),'--mesh-section',str(project),'0','0','0.5','0','0','1','0']
                if args.mesh_boundaries:
                    command=[str(binary),'--mesh-boundaries',str(project)]
                if args.fit_cylinder:
                    command=[str(binary),'--fit-cylinder',str(project),'0.05']
                if args.frame_document:
                    command=[str(binary),'--frame-document',str(project)]
                if args.coordinate_frame:
                    command=[str(binary),'--coordinate-frame',str(project)]
                if args.measurement_outlines:
                    command=[str(binary),'--measurement-outlines',str(project)]
                if args.cloud_normals:
                    command=[str(binary),'--cloud-normals',str(project)]
                if args.mesh_normals:
                    command=[str(binary),'--mesh-normals',str(project)]
                binary_sha256 = hashlib.sha256(binary.read_bytes()).hexdigest()
                app = subprocess.Popen(command, env=env, stdout=app_log, stderr=subprocess.STDOUT)
                time.sleep(5)
                if app.poll() is not None:
                    raise RuntimeError('app exited before capture; inspect app.log')
                camera_fds = []
                for fd in Path(f'/proc/{app.pid}/fd').iterdir():
                    try:
                        name = os.readlink(fd)
                    except FileNotFoundError:
                        continue
                    if name.startswith(('/dev/video', '/dev/bus/usb/')):
                        camera_fds.append(name)
                assert not camera_fds, camera_fds
                cpu_start=process_cpu_ticks(app.pid)
                cpu_started=time.monotonic()
                windows=[]
                for capture in range(args.captures):
                    if capture:
                        time.sleep(1)
                    # XOpenDisplay below receives the same authenticated task display.
                    old_authority = os.environ.get('XAUTHORITY')
                    os.environ['XAUTHORITY'] = str(authority)
                    try:
                        windows.append(window_geometry(f':{display}'))
                    finally:
                        if old_authority is None: os.environ.pop('XAUTHORITY', None)
                        else: os.environ['XAUTHORITY'] = old_authority
                    name = 'window.png' if capture == 0 else f'window-{capture+1}.png'
                    subprocess.run(['import', '-display', f':{display}', '-window', 'root', str(output/name)], env=env, check=True, timeout=10)
                result = {'scope': 'private-display startup render only; requires screenshot inspection, not full GUI workflow or hardware qualification',
                          'project': str(project), 'mode': 'statistical_preview' if args.statistical_preview else 'open_scan_project' if args.open_scan_project else 'export_scan_project' if args.export_scan_project else 'archive_recordings' if args.archive_recordings else 'export_project_bundle' if args.export_project_bundle else 'verify_source_bundle' if args.verify_source_bundle else 'reconstruct_recordings' if args.reconstruct_recordings else 'play_recording' if args.play_recording else 'recover_recording' if args.recover_recording else 'import_mesh' if args.import_mesh else 'mesh_smooth_preview' if args.mesh_smooth_preview else 'mesh_simplify_preview' if args.mesh_simplify_preview else 'fit_cylinder' if args.fit_cylinder else 'frame_document' if args.frame_document else 'coordinate_frame' if args.coordinate_frame else 'mesh_section' if args.mesh_section else 'measurement_outlines' if args.measurement_outlines else 'cloud_normals' if args.cloud_normals else 'mesh_normals' if args.mesh_normals else 'mesh_boundaries' if args.mesh_boundaries else ('mesh_plane_preview' if args.mesh_plane_preview else ('mesh' if args.mesh else ('fuse_sequence' if args.fuse_sequence else ('reconstruct' if args.reconstruct else 'open_file')))), 'result_bundle': str(output/'result') if args.reconstruct or args.reconstruct_recordings or args.export_project_bundle or args.archive_recordings or args.export_scan_project or args.fuse_sequence or args.import_mesh or args.recover_recording else None, 'software_rendering_requested': True, 'camera_device_fds': camera_fds,
                          'screenshot': 'window.png', 'binary_sha256': binary_sha256, 'app_pid': app.pid, 'private_display_window_geometry': windows, 'capture_interval_app_cpu_ticks': process_cpu_ticks(app.pid)-cpu_start, 'capture_interval_seconds': time.monotonic()-cpu_started, 'cpu_ticks_per_second': os.sysconf('SC_CLK_TCK'), 'app_alive_at_capture': True}
                (output/'capture.json').write_text(json.dumps(result, indent=2)+'\n')
                print(json.dumps(result))
        finally:
            stop(app)
            stop(server)


if __name__ == '__main__':
    main()
