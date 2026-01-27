#ifndef VIDEO_EDITING_ENGINE_H
#define VIDEO_EDITING_ENGINE_H

#include <stddef.h>
#include <stdint.h>

// Point structure matching Rust's CPoint
typedef struct {
  float x;
  float y;
  double timestamp_ms;
} CPoint;

// Smoothed path result
typedef struct {
  CPoint *points;
  size_t len;
} CSmoothedPath;

// Video processing configuration
typedef struct {
  float smoothing_alpha;  // 0.5 for centripetal Catmull-Rom (recommended)
  float responsiveness;   // 0.0 = slow/floaty, 1.0 = snappy/immediate (0-1)
  float smoothness;       // 0.0 = slight overshoot, 1.0 = no overshoot (0-1)
  int32_t frame_rate;     // Video frame rate (e.g., 60)
  int32_t log_level;      // 0=off, 1=error, 2=warn, 3=info, 4=debug, 5=trace
} VideoProcessingConfig;

// Progress callback function pointer type
typedef void (*ProgressCallback)(float percent);

// ============================================================================
// Main API - Unified Video Processing
// ============================================================================

/**
 * Process video with cursor smoothing and overlay in one call.
 *
 * Returns:
 *   0: Success
 *  -1: Null pointer argument
 *  -2: Invalid UTF-8 in path
 *  -3: Cursor path smoothing error
 *  -4: Video rendering error
 */
int32_t process_video_with_cursor(
    const char *input_video_path, const char *output_video_path,
    const char *cursor_sprite_path, const CPoint *raw_cursor_points,
    size_t raw_cursor_points_len, const VideoProcessingConfig *config,
    ProgressCallback progress_callback // Can be NULL
);

// ============================================================================
// Legacy API (for backward compatibility)
// ============================================================================

/**
 * Smooth cursor path using Catmull-Rom splines.
 * Caller must free result with free_smoothed_path().
 */
CSmoothedPath smooth_cursor_path(const CPoint *raw_points_ptr,
                                 size_t raw_points_len,
                                 const int64_t *points_per_segment_ptr,
                                 size_t points_per_segment_len, float alpha,
                                 float tension, float friction, float mass);

/**
 * Free memory allocated by smooth_cursor_path.
 */
void free_smoothed_path(CSmoothedPath path);

#endif // VIDEO_EDITING_ENGINE_H
