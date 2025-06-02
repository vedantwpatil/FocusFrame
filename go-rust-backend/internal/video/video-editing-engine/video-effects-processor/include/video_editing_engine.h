#ifndef VIDEO_EDITING_ENGINE_H
#define VIDEO_EDITING_ENGINE_H

#include <stddef.h> // For size_t
#include <stdint.h> // For int64_t

// Matches the CPoint struct in Rust and how Go constructs it.
typedef struct {
  float x;
  float y;
  double timestamp_ms;
} CPoint;

typedef struct {
  CPoint *points;
  size_t len;
} CSmoothedPath;

// Updated FFI function signature
CSmoothedPath smooth_cursor_path(
    const CPoint *raw_points_ptr, size_t raw_points_len,
    const int64_t *points_per_segment_ptr, // Pointer to array of counts
    size_t points_per_segment_len,         // Length of that array
    float alpha,                           // Catmull-Rom alpha
    float tension, float friction, float mass);

void free_smoothed_path(CSmoothedPath path);

#endif // VIDEO_EDITING_ENGINE_H
