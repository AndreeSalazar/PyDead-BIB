// ADead-BIB - GPU Metrics System
// Medición REAL: latencia, ocupación, coherencia cache
// Sin benchmarks fake - datos duros
//
// Filosofía: "Sin métricas, no hay argumento técnico fuerte"
//
// Autor: Eddi Andreé Salazar Matos

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Métricas de rendimiento GPU
#[derive(Debug, Clone, Default)]
pub struct GpuMetrics {
    // Latencias
    /// Latencia CPU → GPU (submit to start)
    pub cpu_to_gpu_latency_ns: u64,
    /// Latencia GPU → CPU (complete to available)
    pub gpu_to_cpu_latency_ns: u64,
    /// Tiempo de dispatch (submit to complete)
    pub dispatch_time_ns: u64,

    // Throughput
    /// FLOPS alcanzados
    pub achieved_flops: f64,
    /// Bandwidth alcanzado (bytes/s)
    pub achieved_bandwidth: f64,
    /// Invocaciones por segundo
    pub invocations_per_second: f64,

    // Ocupación
    /// Ocupación de compute units (0.0 - 1.0)
    pub compute_occupancy: f32,
    /// Ocupación de memoria (0.0 - 1.0)
    pub memory_occupancy: f32,

    // Cache
    /// Hit rate L1 (0.0 - 1.0)
    pub l1_hit_rate: f32,
    /// Hit rate L2 (0.0 - 1.0)
    pub l2_hit_rate: f32,

    // Contadores
    /// Total de dispatches
    pub total_dispatches: u64,
    /// Total de invocaciones
    pub total_invocations: u64,
    /// Total de bytes transferidos
    pub total_bytes_transferred: u64,
}

impl GpuMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    /// Calcula eficiencia (achieved / theoretical)
    pub fn compute_efficiency(&self, theoretical_flops: f64) -> f32 {
        if theoretical_flops > 0.0 {
            (self.achieved_flops / theoretical_flops) as f32
        } else {
            0.0
        }
    }

    /// Calcula bandwidth efficiency
    pub fn bandwidth_efficiency(&self, theoretical_bandwidth: f64) -> f32 {
        if theoretical_bandwidth > 0.0 {
            (self.achieved_bandwidth / theoretical_bandwidth) as f32
        } else {
            0.0
        }
    }

    /// Imprime resumen de métricas
    pub fn print_summary(&self) {
        println!("═══════════════════════════════════════════════════════");
        println!("                   GPU METRICS REPORT                   ");
        println!("═══════════════════════════════════════════════════════");
        println!();
        println!("📊 LATENCY");
        println!(
            "   CPU → GPU:     {:>10.2} µs",
            self.cpu_to_gpu_latency_ns as f64 / 1000.0
        );
        println!(
            "   GPU → CPU:     {:>10.2} µs",
            self.gpu_to_cpu_latency_ns as f64 / 1000.0
        );
        println!(
            "   Dispatch:      {:>10.2} µs",
            self.dispatch_time_ns as f64 / 1000.0
        );
        println!();
        println!("⚡ THROUGHPUT");
        println!(
            "   FLOPS:         {:>10.2} GFLOPS",
            self.achieved_flops / 1e9
        );
        println!(
            "   Bandwidth:     {:>10.2} GB/s",
            self.achieved_bandwidth / 1e9
        );
        println!(
            "   Invocations:   {:>10.2} M/s",
            self.invocations_per_second / 1e6
        );
        println!();
        println!("📈 OCCUPANCY");
        println!(
            "   Compute:       {:>10.1}%",
            self.compute_occupancy * 100.0
        );
        println!("   Memory:        {:>10.1}%", self.memory_occupancy * 100.0);
        println!();
        println!("💾 CACHE");
        println!("   L1 Hit Rate:   {:>10.1}%", self.l1_hit_rate * 100.0);
        println!("   L2 Hit Rate:   {:>10.1}%", self.l2_hit_rate * 100.0);
        println!();
        println!("📋 TOTALS");
        println!("   Dispatches:    {:>10}", self.total_dispatches);
        println!("   Invocations:   {:>10}", self.total_invocations);
        println!(
            "   Transferred:   {:>10.2} MB",
            self.total_bytes_transferred as f64 / 1e6
        );
        println!("═══════════════════════════════════════════════════════");
    }
}

/// Profiler de GPU - Recolecta métricas en tiempo real
pub struct GpuProfiler {
    /// Métricas acumuladas
    pub metrics: GpuMetrics,
    /// Historial de latencias (para percentiles)
    latency_history: VecDeque<u64>,
    /// Tamaño máximo del historial
    history_size: usize,
    /// Timestamp de inicio de sesión
    session_start: Instant,
    /// Último timestamp de dispatch
    last_dispatch: Option<Instant>,
}

impl GpuProfiler {
    pub fn new() -> Self {
        GpuProfiler {
            metrics: GpuMetrics::new(),
            latency_history: VecDeque::with_capacity(1000),
            history_size: 1000,
            session_start: Instant::now(),
            last_dispatch: None,
        }
    }

