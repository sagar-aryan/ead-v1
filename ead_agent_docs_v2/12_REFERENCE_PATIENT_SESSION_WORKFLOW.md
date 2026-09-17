# 12 — Patient, Reference, Calibration and Session Workflow

## 1. Patient record
Researcher creates/selects:
- patient name;
- patient ID.

Do not infer patient information from the device.

## 2. First reference capture
The patient/reference walker performs a short normal walking capture with **no haptic feedback**.

Rules:
- collect as many valid cycles as practical;
- require at least 30 valid cycles before allowing a reference to be finalized;
- 50–100+ valid cycles is preferred;
- reject corrupted/partial cycles;
- build a robust feature distribution using medians and MADs;
- store the resulting reference as a versioned entity.

## 3. Per-session calibration/reference check
At the start of each patient session:
1. perform the 5-second static sensor calibration if startup calibration is stale or hardware was remounted;
2. load the patient's saved reference profile;
3. run a **10-cycle reference check walk** with haptics disabled;
4. show quality and agreement with the stored profile;
5. if the researcher chooses `RECAPTURE_REFERENCE`, collect a new reference capture and create a new version; otherwise keep the stored reference locked.

This prevents the system from learning a bad gait from an error-heavy evaluation sequence.

## 4. Evaluation session
1. Select patient.
2. Select locked reference profile.
3. Enter researcher-defined segment step and error limits.
4. Confirm sensors healthy.
5. Start session.
6. Haptics become active only after the first valid cycle and confidence gating.
7. Continue until researcher stops, a fault occurs, storage is exhausted, or a segment threshold is reached.
8. If a segment limit is reached, close current segment and start the next segment automatically.

## 5. Segmentation
The researcher enters:
- `max_valid_cycles_per_segment`;
- `max_errors_per_segment`.

The segment closes when **either** threshold is reached first.
No fixed software default is invented if the researcher leaves it blank; the UI must require valid values before RUNNING.

## 6. Reference lock
During evaluation:
- reference data is read-only;
- no online adaptation;
- bad patient cycles cannot update the reference automatically.

## 7. Session completion
On stop:
- write session footer;
- flush all pending blocks;
- verify CRCs;
- transmit completion state;
- make export package available.
