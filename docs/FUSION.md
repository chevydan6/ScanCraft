# Bounded voxel cloud fusion

`scancraft_geometry::fusion::VoxelFusion` accumulates accepted aligned point clouds
in a common millimeter world frame. This is voxel-centroid point-cloud fusion,
not TSDF integration, a mesh, or qualified live scanner reconstruction.

Create it with an explicit positive finite voxel size and capacity of 1..1M
voxels. Each frame has 1..100,000 points and a strictly increasing frame ID.
Positions are transformed by an explicit finite proper rigid pose, then keyed
by floor(world position / voxel size). Negative coordinates use mathematical
floor. Out-of-range signed 64-bit keys fail rather than saturating. A voxel
retains its running centroid and observation count; output has deterministic
lexicographic voxel order. Every point contributes equal weight; there is no
measurement-confidence weighting or per-view uncertainty model.

Present colors have independent observation counts, so missing color does not
become black. RGB channels use arithmetic means of stored 8-bit values and
round on output; this is not radiometric/linear-light color fusion. Nonzero
input normals are normalized and rotated without translation. Present normals
are averaged independently and renormalized for output; opposing contributions
can cancel, yielding no normal. No automatic orientation correction is applied.

A frame stages only its affected voxels. Validation errors, index overflow,
capacity exhaustion or cooperative cancellation before commit leave the old
model and last frame ID unchanged. No points are silently evicted to fit capacity.
Commit applies the validated staging map after a final cancellation check and
has no fallible operation; cancellation after that boundary takes effect on a
later operation. Staging contains at most 100,000 voxels and the persistent map
never exceeds its configured capacity. Allocation failure remains governed by
Rust's process allocator; this is not transactional recovery from process death.

`integrate_tracked` accepts only tracking updates explicitly marked accepted
and Tracking, with a pose. A rejected/lost update cannot mutate the model.
This is an API consistency guard, not a trust boundary: callers can construct
updates and are responsible for correct source correspondence, session identity
and calibrated units. Do not reuse an old fusion map after resetting/re-anchoring
a tracker into a new world. Independent `integrate` also supports manually
validated alignments. The current API does not transact tracker acceptance and
fusion together: if fusion fails after tracking accepted a pose, callers must
report that frame was not integrated rather than assuming the model advanced.

`cloud` returns one centroid per voxel for existing PLY export/editor workflows.
It does not preserve raw observations, trajectory, uncertainty, fusion weights
or resumable accumulator state. Preserve original raw recordings separately;
exported clouds are derived representations. Voxel averaging can remove small
features and is not an approved accuracy-preserving parameter choice without
physical testing. No GUI/live acquisition wiring or performance qualification
is claimed.

Three Rust regressions cover analytical centroids, signed keys, color and normal
handling, rigid transformation, atomic failure/cancellation/capacity behavior,
frame ordering, and a ten-frame tracker-to-fusion synthetic sequence with world
positions checked within 1e-10 mm. A lost-tracking frame leaves the model
unchanged. Derived cloud PLY round-trip is exact. These are synthetic numerical
checks, not physical precision measurements. All 173 workspace tests, Clippy,
formatting and release workspace build pass; evidence in fusion test/build logs.

Next: couple frame acceptance and integration in a session workflow, add explicit
integration status and recorded cloud-sequence replay, then qualify real
calibrated data and live throughput before enabling continuous scanning.

The subsequent `ScanSession` controller now provides coupled tracking/fusion
publication and explicit lifecycle; `fuse-sequence` exercises it on offline
cloud files. Use that controller to avoid independently advancing the tracker
when integration fails. See `SCAN_SESSIONS.md`; camera wiring and physical
qualification remain unfinished.
