package main

import (
	"encoding/json"
	"log"
	"net/http"
	"os"
	"sync/atomic"
	"time"
)

var (
	startTime    = time.Now()
	requestCount atomic.Int64
)

type healthResponse struct {
	Status    string  `json:"status"`
	Uptime    string  `json:"uptime"`
	UptimeSec float64 `json:"uptime_seconds"`
}

type metricsResponse struct {
	Uptime        string  `json:"uptime"`
	UptimeSec     float64 `json:"uptime_seconds"`
	RequestsTotal int64   `json:"requests_total"`
}

func healthHandler(w http.ResponseWriter, r *http.Request) {
	uptime := time.Since(startTime)
	resp := healthResponse{
		Status:    "ok",
		Uptime:    uptime.Round(time.Second).String(),
		UptimeSec: uptime.Seconds(),
	}
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(resp)
}

func metricsHandler(w http.ResponseWriter, r *http.Request) {
	uptime := time.Since(startTime)
	resp := metricsResponse{
		Uptime:        uptime.Round(time.Second).String(),
		UptimeSec:     uptime.Seconds(),
		RequestsTotal: requestCount.Load(),
	}
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(resp)
}

// countRequests wraps a handler and increments the global request counter.
func countRequests(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		requestCount.Add(1)
		next.ServeHTTP(w, r)
	})
}

func main() {
	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}

	mux := http.NewServeMux()
	mux.HandleFunc("GET /health", healthHandler)
	mux.HandleFunc("GET /metrics", metricsHandler)

	handler := countRequests(mux)

	log.Printf("taskflow-api listening on :%s", port)
	if err := http.ListenAndServe(":"+port, handler); err != nil {
		log.Fatal(err)
	}
}
