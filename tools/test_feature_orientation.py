import unittest
import numpy as np
from check_factory_epipolar import camera_order_pixels


class CameraOrderTests(unittest.TestCase):
    def test_swapping_assigns_intrinsic_slots_and_preserves_pairs(self):
        a = np.array([[100., 200.], [300., 400.]])
        b = np.array([[110., 210.], [310., 410.]])
        first, second = camera_order_pixels([a, b], True, True)
        np.testing.assert_array_equal(first, [[1809, 210], [1609, 410]])
        np.testing.assert_array_equal(second, [[1819, 200], [1619, 400]])
        restored = camera_order_pixels([first, second], True, True)
        np.testing.assert_array_equal(restored[0], a)
        np.testing.assert_array_equal(restored[1], b)
        np.testing.assert_array_equal(a, [[100, 200], [300, 400]])

    def test_domains_and_shapes(self):
        valid = np.array([[0., 0.], [1919., 1199.]])
        np.testing.assert_array_equal(camera_order_pixels([valid, valid], False, True)[0],
                                      [[1919, 0], [0, 1199]])
        for bad in [np.array([[1920., 0.], [0., 0.]]),
                    np.array([[0., 1200.], [0., 0.]]),
                    np.array([[float("nan"), 0.], [0., 0.]]),
                    np.array([[1., 2.]]), np.array([1., 2.])]:
            with self.assertRaises(ValueError):
                camera_order_pixels([valid, bad])


if __name__ == "__main__":
    unittest.main()