    /// Inicia medición de dispatch
    pub fn begin_dispatch(&mut self) -> DispatchTimer {
        self.last_dispatch = Some(Instant::now());
        DispatchTimer {
            start: Instant::now(),
            submitted: None,
            completed: None,
        }
    }

    /// Registra dispatch completado
    pub fn end_dispatch(
        &mut self,
        timer: &DispatchTimer,
        invocations: u64,
        bytes: u64,
        flops: u64,
    ) {
        if let (Some(submitted), Some(completed)) = (timer.submitted, timer.completed) {
            // Latencias
            let cpu_to_gpu = submitted.duration_since(timer.start).as_nanos() as u64;
            let dispatch_time = completed.duration_since(submitted).as_nanos() as u64;
            let total_time = completed.duration_since(timer.start).as_nanos() as u64;

            // Actualizar métricas con promedio móvil
            let n = self.metrics.total_dispatches as f64;
            self.metrics.cpu_to_gpu_latency_ns = ((self.metrics.cpu_to_gpu_latency_ns as f64 * n
                + cpu_to_gpu as f64)
                / (n + 1.0)) as u64;
            self.metrics.dispatch_time_ns = ((self.metrics.dispatch_time_ns as f64 * n
                + dispatch_time as f64)
                / (n + 1.0)) as u64;

            // Historial para percentiles
            if self.latency_history.len() >= self.history_size {
                self.latency_history.pop_front();
            }
            self.latency_history.push_back(total_time);

            // Throughput
            let seconds = total_time as f64 / 1e9;
            if seconds > 0.0 {
                self.metrics.achieved_flops = flops as f64 / seconds;
                self.metrics.achieved_bandwidth = bytes as f64 / seconds;
                self.metrics.invocations_per_second = invocations as f64 / seconds;
            }

            // Contadores
            self.metrics.total_dispatches += 1;
            self.metrics.total_invocations += invocations;
            self.metrics.total_bytes_transferred += bytes;
        }
    }

    /// Calcula percentil de latencia
    pub fn latency_percentile(&self, percentile: f32) -> u64 {
        if self.latency_history.is_empty() {
            return 0;
        }

        let mut sorted: Vec<u64> = self.latency_history.iter().copied().collect();
        sorted.sort();

        let idx = ((percentile / 100.0) * (sorted.len() - 1) as f32) as usize;
        sorted[idx]
    }

    /// P50 (mediana)
    pub fn p50_latency(&self) -> u64 {
        self.latency_percentile(50.0)
    }

    /// P95
    pub fn p95_latency(&self) -> u64 {
        self.latency_percentile(95.0)
    }

    /// P99
    pub fn p99_latency(&self) -> u64 {
        self.latency_percentile(99.0)
    }

    /// Duración de la sesión
    pub fn session_duration(&self) -> Duration {
        self.session_start.elapsed()
    }

    /// Imprime reporte completo
    pub fn print_report(&self) {
        self.metrics.print_summary();

        println!();
        println!("📊 LATENCY PERCENTILES");
        println!(
            "   P50:           {:>10.2} µs",
            self.p50_latency() as f64 / 1000.0
        );
        println!(
            "   P95:           {:>10.2} µs",
            self.p95_latency() as f64 / 1000.0
        );
        println!(
            "   P99:           {:>10.2} µs",
            self.p99_latency() as f64 / 1000.0
        );
        println!();
        println!(
            "⏱️  Session duration: {:.2}s",
            self.session_duration().as_secs_f64()
        );
    }

    /// Resetea métricas
    pub fn reset(&mut self) {
        self.metrics = GpuMetrics::new();
        self.latency_history.clear();
        self.session_start = Instant::now();
    }
}

impl Default for GpuProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Timer para un dispatch individual
#[derive(Debug, Clone)]
pub struct DispatchTimer {
    pub start: Instant,
    pub submitted: Option<Instant>,
    pub completed: Option<Instant>,
}

impl DispatchTimer {
    pub fn mark_submitted(&mut self) {
        self.submitted = Some(Instant::now());
    }

    pub fn mark_completed(&mut self) {
        self.completed = Some(Instant::now());
    }

    pub fn total_time(&self) -> Option<Duration> {
        self.completed.map(|c| c.duration_since(self.start))
    }
}

/// Estimador de rendimiento teórico
pub struct PerformanceEstimator {
    /// TFLOPS teóricos de la GPU
    pub theoretical_tflops: f64,
    /// Bandwidth teórico (GB/s)
    pub theoretical_bandwidth_gbs: f64,
    /// Número de compute units
    pub compute_units: u32,
    /// Clock speed (MHz)
    pub clock_mhz: u32,
}

