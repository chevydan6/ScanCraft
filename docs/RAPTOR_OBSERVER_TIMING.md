# Raptor acquisition and host observer timing

**Confirmed finite result:** with the installed baseline driver unchanged, three
600-frame/camera captures give ongoing rejected buffers per camera of **4 → 0 → 3**
while a local session/power logger is running → temporarily stopped → restored.
All 3,600 saved frames replay. This supports investigating the observer as a
contributor; it does not isolate one operation or establish a complete fix.
IRQ placement differs between arms and startup failures persist.

## Lead and read-only reproduction

Reanalysis of the preceding inline-copy experiment finds a task named
`session-watch` scheduled at the start of the largest within-PTS completion gap
in all 14 left-camera paired incidents. Thirteen have the original monitor's
main PID; one has a different same-name task whose parent identity is not proven.
This is saved scheduler state, not a kernel stack or continuous-execution proof.

The inspected local logger records session/display/power events, listens to
D-Bus/journal events and polls display, backlight and laptop-lid state every two
seconds. It does not change power settings or inhibit sleep. A separate read-only
reproduction invokes its existing snapshot function eight times with timed read
wrappers, without starting another logging loop or modifying the original file.
The original observer remains running during that reproduction.

The slowest operation in all eight snapshots is
`/proc/acpi/button/lid/LID/state`: **1.382848–5.122096 ms** elapsed. Other file reads
in this sample are at most 0.256019 ms. These durations include scheduling and are
not isolated firmware/driver CPU times. Local Linux reference
`drivers/acpi/button.c::{acpi_button_state_seq_show,acpi_lid_evaluate_state}`
shows that reading this proc file evaluates the ACPI `_LID` method. In contrast,
`drivers/gpu/drm/drm_sysfs.c::status_show` reads cached connector status. Source
inspection alone does not attest the exact resident implementation or prove
which call caused a scanner failure. No independent hardware clock was measured.
A strace attempt failed because the executable is absent; no trace/attachment
result is claimed from it.

## Controlled observer comparison

Acquisition checkout `8a97cb2715d9e38dc4712bca1dd337b8dd84f08e` uses unchanged CLI
SHA-256 `be02dd6a39ecf18d451377517a19b3d3a03a1fe57fffd59a7e188ef958fecda0` and
installed baseline module srcversion `9D1275CDD5E013864E4C564`, five requests per
video endpoint. No experimental module is loaded, and no firmware, affinity,
power setting or original observer script is changed. Only the logging service
is briefly stopped for B1 and restarted during cleanup. Its active/running state
and unchanged script hash are verified after every arm.

The established collector retains ordinary locks/consumer checks, requested
30 FPS 1920 × 1200 GREY dual capture with temporary flood illumination, explicit
64 MiB USB-monitor ring, 32-byte transfer prefixes and complete xHCI/IRQ,
workqueue/scheduler/local-timer traces. It requests 128 MiB ftrace capacity per CPU
on 16 CPUs. All five idle camera formats, WirePlumber and global/private tracing
state are restored. Fourteen native state-field comparisons pass per arm.

| Arm | Observer | Startup rejects L/R | Ongoing rejects and sequence gaps L/R | Total rejection proxy |
| --- | --- | ---: | ---: | ---: |
| A1 | Running | 96 / 0 | 4 / 4 | 104/1304 = 7.975460% |
| B1 | Stopped for trial | 92 / 95 | 0 / 0 | 187/1387 = 13.482336% |
| A2 | Restored and running | 1 / 0 | 3 / 3 | 7/1207 = 0.579950% |

All complete proxies fail the below-0.1% target. B1's ongoing-only proxy is zero
for this finite sample; it is not zero startup loss, repeatability qualification,
physical exposure accounting or an application-wide reliability guarantee.

| Observation | A1 | B1 | A2 |
| --- | ---: | ---: | ---: |
| Complete USB events | 455,650 | 452,890 | 454,328 |
| Complete ftrace events | 4,089,085 | 3,702,585 | 3,704,304 |
| Strict matched accepted/ongoing-rejected associations | 1,208 | 1,200 | 1,206 |
| Camera-copy worker cycles | 227,686 | 226,248 | 227,075 |
| Effective IRQ affinity before → after | CPU4 → CPU4 | CPU11 → CPU11 | CPU9 → CPU10 |
| Actual IRQ39 trace placement | CPU4: 227,329 | CPU11: 225,791 | CPU9: 226,623 |

All identities/per-CPU counts balance with zero reported collector loss. Startup
still includes 188 unrecognized successful prefixes on A1 left and B1 each camera;
those are retained, not qualified. Affinity drift and differing actual CPU
placement are uncontrolled variables. The readbacks do not identify migration
time or cause. There is no affinity write and no isolated performance claim.

All four A1 paired rejects overlap 3.067–4.588 ms without selected host events,
with the observer main task scheduled at the gap start. A2's first two paired
rejects overlap 1.639/1.711 ms of such absence with the restored observer task.
The third A2 pair instead shows camera-copy work waiting up to 1.847 ms and
software-request starvation, without that task in the saved gap-start state.
Multiple failure mechanisms therefore remain plausible. B1 has no rejected
ongoing pair; one short control does not exclude rare worker delays or other loss.

## Admission, restoration and next experiment

The new private `admit.py` explicitly checks semantic JSON fields as well as
native replay, frame counts, collector completeness, checked state, trace cleanup,
module parameters and observer restoration. Regression against the preceding
inline A1 accepts it, while preceding inline B1 returns exit 1 for its strict
header and byte mismatches. The original evidence/analyzers are unchanged.
Each new arm passed this gate before advancing; it is diagnostic admission, not
physical accuracy or rejection-target qualification.

After all arms the original logger is active and its source hash is unchanged.
The baseline module/parameters remain unchanged; all 42 native state-field checks
pass. Saved kernel-log comparisons contain no new messages. The B1 final image
pair was extracted and viewed: board still clipped at the right image edge,
without a separately measured object. There is no new calibrated point cloud.

**Next experiment:** compare timed direct lid reads against cached-display reads
within one baseline-driver capture, with the original observer temporarily
stopped and restored. Predeclare read order/times, retain actual call intervals
and compare individual incidents with the complete trace. This can separate the
specific read from other observer work and reduce between-arm confounding.
No permanent observer change is justified by the current comparison alone.

Private `session-watch-timing-20261104` retains the source-bound timing probe,
saved-task correlation, source references and failed-tool note.
`observer-aba-20261104` retains plans, unchanged observer source, per-arm bindings,
original recordings, replay, full traces, semantic admission/regression, restoration,
resource measurements, analyzers and final images. No private logger source or
capture is published. Application/kernel code is unchanged; existing 890-test,
formatting, strict Clippy and release evidence applies. Both 8a97cb2 CI runs passed,
independently confirmed in assignment465. New observer evidence awaits independent
review; prior SCAN-REV-029's affinity correction is separately committed in f92cf09.
CodeRabbit quota lookup remains unavailable, not a completed review. Physical
scale, first measurable object scan, repeatability and whole-app qualification
remain outstanding.
