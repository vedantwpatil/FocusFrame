#ifndef VIDEO_EDITING_ENGINE_H
#define VIDEO_EDITING_ENGINE_H

#include <stddef.h> // For size_t
#include <stdint.h>
#include <stdio.h>

typedef struct {
  double x;
  double y;
  int64_t timestamp_ms;
} CPoint;

typedef struct {
  CPoint *points;
  size_t len;
} CSmoothedPath;

CSmoothedPath smooth_cursor_path(const CPoint *raw_points, size_t num_points,
                                 double tension, double friction, double mass);
void free_smoothed_path(CSmoothedPath path);
int32_t add(int32_t a, int32_t b);
void greet_from_rust(void);

#endif
