//! Ankle angles from the device's foot and shank quaternions.
//!
//! The device estimates orientation; the dashboard turns the pair into the three
//! angles a reader can interpret. Doc 05 §11 names the features it needs as
//! "dorsiflexion-related", "plantarflexion-related" and "inversion/eversion-
//! related": an IMU on a shoe and a strap on a shin measures segment rotation,
//! not joint kinematics, and the wording is kept so nothing here reads as
//! clinical goniometry.
//!
//! The device will need the same decomposition for per-cycle features in M4;
//! that implementation goes in `ead_core`, and both are checked against the same
//! cases (DEC-013).

/// Q15 as sent in RAW_SAMPLE_BATCH: ±1.0 maps to ±32767.
pub fn from_q15(q: [i16; 4]) -> [f32; 4] {
    [
        q[0] as f32 / 32767.0,
        q[1] as f32 / 32767.0,
        q[2] as f32 / 32767.0,
        q[3] as f32 / 32767.0,
    ]
}

/// `q_relative = inverse(q_shank) * q_foot` (doc 04 §6): the foot's orientation
/// with respect to the shank, unaffected by a heading the two segments share.
pub fn relative(shank: [f32; 4], foot: [f32; 4]) -> [f32; 4] {
    let inverse = [shank[0], -shank[1], -shank[2], -shank[3]];
    let (a, b) = (inverse, foot);
    let mut out = [
        a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
        a[0] * b[1] + a[1] * b[0] + a[2] * b[3] - a[3] * b[2],
        a[0] * b[2] - a[1] * b[3] + a[2] * b[0] + a[3] * b[1],
        a[0] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[0],
    ];
    let length = (out.iter().map(|v| v * v).sum::<f32>()).sqrt();
    if length > 1e-9 {
        for value in &mut out {
            *value /= length;
        }
    } else {
        out = [1.0, 0.0, 0.0, 0.0];
    }
    out
}

/// The three angles, in degrees.
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize)]
pub struct AnkleAngles {
    /// Sagittal: positive is dorsiflexion-related (toes toward the shin),
    /// negative is plantarflexion-related.
    pub sagittal_deg: f32,
    /// Frontal: positive is inversion-related (sole turning medially).
    pub frontal_deg: f32,
    /// Transverse: positive is internal-rotation-related. Yaw has no gravity
    /// reference, so this one drifts; it is shown, never used as a feature.
    pub transverse_deg: f32,
}

/// Decomposes the relative orientation as Ry(sagittal) · Rx(frontal) ·
/// Rz(transverse), the order closest to the joint convention: flexion about the
/// shank's medial axis, then inversion about the floating anterior axis, then
/// rotation about the foot's long axis.
pub fn ankle_angles(relative: [f32; 4]) -> AnkleAngles {
    let [w, x, y, z] = relative;
    // Rows of the rotation matrix that the decomposition needs.
    let m02 = 2.0 * (x * z + w * y);
    let m10 = 2.0 * (x * y + w * z);
    let m11 = w * w - x * x + y * y - z * z;
    let m12 = 2.0 * (y * z - w * x);
    let m22 = w * w - x * x - y * y + z * z;

    let frontal = (-m12).clamp(-1.0, 1.0).asin();
    // Toes-up is a negative rotation about the medial +Y axis (TEST-027), so the
    // sign is flipped to make dorsiflexion positive.
    let sagittal = -m02.atan2(m22);
    let transverse = m10.atan2(m11);
    AnkleAngles {
        sagittal_deg: sagittal.to_degrees(),
        frontal_deg: frontal.to_degrees(),
        transverse_deg: transverse.to_degrees(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A rotation of `degrees` about one axis, as (w, x, y, z).
    fn about(axis: usize, degrees: f32) -> [f32; 4] {
        let half = degrees.to_radians() / 2.0;
        let mut q = [half.cos(), 0.0, 0.0, 0.0];
        q[axis + 1] = half.sin();
        q
    }

    #[test]
    fn identity_is_a_neutral_ankle() {
        let angles = ankle_angles([1.0, 0.0, 0.0, 0.0]);
        assert_eq!(angles, AnkleAngles::default());
    }

    #[test]
    fn toes_up_reads_as_positive_dorsiflexion() {
        // Raising the toes turns the foot negatively about the medial Y axis.
        let angles = ankle_angles(about(1, -20.0));
        assert!((angles.sagittal_deg - 20.0).abs() < 0.01, "{angles:?}");
        assert!(angles.frontal_deg.abs() < 0.01);
    }

    #[test]
    fn toes_down_reads_as_negative_plantarflexion() {
        let angles = ankle_angles(about(1, 25.0));
        assert!((angles.sagittal_deg + 25.0).abs() < 0.01, "{angles:?}");
    }

    #[test]
    fn rolling_the_sole_inward_reads_as_positive_inversion() {
        let angles = ankle_angles(about(0, 15.0));
        assert!((angles.frontal_deg - 15.0).abs() < 0.01, "{angles:?}");
        assert!(angles.sagittal_deg.abs() < 0.01);
    }

    #[test]
    fn a_shared_heading_leaves_the_angles_alone() {
        // Both segments yawed 40°: the relative orientation must not change.
        let yaw = about(2, 40.0);
        let foot_local = about(1, -12.0);
        let (a, b) = (yaw, foot_local);
        let foot = [
            a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
            a[0] * b[1] + a[1] * b[0] + a[2] * b[3] - a[3] * b[2],
            a[0] * b[2] - a[1] * b[3] + a[2] * b[0] + a[3] * b[1],
            a[0] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[0],
        ];
        let angles = ankle_angles(relative(yaw, foot));
        assert!((angles.sagittal_deg - 12.0).abs() < 0.01, "{angles:?}");
        assert!(angles.transverse_deg.abs() < 0.01);
    }

    #[test]
    fn q15_round_trips_within_a_count() {
        let q = from_q15([32767, 0, -16384, 0]);
        assert!((q[0] - 1.0).abs() < 1e-4);
        assert!((q[2] + 0.5).abs() < 1e-4);
    }

    #[test]
    fn an_all_zero_quaternion_does_not_produce_nan() {
        let angles = ankle_angles(relative([0.0; 4], [0.0; 4]));
        assert!(angles.sagittal_deg.is_finite() && angles.frontal_deg.is_finite());
    }
}
