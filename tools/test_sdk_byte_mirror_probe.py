"""Hardware-free rejection checks; no proprietary fixture is required."""
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import probe_sdk_byte_mirror as probe


class ReferenceGuards(unittest.TestCase):
    def test_unknown_contents_rejected_before_compilation_or_execution(self):
        with tempfile.TemporaryDirectory() as root:
            source = Path(root) / "synthetic.dll"
            for contents in (b"", b"MZ", b"not an SDK"):
                source.write_bytes(contents)
                with mock.patch.object(probe.subprocess, "run") as run:
                    with self.assertRaisesRegex(ValueError, "SDK hash differs"):
                        probe.probe(source)
                    run.assert_not_called()

    def test_symlink_and_fifo_are_rejected_without_opening_a_device(self):
        with tempfile.TemporaryDirectory() as root:
            target = Path(root) / "target"
            target.write_bytes(b"synthetic")
            link = Path(root) / "link"
            link.symlink_to(target)
            with self.assertRaises(OSError):
                probe.extract_leaf(link)
            fifo = Path(root) / "fifo"
            os.mkfifo(fifo)
            with self.assertRaisesRegex(ValueError, "bounded regular file"):
                probe.extract_leaf(fifo)

    def test_oversize_sparse_file_rejected(self):
        with tempfile.TemporaryDirectory() as root:
            source = Path(root) / "large"
            with source.open("wb") as stream:
                stream.truncate(probe.MAX_DLL_BYTES + 1)
            with self.assertRaisesRegex(ValueError, "bounded regular file"):
                probe.extract_leaf(source)


if __name__ == "__main__":
    unittest.main()
