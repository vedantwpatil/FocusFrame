package video

import (
	"context"
)

type VideoSegment struct {
	Path      string
	StartTime float64
	EndTime   float64
	Metadata  map[string]any
}

type ProgressReporter interface {
	Report(progress float64)
	ReportError(err error)
	ReportComplete()
}

type ProcessingError struct {
	Stage    string
	Error    error
	Recovery func() error
}

type Effect interface {
	Apply(ctx context.Context, input VideoSegment) (VideoSegment, error)
	Validate() error
	GetName() string
	GetProcessedSegment() VideoSegment
	SetProcessedSegment(segment VideoSegment)
}
