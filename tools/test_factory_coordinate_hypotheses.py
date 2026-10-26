"""Synthetic coordinate-convention checks; no scanner or vendor inputs."""
import copy
import unittest

from compare_factory_coordinate_hypotheses import compare


def fixture():
    # A quarter-turn about Z, horizontal basis reflection, and distinct lens slots.
    lenses = [{"width": 100, "height": 80,
               "intrinsic_values": [50, 55, 30 + i, 39],
               "coefficient_values": [0.1, 0.2, 0.3, 0, 0, 0, 0.01, 0.02]}
              for i in range(3)]
    named_lenses = [{"width": 100, "height": 80,
                     "intrinsic_values": [50, 55, 69 - i, 39],
                     "brown_values": [0.1, 0.2, 0.3, 0, 0, 0, 0.01, -0.02]}
                    for i in range(3)]
    named = {"checksum_verified": True, "lenses": named_lenses,
             "second_times_inverse_first_raw_units":
             [[0, 1, 0, -10], [-1, 0, 0, 20], [0, 0, 1, 30], [0, 0, 0, 1]]}
    laser = {"checksum_verified": True, "records": [
        {"label": "synthetic", "lenses": lenses, "transforms": [
            {"matrix_values": [0, -1, 0, 1, 0, 0, 0, 0, 1],
             "translation_values": [10, 20, 30]}]}]}
    return named, laser


class ReflectionTests(unittest.TestCase):
    def test_known_basis_and_pixel_reflection(self):
        report = compare(*fixture())
        record = report["matching_full_size_records"][0]
        self.assertEqual(record["rotation_S_R_S_max_abs_difference"], 0)
        self.assertEqual(record["rotation_direct_max_abs_difference"], 2)
        self.assertEqual(record["translation_S_t_max_abs_difference_raw_units"], 0)
        self.assertEqual(record["translation_direct_max_abs_difference_raw_units"], 20)
        for lens in record["lens_slot_differences"]:
            self.assertEqual(lens["cx_horizontal_reflection_abs_difference_pixels"], 0)
            self.assertGreater(lens["cx_direct_abs_difference_pixels"], 30)
            self.assertEqual(lens["p2_horizontal_reflection_abs_difference"], 0)
            self.assertEqual(lens["p2_direct_abs_difference"], 0.04)
        for flag in ["physical_accuracy_verified", "translation_units_verified",
                     "image_orientation_verified", "camera_assignment_verified",
                     "reflection_is_a_proper_rotation"]:
            self.assertIs(report[flag], False)

    def test_cropped_record_is_not_bound_to_full_size(self):
        named, laser = fixture()
        cropped = copy.deepcopy(laser["records"][0])
        cropped["lenses"][0]["width"] = 90
        laser["records"].append(cropped)
        self.assertEqual(len(compare(named, laser)["matching_full_size_records"]), 1)

    def test_invalid_inputs_refused(self):
        for field in ["crc", "pose_shape", "pose_nonfinite", "lens_nonfinite", "rotation_shape"]:
            with self.subTest(field=field):
                named, laser = fixture()
                if field == "crc":
                    laser["checksum_verified"] = False
                elif field == "pose_shape":
                    named["second_times_inverse_first_raw_units"][0].pop()
                elif field == "pose_nonfinite":
                    named["second_times_inverse_first_raw_units"][0][0] = float("nan")
                elif field == "lens_nonfinite":
                    laser["records"][0]["lenses"][0]["intrinsic_values"][0] = float("inf")
                else:
                    laser["records"][0]["transforms"][0]["matrix_values"].pop()
                with self.assertRaises(ValueError):
                    compare(named, laser)


if __name__ == "__main__":
    unittest.main()
