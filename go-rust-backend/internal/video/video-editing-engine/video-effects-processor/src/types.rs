#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CPoint {
    pub x: f32,
    pub y: f32,
    pub timestamp_ms: f64,
}

/// Extended point type with velocity information from spring physics.
/// Used internally for Hermite interpolation between frames.
#[derive(Clone, Copy, Debug)]
pub struct PathPoint {
    pub x: f64,
    pub y: f64,
    pub timestamp_ms: f64,
    /// Velocity in x direction (pixels per second)
    pub vx: f64,
    /// Velocity in y direction (pixels per second)
    pub vy: f64,
}

impl PathPoint {
    /// Create a new PathPoint with zero velocity
    pub fn new(x: f64, y: f64, timestamp_ms: f64) -> Self {
        Self {
            x,
            y,
            timestamp_ms,
            vx: 0.0,
            vy: 0.0,
        }
    }

    /// Create a PathPoint with specified velocity
    pub fn with_velocity(x: f64, y: f64, timestamp_ms: f64, vx: f64, vy: f64) -> Self {
        Self {
            x,
            y,
            timestamp_ms,
            vx,
            vy,
        }
    }

    /// Convert to CPoint (loses velocity information)
    pub fn to_cpoint(&self) -> CPoint {
        CPoint {
            x: self.x as f32,
            y: self.y as f32,
            timestamp_ms: self.timestamp_ms,
        }
    }
}

impl From<CPoint> for PathPoint {
    fn from(p: CPoint) -> Self {
        PathPoint::new(p.x as f64, p.y as f64, p.timestamp_ms)
    }
}

/// Legacy smoothed path result for backward compatibility.
/// Used by the deprecated `smooth_cursor_path` function when `legacy-api` feature is enabled.
#[repr(C)]
#[allow(dead_code)]
pub struct CSmoothedPath {
    pub points: *mut CPoint,
    pub len: usize,
}

/// Video processing configuration passed from FFI.
///
/// # Perceptual Parameters
/// - `responsiveness`: 0.0 = smooth/slow tracking, 1.0 = snappy/immediate response
/// - `smoothness`: 0.0 = may overshoot slightly, 1.0 = no overshoot, gradual motion
///
/// These map internally to spring physics parameters (stiffness, damping).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VideoProcessingConfig {
    /// Catmull-Rom alpha: 0.5 for centripetal (recommended)
    pub smoothing_alpha: f32,
    /// How quickly the cursor responds to target changes (0.0-1.0)
    /// 0.0 = slow, floaty tracking (~400ms settling)
    /// 1.0 = snappy, immediate tracking (~60ms settling)
    pub responsiveness: f32,
    /// How smooth/damped the motion is (0.0-1.0)
    /// 0.0 = slight overshoot allowed (zeta=0.7)
    /// 1.0 = no overshoot, very smooth (zeta=1.5)
    pub smoothness: f32,
    /// Video frame rate (e.g., 60)
    pub frame_rate: i32,
    /// Log verbosity level: 0=off, 1=error, 2=warn, 3=info, 4=debug, 5=trace
    pub log_level: i32,
}

impl Default for VideoProcessingConfig {
    fn default() -> Self {
        Self {
            smoothing_alpha: 0.5,
            responsiveness: 0.5,  // Balanced default
            smoothness: 0.7,      // Mostly smooth with minimal overshoot
            frame_rate: 60,
            log_level: 3,         // Info level by default
        }
    }
}

pub type ProgressCallback = extern "C" fn(percent: f32);



