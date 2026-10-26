"""ScanCraft desktop application. No Wine or vendor DLL dependency."""
import fcntl
import json
import os
import sys
from datetime import datetime
from pathlib import Path
from PySide6.QtCore import Qt
from PySide6.QtGui import QPixmap
from PySide6.QtWidgets import (QApplication, QComboBox, QFileDialog, QHBoxLayout, QLabel,
    QMainWindow, QMessageBox, QPushButton, QPlainTextEdit, QVBoxLayout, QWidget)
from .capture import Camera
from .transport import discover


class Window(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle('ScanCraft — Native Linux')
        self.resize(1080, 800)
        self.camera = None
        self.session_lock = None
        self.devices = []
        self.setStyleSheet('QWidget {font-size: 14px;} QPushButton {padding: 9px;}')
        root = QWidget(); self.setCentralWidget(root); layout = QVBoxLayout(root)
        title = QLabel('ScanCraft'); title.setStyleSheet('font-size: 28px; font-weight: bold;')
        layout.addWidget(title)
        layout.addWidget(QLabel('Camera workbench • Live preview and PNG recording • 3D reconstruction is not implemented'))
        row = QHBoxLayout(); layout.addLayout(row)
        self.refresh = QPushButton('Find scanner'); self.refresh.clicked.connect(self.detect); row.addWidget(self.refresh)
        self.selector = QComboBox(); row.addWidget(self.selector, 1)
        self.start = QPushButton('Start preview'); self.start.clicked.connect(self.start_camera); row.addWidget(self.start)
        self.stop = QPushButton('Stop'); self.stop.clicked.connect(self.stop_camera); row.addWidget(self.stop)
        self.status = QLabel(); layout.addWidget(self.status)
        self.view = QLabel('Connect a Raptor, then start a camera preview.'); self.view.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.view.setMinimumSize(640, 400); self.view.setStyleSheet('background: #121c28; color: #d6e2ee; border-radius: 8px;')
        layout.addWidget(self.view, 1)
        row = QHBoxLayout(); layout.addLayout(row)
        self.snapshot = QPushButton('Save snapshot'); self.snapshot.clicked.connect(self.save_snapshot); row.addWidget(self.snapshot)
        self.record = QPushButton('Record preview'); self.record.clicked.connect(self.toggle_record); row.addWidget(self.record)
        layout.addWidget(QLabel('Recording: 640 × 400 RGB preview at up to 10 fps; host timestamps, no sensor synchronization.'))
        self.log = QPlainTextEdit(); self.log.setReadOnly(True); self.log.setMaximumHeight(120); self.log.setMaximumBlockCount(200)
        layout.addWidget(self.log)
        self.detect(); self.controls(False)

    def controls(self, running):
        self.refresh.setEnabled(not running); self.selector.setEnabled(not running)
        self.start.setEnabled(not running and self.selector.count() > 0)
        self.stop.setEnabled(running); self.snapshot.setEnabled(running); self.record.setEnabled(running)

    def detect(self):
        self.selector.clear()
        try:
            self.devices = discover()
            for device in self.devices:
                for node in device['nodes']:
                    if node.get('kind') == 'capture' and node.get('accessible'):
                        names = {0: 'Depth / left IR', 2: 'Right IR', 4: 'Color'}
                        self.selector.addItem(f"{names.get(node['interface'], 'Camera')} — {node['path']}", node)
            self.status.setText(f'{len(self.devices)} Raptor detected · ' + (f"USB {self.devices[0]['speed_mbps']} Mbps" if self.devices else 'Check power and USB cable'))
        except Exception as exc:
            self.status.setText(f'Detection failed: {exc}')
        self.controls(False)

    def acquire(self):
        # Share the bridge's lock on this workstation; also lock our own app.
        paths = [Path(os.environ.get('XDG_RUNTIME_DIR', '/tmp')) / f'scancraft-{os.getuid()}.lock']
        bridge = Path.home() / 'raptor-driver-research/wine-bridge/session.lock'
        if bridge.exists(): paths.append(bridge)
        handles = []
        try:
            for path in paths:
                handle = path.open('a'); handles.append(handle)
                fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError:
            for handle in handles: handle.close()
            raise RuntimeError('Another scanner session is running. Close Creality Scan or the other ScanCraft window first.')
        self.session_lock = handles

    def start_camera(self):
        try:
            self.acquire()
            node = self.selector.currentData()
            if not node: raise RuntimeError('No accessible camera selected.')
            self.camera = Camera(node['path'], node['interface'], self)
            self.camera.frame.connect(self.show_frame); self.camera.message.connect(self.log.appendPlainText)
            self.camera.ended.connect(self.capture_ended)
            self.camera.start(); self.controls(True)
            self.status.setText('Starting native V4L2 camera…')
        except Exception as exc:
            self.release(); QMessageBox.warning(self, 'Cannot start preview', str(exc))

    def show_frame(self, image):
        self.view.setPixmap(QPixmap.fromImage(image).scaled(self.view.size(), Qt.AspectRatioMode.KeepAspectRatio, Qt.TransformationMode.SmoothTransformation))
        self.status.setText(f'Live · {self.camera.frames} frames · {self.camera.node}')

    def release(self):
        if self.session_lock:
            for handle in self.session_lock: handle.close()
        self.session_lock = None

    def capture_ended(self):
        self.release(); self.controls(False); self.record.setText('Record preview')
        self.status.setText('Preview stopped. See diagnostics below if capture failed.')

    def stop_camera(self):
        if self.camera:
            self.camera.stop(); self.camera.deleteLater(); self.camera = None
        self.release(); self.controls(False)

    def save_snapshot(self):
        if not self.camera or self.camera.latest is None: return
        path, _ = QFileDialog.getSaveFileName(self, 'Save snapshot', str(Path.home() / 'raptor.png'), 'PNG image (*.png)')
        if path and not self.camera.latest.save(path): QMessageBox.warning(self, 'Save failed', 'Could not save the image.')

    def toggle_record(self):
        if not self.camera: return
        if self.camera.recording:
            self.camera.stop_recording(); self.record.setText('Record preview'); return
        path = QFileDialog.getExistingDirectory(self, 'Choose recording parent folder', str(Path.home()))
        if not path: return
        folder = Path(path) / datetime.now().strftime('scancraft-%Y%m%d-%H%M%S-%f')
        try:
            self.camera.record(folder)
            (folder / 'session.json').write_text(json.dumps({'node': self.camera.node, 'interface': self.camera.interface,
                'width': 640, 'height': 400, 'timestamp_clock': 'host_monotonic_ns', 'sensor_synchronized': False}, indent=2))
            self.record.setText('Stop recording'); self.log.appendPlainText(f'Recording to {folder}')
        except OSError as exc:
            self.camera.stop_recording(); QMessageBox.warning(self, 'Recording failed', str(exc))

    def closeEvent(self, event):
        self.stop_camera(); event.accept()


def main():
    app = QApplication(sys.argv); app.setApplicationName('ScanCraft')
    window = Window(); window.show(); sys.exit(app.exec())


if __name__ == '__main__': main()
