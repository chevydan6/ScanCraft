#!/usr/bin/env python3
"""Offline exploratory epipolar check on private, metadata-backed IR images.
This is an OpenCV reference diagnostic, not production reconstruction or
physical qualification. Matches are image-only; factory geometry does not
influence feature matching or the fundamental-matrix RANSAC comparison.
"""
import argparse
import hashlib
import json
from pathlib import Path
import cv2
import numpy as np


def camera_order_pixels(pixels, swap_camera_inputs=False, mirror_x=False):
    """Map image-only correspondences to lens slots without changing their pairing."""
    mapped = [np.array(p, dtype=np.float64, copy=True) for p in pixels]
    if len(mapped) != 2 or any(p.ndim != 2 or p.shape[1] != 2 or not np.isfinite(p).all()
                               for p in mapped) or mapped[0].shape != mapped[1].shape:
        raise ValueError("requires two equally sized finite Nx2 correspondence arrays")
    if any(np.any(p < 0) or np.any(p[:, 0] > 1919) or np.any(p[:, 1] > 1199) for p in mapped):
        raise ValueError("correspondence outside full-size pixel-center domain")
    if mirror_x:
        for p in mapped:
            p[:, 0] = 1919 - p[:, 0]
    return mapped[::-1] if swap_camera_inputs else mapped


