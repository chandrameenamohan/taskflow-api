package main

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
)

func TestHealthHandler(t *testing.T) {
	req := httptest.NewRequest(http.MethodGet, "/health", nil)
	w := httptest.NewRecorder()

	healthHandler(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}

	ct := w.Header().Get("Content-Type")
	if ct != "application/json" {
		t.Fatalf("expected application/json, got %s", ct)
	}

	var resp healthResponse
	if err := json.NewDecoder(w.Body).Decode(&resp); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}

	if resp.Status != "ok" {
		t.Errorf("expected status ok, got %s", resp.Status)
	}
	if resp.UptimeSec <= 0 {
		t.Errorf("expected positive uptime_seconds, got %f", resp.UptimeSec)
	}
}

func TestMetricsHandler(t *testing.T) {
	// Reset counter for deterministic test
	requestCount.Store(0)

	req := httptest.NewRequest(http.MethodGet, "/metrics", nil)
	w := httptest.NewRecorder()

	metricsHandler(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}

	var resp metricsResponse
	if err := json.NewDecoder(w.Body).Decode(&resp); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}

	if resp.RequestsTotal != 0 {
		t.Errorf("expected 0 requests_total, got %d", resp.RequestsTotal)
	}
	if resp.UptimeSec <= 0 {
		t.Errorf("expected positive uptime_seconds, got %f", resp.UptimeSec)
	}
}

func TestCountRequestsMiddleware(t *testing.T) {
	requestCount.Store(0)

	handler := countRequests(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
	}))

	for i := 0; i < 5; i++ {
		req := httptest.NewRequest(http.MethodGet, "/", nil)
		w := httptest.NewRecorder()
		handler.ServeHTTP(w, req)
	}

	if got := requestCount.Load(); got != 5 {
		t.Errorf("expected 5 requests counted, got %d", got)
	}
}
