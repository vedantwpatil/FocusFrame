//! Documented constants for cursor smoothing algorithms.
//!
//! These values are tuned for typical desktop screen recording scenarios
//! where smooth cursor motion is desired without losing responsiveness.

#![allow(dead_code)] // Some constants are for documentation/future use

// ============================================================================
// Motion Kinematics Constants
// ============================================================================

/// Maximum cursor velocity in pixels per second.
///
/// **Rationale**: On a 1920px-wide display, moving the cursor from one edge
/// to the other in ~1 second requires ~1920 px/s. We allow slightly lower to
/// ensure the spring can keep up. 1800 px/s corresponds to crossing a
/// Full HD screen in ~1.07 seconds, which feels natural for smooth motion.
///
/// This also aligns with typical mouse DPI settings (800-1600 DPI) where
/// fast but controlled movements fall within this range.
pub const VELOCITY_MAX_PX_PER_SEC: f64 = 1800.0;

/// Maximum cursor acceleration in pixels per second squared.
///
/// **Rationale**: Human hand movements during mouse control typically produce
/// accelerations in the 2000-6000 px/s² range. We use 4000 px/s² as a balanced
/// value that:
/// - Allows snappy responses to direction changes
/// - Prevents jarring, unrealistic motion
/// - Keeps the trapezoidal speed profiles from appearing jerky
///
/// At 4000 px/s², reaching max velocity (1800 px/s) takes 0.45 seconds,
/// which provides a smooth ramp-up period.
pub const ACCELERATION_MAX_PX_PER_SEC2: f64 = 4000.0;

// ============================================================================
// Spring Physics Constants
// ============================================================================

/// Default settling time for spring physics in seconds.
///
/// **Rationale**: 120ms (0.12s) aligns with UI animation best practices:
/// - iOS/macOS system animations: 100-300ms
/// - Material Design "standard easing": 300ms for complex, 200ms for simple
/// - CSS transition defaults: 250ms
///
/// 120ms provides snappy tracking while maintaining smooth motion. This is
/// the time for the spring to reach ~98% of the target position (using
/// standard 2nd-order system settling time ≈ 4/(ζ·ωn)).
pub const SPRING_SETTLING_TIME_SEC: f64 = 0.12;

/// Default damping ratio (zeta) for critically damped behavior.
///
/// **Rationale**: A damping ratio of 1.0 produces critically damped motion:
/// - No overshoot (important for precise cursor positioning)
/// - Fastest approach to target without oscillation
///
/// Values < 1.0 cause oscillation (underdamped)
/// Values > 1.0 cause sluggish response (overdamped)
pub const SPRING_DAMPING_RATIO_DEFAULT: f64 = 1.0;

/// Default spring mass for physics simulation.
///
/// **Rationale**: The mass value is normalized to 1.0 as it only affects
/// the relationship between stiffness (k) and damping (c). With m=1.0:
/// - ωn = sqrt(k/m) = sqrt(k)
/// - c = 2·ζ·ωn·m = 2·ζ·sqrt(k)
///
/// This simplifies tuning since users think in terms of settling time
/// and damping behavior, not absolute mass values.
pub const SPRING_MASS_DEFAULT: f64 = 1.0;

// ============================================================================
// Timestamp Alignment Constants
// ============================================================================

/// Number of frames to sample for robust timestamp alignment.
///
/// **Rationale**: Video codecs may produce irregular timestamps on early
/// frames due to:
/// - B-frame reordering (decode order ≠ presentation order)
/// - Variable frame timing at stream start
/// - Dropped frames during capture initialization
///
/// Using 10 frames provides enough samples to compute a robust median
/// while not delaying alignment significantly (~166ms at 60fps).
pub const ALIGNMENT_SAMPLE_COUNT: usize = 10;

/// Maximum time window (in ms) to collect alignment samples.
///
/// **Rationale**: 500ms ensures we don't wait forever on slow video sources
/// while still collecting enough samples for a robust alignment offset.
/// This covers 30 frames at 60fps or 15 frames at 30fps.
pub const ALIGNMENT_MAX_WINDOW_MS: f64 = 500.0;

// ============================================================================
// Interpolation Constants
// ============================================================================

/// Default Catmull-Rom alpha parameter for centripetal spline.
///
/// **Rationale**: Alpha controls the "tightness" of the spline:
/// - 0.0: Uniform (can produce cusps and loops)
/// - 0.5: Centripetal (no cusps, most natural for motion paths)
/// - 1.0: Chordal (follows control points more closely)
///
/// 0.5 is the standard choice for motion smoothing as proven by
/// Yuksel et al. "On the Parameterization of Catmull-Rom Curves".
pub const CATMULL_ROM_ALPHA_CENTRIPETAL: f32 = 0.5;

// ============================================================================
// Perceptual Parameter Mapping
// ============================================================================

/// Minimum settling time for "fully responsive" (responsiveness = 1.0).
///
/// **Rationale**: 60ms is approximately the limit of human perception for
/// motion continuity. Below this, faster tracking provides no perceptible
/// benefit but may introduce numerical instability.
pub const RESPONSIVENESS_MAX_SETTLING_SEC: f64 = 0.06;

/// Maximum settling time for "fully smooth" (responsiveness = 0.0).
///
/// **Rationale**: 400ms matches longer UI animations and provides very
/// smooth, almost "floaty" cursor motion. Beyond this, latency becomes
/// noticeable and the cursor feels disconnected from user input.
pub const RESPONSIVENESS_MIN_SETTLING_SEC: f64 = 0.40;

/// Minimum damping ratio for "no smoothness" (smoothness = 0.0).
///
/// **Rationale**: At ζ=0.7, slight overshoot occurs but motion remains
/// controlled. This is the boundary of acceptable damping for cursor
/// tracking without appearing oscillatory.
pub const SMOOTHNESS_MIN_DAMPING: f64 = 0.7;

/// Maximum damping ratio for "full smoothness" (smoothness = 1.0).
///
/// **Rationale**: At ζ=1.5, the system is overdamped, producing very
/// smooth, gradual motion without any overshoot. Higher values would
/// make the cursor feel sluggish.
pub const SMOOTHNESS_MAX_DAMPING: f64 = 1.5;