def check(factory_path, left_dir, right_dir, laser_record_index=None,
          swap_camera_inputs=False, mirror_x=False):
    factory = json.loads(Path(factory_path).read_text())
    if factory.get("checksum_verified") is not True:
        raise ValueError("requires CRC-verified native CLI decoded factory fields")
    scope = {"factory_source": "named-camera fields"}
    if laser_record_index is not None:
        records = factory["records"]
        if not 0 <= laser_record_index < len(records):
            raise ValueError("laser record index outside decoded records")
        record = records[laser_record_index]
        if any((lens["width"], lens["height"]) != (1920, 1200)
               for lens in record["lenses"][:2]):
            raise ValueError("laser hypothesis requires two full-size IR lens slots")
        transform = np.eye(4)
        transform[:3, :3] = np.array(record["transforms"][0]["matrix_values"]).reshape(3, 3)
        transform[:3, 3] = record["transforms"][0]["translation_values"]
        factory = {
            "lenses": [{"intrinsic_values": lens["intrinsic_values"],
                        "brown_values": lens["coefficient_values"]}
                       for lens in record["lenses"]],
            "second_times_inverse_first_raw_units": transform.tolist(),
        }
        scope = {"factory_source": "laser record fields", "record_index": laser_record_index,
                 "coefficient_order_is_hypothesis": True, "camera_assignment_is_hypothesis": True,
                 "mode_binding_verified": False}
    dirs = [Path(left_dir), Path(right_dir)]
    manifests = [json.loads((p / "manifest.json").read_text()) for p in dirs]
    if manifests[0]["sensor_pts"] != manifests[1]["sensor_pts"]:
        raise ValueError("reference diagnostic requires identical saved sensor PTS")
    if [m["profile"]["interface"] for m in manifests] != [0, 2]:
        raise ValueError("requires left/right interface 0/2 in that order")
    cv2.setNumThreads(2)
    sift = cv2.SIFT_create(nfeatures=3000, contrastThreshold=0.01)
    images, keypoints, descriptors = [], [], []
    for path, manifest in zip(dirs, manifests):
        image = cv2.imread(str(path / "image.png"), cv2.IMREAD_GRAYSCALE)
        if image is None or image.shape != (1200, 1920):
            raise ValueError("requires genuine 1920x1200 IR image")
        if (manifest["profile"]["height"], manifest["profile"]["width"]) != image.shape:
            raise ValueError("image/manifest dimensions differ")
        images.append(image)
        enhanced = cv2.createCLAHE(clipLimit=2.0, tileGridSize=(8, 8)).apply(image)
        keypoint, descriptor = sift.detectAndCompute(enhanced, None)
        keypoints.append(keypoint)
        descriptors.append(descriptor)
    pairs = []
    if all(d is not None and len(d) >= 2 for d in descriptors):
        matcher = cv2.BFMatcher(cv2.NORM_L2)
        def ratio(a, b):
            return {m.queryIdx: m.trainIdx for row in matcher.knnMatch(a, b, k=2)
                    if len(row) == 2 for m, n in [row] if m.distance < 0.7 * n.distance}
        forward = ratio(*descriptors)
        reverse = ratio(*reversed(descriptors))
        pairs = [(a, b) for a, b in forward.items() if reverse.get(b) == a]
    report = {
        "schema_version": 1, "exploratory_only": True,
        "feature_method": "SIFT after CLAHE, mutual ratio 0.7; geometry-independent",
        "opencv_version": cv2.__version__, "keypoints": [len(k) for k in keypoints],
        "mutual_matches": len(pairs), "sensor_pts_equal": True,
        "host_timestamp_difference_us": abs(manifests[0]["host_timestamp_us"] - manifests[1]["host_timestamp_us"]),
        "synchronization_verified": False, "physical_accuracy_verified": False,
        "image_sha256": [hashlib.sha256((p / "image.png").read_bytes()).hexdigest() for p in dirs],
        "intensity_percentiles": [np.percentile(im, [0, 50, 99, 100]).tolist() for im in images],
        "factory_epipolar_hypotheses": {}, "geometry_qualified": False,
        **scope,
    }
    report["camera_input_interfaces_in_lens_order"] = [2, 0] if swap_camera_inputs else [0, 2]
    report["both_observation_sets_reflected_x"] = mirror_x
    report["camera_orientation_assignment_verified"] = False
    report["matching_unchanged_by_camera_hypothesis"] = True
    if not pairs:
        return report
    pixels = [np.array([keypoints[i][pair[i]].pt for pair in pairs], dtype=np.float64) for i in range(2)]
    pixels = camera_order_pixels(pixels, swap_camera_inputs, mirror_x)
    rays = []
    for i in range(2):
        lens = factory["lenses"][i]
        fx, fy, cx, cy = lens["intrinsic_values"]
        k1, k2, k3, k4, k5, k6, p1, p2 = lens["brown_values"]
        if any(v != 0 for v in [k4, k5, k6]):
            raise ValueError("this reference check only supports zero rational denominator coefficients")
        K = np.array([[fx, 0, cx], [0, fy, cy], [0, 0, 1]], dtype=np.float64)
        undistorted = cv2.undistortPoints(pixels[i].reshape(-1, 1, 2), K, np.array([k1, k2, p1, p2, k3]))
        rays.append(np.column_stack((undistorted.reshape(-1, 2), np.ones(len(pairs)))))
    base = np.array(factory["second_times_inverse_first_raw_units"], dtype=np.float64)
    scale = sum(l["intrinsic_values"][0] for l in factory["lenses"][:2]) / 2
    for name, transform in [("stored_order", base), ("inverse", np.linalg.inv(base))]:
        t, r = transform[:3, 3], transform[:3, :3]
        skew = np.array([[0, -t[2], t[1]], [t[2], 0, -t[0]], [-t[1], t[0], 0]])
        E = skew @ r
        lines_right = rays[0] @ E.T
        lines_left = rays[1] @ E
        numerator = np.abs(np.sum(rays[1] * lines_right, axis=1))
        denom = np.sqrt(np.sum(lines_right[:, :2] ** 2, axis=1) + np.sum(lines_left[:, :2] ** 2, axis=1))
        errors = numerator / np.maximum(denom, 1e-30) * scale
        report["factory_epipolar_hypotheses"][name] = {
            "approx_pixel_sampson_median": float(np.median(errors)),
            "approx_pixel_sampson_p95": float(np.percentile(errors, 95)),
            "under_2_approx_pixels": int(np.count_nonzero(errors < 2)),
        }
    if len(pairs) >= 8:
        cv2.setRNGSeed(0)
        _, mask = cv2.findFundamentalMat(pixels[0], pixels[1], cv2.FM_RANSAC, 2.0, 0.999)
        report["image_only_ransac_inliers"] = int(np.count_nonzero(mask)) if mask is not None else 0
    return report


if __name__ == "__main__":
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("factory_json")
    p.add_argument("left_frame_directory")
    p.add_argument("right_frame_directory")
    p.add_argument("--laser-record-index", type=int,
                   help="Test a raw laser-record hypothesis; camera/coefficients/mode remain unverified")
    p.add_argument("--swap-camera-inputs", action="store_true",
                   help="Assign interface 2 to lens slot 1 and interface 0 to slot 2")
    p.add_argument("--mirror-x", action="store_true",
                   help="Reflect matched observations only as width-1-x")
    args = p.parse_args()
    print(json.dumps(check(args.factory_json, args.left_frame_directory, args.right_frame_directory,
                           args.laser_record_index, args.swap_camera_inputs, args.mirror_x), indent=2))
