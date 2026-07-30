# 4. Unsynchronized read of `cursorHistory`

**Severity:** Medium
**Phase:** recording
**Status:** confirmed by source reading

---

## The race

`internal/recording/recorder.go:179`:

```go
func (r *Recorder) GetCursorHistory() []tracking.CursorPosition {
	return r.cursorHistory        // no lock
}
```

The writer is the polling goroutine in `internal/tracking/mouse.go`, guarded by a
mutex that is **function-local**:

```go
func StartMouseTracking(mouseEvents *[]CursorPosition, ...) {
	// Guards mouseEvents: the poll loop below and the click hook run on
	// separate goroutines and both append to the same slice.
	var mu sync.Mutex        // <-- line 17, local to this function
	...
			mu.Lock()
			*mouseEvents = append(*mouseEvents, mousePos)
			mu.Unlock()
```

`mu` is declared inside `StartMouseTracking` and captured only by the two
closures inside it. `GetCursorHistory()` — and therefore `main.go`'s
`editVideo()` — cannot acquire it. The result is an unsynchronized read of a
slice header (pointer, length, capacity) that another goroutine is concurrently
reassigning via `append`.

Consequences: a torn or stale slice header, a short read that loses the tail of
the recording, or — since `append` reallocates — a pointer to the old backing
array.

## Not what commit `8a248b3` fixed

`8a248b3` ("guard concurrent writes to mouseEvents in StartMouseTracking")
introduced the local mutex to fix **writer vs. writer**: the 60 Hz poll loop and
the `hook.MouseDown` callback both appending. That fix is correct for what it
targeted.

This is **writer vs. reader**, across a different boundary, and is still open.

## Fix direction

The mutex must live where both sides can reach it — on the `Recorder` struct
(reusing `r.mu`), or behind a channel that the tracker sends into and the
recorder drains. Passing `*[]CursorPosition` plus a private mutex is the shape
that makes this unfixable in place.

---

## 4b. Poll loop can append after `Stop()`

The polling loop checks `ctx.Done()` only at the top of each iteration and then
sleeps a full frame interval:

```go
time.Sleep(1 * time.Second / time.Duration(targetFPS))   // 16.7 ms at 60 fps
```

So after `Stop()` cancels the context, the goroutine can still append up to one
more sample, up to 16.7 ms late. Combined with 4a, `GetCursorHistory()` can be
racing a write that happens after the caller believes recording has ended.

---

## 4c. `hook.End()` is never called

`internal/tracking/mouse.go:74`:

```go
evChan := hook.Start()
// Start processing events. This blocks until hook.End() is called.
<-hook.Process(evChan)
```

Nothing in the codebase calls `hook.End()`. `StartMouseTracking` therefore never
returns and its goroutine leaks — one per recording. Worse, the registered
`hook.MouseDown` callback keeps a live reference to the **previous** recording's
slice, so clicks after a second recording starts append into the old history.
