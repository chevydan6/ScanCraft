"""Native V4L2 capture using FFmpeg. Preview timestamps are host timestamps."""
import json
import time
from pathlib import Path
from PySide6.QtCore import QObject, QProcess, Signal
from PySide6.QtGui import QImage

WIDTH, HEIGHT = 640, 400
FRAME_BYTES = WIDTH * HEIGHT * 3


def arguments(node, interface):
    color = interface == 4
    return ['-hide_banner', '-loglevel', 'warning', '-nostdin', '-f', 'v4l2',
            '-input_format', 'mjpeg' if color else 'gray', '-video_size',
            '1280x800' if color else '1920x1200', '-framerate', '10' if color else '50',
            '-i', node, '-an', '-vf', f'scale={WIDTH}:{HEIGHT},fps=10',
            '-pix_fmt', 'rgb24', '-threads', '1', '-f', 'rawvideo', 'pipe:1']


class Camera(QObject):
    frame = Signal(object)
    message = Signal(str)
    ended = Signal()

    def __init__(self, node, interface, parent=None):
        super().__init__(parent)
        self.node, self.interface = node, interface
        self.buffer = bytearray()
        self.latest = None
        self.frames = 0
        self.recording = None
        self.process = QProcess(self)
        self.process.readyReadStandardOutput.connect(self.read)
        self.process.readyReadStandardError.connect(self.errors)
        self.process.errorOccurred.connect(lambda _: self.message.emit(self.process.errorString()))
        self.process.finished.connect(self.finished)

    def start(self):
        self.process.start('ffmpeg', arguments(self.node, self.interface))

    def read(self):
        self.buffer.extend(bytes(self.process.readAllStandardOutput()))
        while len(self.buffer) >= FRAME_BYTES:
            raw = bytes(self.buffer[:FRAME_BYTES])
            del self.buffer[:FRAME_BYTES]
            image = QImage(raw, WIDTH, HEIGHT, WIDTH * 3, QImage.Format.Format_RGB888).copy()
            self.latest = image
            self.frames += 1
            if self.recording:
                folder, manifest, count = self.recording
                name = f'{count:08d}.png'
                if image.save(str(folder / name)):
                    manifest.write(json.dumps({'file': name, 'host_monotonic_ns': time.monotonic_ns()}) + '\n')
                    self.recording = (folder, manifest, count + 1)
                else:
                    self.message.emit('Recording failed: unable to save frame; check free disk space.')
                    self.stop_recording()
            self.frame.emit(image)

    def errors(self):
        msg = bytes(self.process.readAllStandardError()).decode(errors='replace').strip()
        if msg:
            self.message.emit(msg[-2000:])

    def record(self, folder):
        folder = Path(folder)
        folder.mkdir(parents=True, exist_ok=False)
        manifest = (folder / 'frames.jsonl').open('x')
        self.recording = (folder, manifest, 0)

    def stop_recording(self):
        if self.recording:
            self.recording[1].close()
            self.recording = None

    def finished(self, code, status):
        self.stop_recording()
        self.message.emit(f'Capture stopped (exit {code}); {self.frames} preview frames received.')
        self.ended.emit()

    def stop(self):
        self.process.terminate()
        if not self.process.waitForFinished(2000):
            self.process.kill()
            self.process.waitForFinished(1000)
        self.stop_recording()