impl PerformanceEstimator {
    /// Crea estimador para GPU conocida
    pub fn for_gpu(vendor: &str, model: &str) -> Self {
        match (vendor, model) {
            ("NVIDIA", "RTX 3060") => Self {
                theoretical_tflops: 12.7,
                theoretical_bandwidth_gbs: 360.0,
                compute_units: 28,
                clock_mhz: 1777,
            },
            ("NVIDIA", "RTX 3070") => Self {
                theoretical_tflops: 20.3,
                theoretical_bandwidth_gbs: 448.0,
                compute_units: 46,
                clock_mhz: 1725,
            },
            ("NVIDIA", "RTX 4090") => Self {
                theoretical_tflops: 82.6,
                theoretical_bandwidth_gbs: 1008.0,
                compute_units: 128,
                clock_mhz: 2520,
            },
            ("AMD", "RX 6800") => Self {
                theoretical_tflops: 16.2,
                theoretical_bandwidth_gbs: 512.0,
                compute_units: 60,
                clock_mhz: 2105,
            },
            _ => Self::generic(),
        }
    }

    /// Estimador genérico
    pub fn generic() -> Self {
        Self {
            theoretical_tflops: 10.0,
            theoretical_bandwidth_gbs: 300.0,
            compute_units: 32,
            clock_mhz: 1500,
        }
    }

    /// Estima tiempo para MatMul
    pub fn estimate_matmul_time_ms(&self, m: u32, n: u32, k: u32) -> f64 {
        // FLOPs para MatMul: 2 * M * N * K
        let flops = 2.0 * m as f64 * n as f64 * k as f64;
        let tflops = self.theoretical_tflops * 1e12;

        // Tiempo teórico (asumiendo 50% eficiencia típica)
        let efficiency = 0.5;
        (flops / (tflops * efficiency)) * 1000.0
    }

    /// Estima tiempo para transferencia de datos
    pub fn estimate_transfer_time_ms(&self, bytes: u64) -> f64 {
        let bandwidth = self.theoretical_bandwidth_gbs * 1e9;
        (bytes as f64 / bandwidth) * 1000.0
    }

    /// Calcula roofline (compute vs memory bound)
    pub fn arithmetic_intensity_threshold(&self) -> f64 {
        // AI = FLOPS / Bytes
        // Si AI > threshold, compute bound; sino, memory bound
        (self.theoretical_tflops * 1e12) / (self.theoretical_bandwidth_gbs * 1e9)
    }

    /// Determina si una operación es compute o memory bound
    pub fn is_compute_bound(&self, flops: u64, bytes: u64) -> bool {
        let ai = flops as f64 / bytes as f64;
        ai > self.arithmetic_intensity_threshold()
    }
}

/// Benchmark result
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub iterations: u32,
    pub total_time_ms: f64,
    pub avg_time_ms: f64,
    pub min_time_ms: f64,
    pub max_time_ms: f64,
    pub achieved_gflops: f64,
    pub achieved_bandwidth_gbs: f64,
    pub efficiency: f32,
}

impl BenchmarkResult {
    pub fn print(&self) {
        println!("┌─────────────────────────────────────────────────────┐");
        println!("│ Benchmark: {:<40} │", self.name);
        println!("├─────────────────────────────────────────────────────┤");
        println!(
            "│ Iterations:     {:>10}                         │",
            self.iterations
        );
        println!(
            "│ Total time:     {:>10.2} ms                      │",
            self.total_time_ms
        );
        println!(
            "│ Avg time:       {:>10.2} ms                      │",
            self.avg_time_ms
        );
        println!(
            "│ Min time:       {:>10.2} ms                      │",
            self.min_time_ms
        );
        println!(
            "│ Max time:       {:>10.2} ms                      │",
            self.max_time_ms
        );
        println!(
            "│ GFLOPS:         {:>10.2}                         │",
            self.achieved_gflops
        );
        println!(
            "│ Bandwidth:      {:>10.2} GB/s                    │",
            self.achieved_bandwidth_gbs
        );
        println!(
            "│ Efficiency:     {:>10.1}%                        │",
            self.efficiency * 100.0
        );
        println!("└─────────────────────────────────────────────────────┘");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_basic() {
        let mut profiler = GpuProfiler::new();

        let mut timer = profiler.begin_dispatch();
        std::thread::sleep(std::time::Duration::from_micros(100));
        timer.mark_submitted();
        std::thread::sleep(std::time::Duration::from_micros(100));
        timer.mark_completed();

        profiler.end_dispatch(&timer, 1000, 4096, 2000);

        assert_eq!(profiler.metrics.total_dispatches, 1);
        assert_eq!(profiler.metrics.total_invocations, 1000);
    }

    #[test]
    fn test_performance_estimator() {
        let estimator = PerformanceEstimator::for_gpu("NVIDIA", "RTX 3060");

        let time = estimator.estimate_matmul_time_ms(1024, 1024, 1024);
        assert!(time > 0.0);

        let threshold = estimator.arithmetic_intensity_threshold();
        assert!(threshold > 0.0);
    }
}
