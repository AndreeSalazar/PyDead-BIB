// ADead-BIB - GPU Scheduler
// Scheduler determinista ultra-delgado CPU → GPU
// Sin colas dinámicas, sin locks, sin abstracciones
//
// Filosofía: "Quién decide cuándo, cómo y por qué se ejecuta cada shader"
//
// Autor: Eddi Andreé Salazar Matos

use std::time::{Duration, Instant};

/// Dispatch request - Estructura mínima para ejecutar un shader
#[derive(Debug, Clone)]
pub struct Dispatch {
    /// ID del shader a ejecutar
    pub shader_id: u32,
    /// Número de workgroups (x, y, z)
    pub workgroups: (u32, u32, u32),
    /// IDs de buffers a usar [input1, input2, ..., output]
    pub buffer_ids: Vec<u32>,
    /// Push constants (datos pequeños inline)
    pub push_constants: Vec<u8>,
    /// Prioridad (0 = máxima)
    pub priority: u8,
    /// Dependencias (IDs de dispatches que deben completar antes)
    pub dependencies: Vec<u32>,
}

impl Dispatch {
    pub fn new(shader_id: u32, workgroups: (u32, u32, u32)) -> Self {
        Dispatch {
            shader_id,
            workgroups,
            buffer_ids: Vec::new(),
            push_constants: Vec::new(),
            priority: 0,
            dependencies: Vec::new(),
        }
    }

    pub fn with_buffers(mut self, buffers: Vec<u32>) -> Self {
        self.buffer_ids = buffers;
        self
    }

    pub fn with_push_constants(mut self, data: Vec<u8>) -> Self {
        self.push_constants = data;
        self
    }

    pub fn with_dependency(mut self, dep_id: u32) -> Self {
        self.dependencies.push(dep_id);
        self
    }

    /// Calcula el número total de invocaciones
    pub fn total_invocations(&self) -> u64 {
        self.workgroups.0 as u64 * self.workgroups.1 as u64 * self.workgroups.2 as u64
    }
}

/// Estado de un dispatch
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchState {
    Pending,
    Ready,     // Dependencias satisfechas
    Submitted, // Enviado a GPU
    Completed,
    Failed,
}

/// Dispatch con estado y métricas
#[derive(Debug, Clone)]
pub struct TrackedDispatch {
    pub id: u32,
    pub dispatch: Dispatch,
    pub state: DispatchState,
    /// Timestamp de creación
    pub created_at: Instant,
    /// Timestamp de submit a GPU
    pub submitted_at: Option<Instant>,
    /// Timestamp de completado
    pub completed_at: Option<Instant>,
}

impl TrackedDispatch {
    pub fn new(id: u32, dispatch: Dispatch) -> Self {
        TrackedDispatch {
            id,
            dispatch,
            state: DispatchState::Pending,
            created_at: Instant::now(),
            submitted_at: None,
            completed_at: None,
        }
    }

    /// Latencia total (creación → completado)
    pub fn total_latency(&self) -> Option<Duration> {
        self.completed_at.map(|c| c.duration_since(self.created_at))
    }

    /// Tiempo en GPU (submit → completado)
    pub fn gpu_time(&self) -> Option<Duration> {
        match (self.submitted_at, self.completed_at) {
            (Some(s), Some(c)) => Some(c.duration_since(s)),
            _ => None,
        }
    }
}

/// Scheduler determinista - Sin colas dinámicas, sin locks
pub struct GpuScheduler {
    /// Dispatches pendientes (array fijo, no Vec dinámico en hot path)
    dispatches: Vec<TrackedDispatch>,
    /// Siguiente ID de dispatch
    next_id: u32,
    /// Dispatches completados (para tracking de dependencias)
    completed_ids: Vec<u32>,
    /// Métricas acumuladas
    pub metrics: SchedulerMetrics,
    /// Configuración
    pub config: SchedulerConfig,
}

/// Configuración del scheduler
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Máximo de dispatches en vuelo
    pub max_in_flight: usize,
    /// Tamaño máximo de batch
    pub max_batch_size: usize,
    /// Timeout para dispatches (ms)
    pub timeout_ms: u64,
    /// Ordenar por prioridad
    pub priority_scheduling: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        SchedulerConfig {
            max_in_flight: 8,
            max_batch_size: 16,
            timeout_ms: 1000,
            priority_scheduling: true,
        }
    }
}

/// Métricas del scheduler
#[derive(Debug, Clone, Default)]
pub struct SchedulerMetrics {
    /// Total de dispatches procesados
    pub total_dispatches: u64,
    /// Dispatches exitosos
    pub successful_dispatches: u64,
    /// Dispatches fallidos
    pub failed_dispatches: u64,
    /// Latencia promedio (ns)
    pub avg_latency_ns: u64,
    /// Latencia mínima (ns)
    pub min_latency_ns: u64,
    /// Latencia máxima (ns)
    pub max_latency_ns: u64,
    /// Tiempo total en GPU (ns)
    pub total_gpu_time_ns: u64,
    /// Invocaciones totales
    pub total_invocations: u64,
}

impl GpuScheduler {
    pub fn new() -> Self {
        GpuScheduler {
            dispatches: Vec::with_capacity(64),
            next_id: 0,
            completed_ids: Vec::with_capacity(256),
            metrics: SchedulerMetrics::default(),
            config: SchedulerConfig::default(),
        }
    }

    pub fn with_config(config: SchedulerConfig) -> Self {
        let mut scheduler = Self::new();
        scheduler.config = config;
        scheduler
    }

    /// Encola un dispatch y retorna su ID
    pub fn submit(&mut self, dispatch: Dispatch) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        let tracked = TrackedDispatch::new(id, dispatch);
        self.dispatches.push(tracked);

        id
    }

    /// Obtiene dispatches listos para ejecutar (dependencias satisfechas)
    pub fn get_ready_dispatches(&mut self) -> Vec<u32> {
        let mut ready = Vec::new();

        for dispatch in &mut self.dispatches {
            if dispatch.state != DispatchState::Pending {
                continue;
            }

            // Verificar dependencias
            let deps_satisfied = dispatch
                .dispatch
                .dependencies
                .iter()
                .all(|dep_id| self.completed_ids.contains(dep_id));

            if deps_satisfied {
                dispatch.state = DispatchState::Ready;
                ready.push(dispatch.id);
            }
        }

        // Ordenar por prioridad si está habilitado
        if self.config.priority_scheduling {
            ready.sort_by(|a, b| {
                let pa = self
                    .dispatches
                    .iter()
                    .find(|d| d.id == *a)
                    .map(|d| d.dispatch.priority)
                    .unwrap_or(255);
                let pb = self
                    .dispatches
                    .iter()
                    .find(|d| d.id == *b)
                    .map(|d| d.dispatch.priority)
                    .unwrap_or(255);
                pa.cmp(&pb)
            });
        }

        // Limitar a max_batch_size
        ready.truncate(self.config.max_batch_size);

        ready
    }

    /// Marca un dispatch como enviado a GPU
    pub fn mark_submitted(&mut self, id: u32) {
        if let Some(dispatch) = self.dispatches.iter_mut().find(|d| d.id == id) {
            dispatch.state = DispatchState::Submitted;
            dispatch.submitted_at = Some(Instant::now());
        }
    }

    /// Marca un dispatch como completado
    pub fn mark_completed(&mut self, id: u32) {
        // Extraer datos necesarios primero para evitar borrow múltiple
        let dispatch_data = self
            .dispatches
            .iter_mut()
            .find(|d| d.id == id)
            .map(|dispatch| {
                dispatch.state = DispatchState::Completed;
                dispatch.completed_at = Some(Instant::now());
                (
                    dispatch.dispatch.total_invocations(),
                    dispatch.total_latency(),
                    dispatch.gpu_time(),
                )
            });

        if let Some((invocations, latency, gpu_time)) = dispatch_data {
            // Actualizar métricas
            self.metrics.total_dispatches += 1;
            self.metrics.successful_dispatches += 1;
            self.metrics.total_invocations += invocations;

            if let Some(lat) = latency {
                let ns = lat.as_nanos() as u64;
                self.update_latency_metrics(ns);
            }

            if let Some(gt) = gpu_time {
                self.metrics.total_gpu_time_ns += gt.as_nanos() as u64;
            }

            // Agregar a completados para dependencias
            self.completed_ids.push(id);
        }
    }

    /// Marca un dispatch como fallido
    pub fn mark_failed(&mut self, id: u32) {
        if let Some(dispatch) = self.dispatches.iter_mut().find(|d| d.id == id) {
            dispatch.state = DispatchState::Failed;
            self.metrics.total_dispatches += 1;
            self.metrics.failed_dispatches += 1;
        }
    }

    fn update_latency_metrics(&mut self, latency_ns: u64) {
        if self.metrics.min_latency_ns == 0 || latency_ns < self.metrics.min_latency_ns {
            self.metrics.min_latency_ns = latency_ns;
        }
        if latency_ns > self.metrics.max_latency_ns {
            self.metrics.max_latency_ns = latency_ns;
        }

        // Promedio móvil
        let n = self.metrics.successful_dispatches;
        self.metrics.avg_latency_ns = (self.metrics.avg_latency_ns * (n - 1) + latency_ns) / n;
    }

    /// Obtiene un dispatch por ID
    pub fn get_dispatch(&self, id: u32) -> Option<&TrackedDispatch> {
        self.dispatches.iter().find(|d| d.id == id)
    }

    /// Limpia dispatches completados (libera memoria)
    pub fn cleanup_completed(&mut self) {
        self.dispatches
            .retain(|d| d.state != DispatchState::Completed && d.state != DispatchState::Failed);
    }

    /// Número de dispatches pendientes
    pub fn pending_count(&self) -> usize {
        self.dispatches
            .iter()
            .filter(|d| d.state == DispatchState::Pending || d.state == DispatchState::Ready)
            .count()
    }

    /// Número de dispatches en vuelo (submitted)
    pub fn in_flight_count(&self) -> usize {
        self.dispatches
            .iter()
            .filter(|d| d.state == DispatchState::Submitted)
            .count()
    }

    /// Imprime métricas
    pub fn print_metrics(&self) {
        println!("📊 GPU Scheduler Metrics:");
        println!("   Total dispatches:    {}", self.metrics.total_dispatches);
        println!(
            "   Successful:          {}",
            self.metrics.successful_dispatches
        );
        println!("   Failed:              {}", self.metrics.failed_dispatches);
        println!("   Total invocations:   {}", self.metrics.total_invocations);
        println!(
            "   Avg latency:         {:.2} µs",
            self.metrics.avg_latency_ns as f64 / 1000.0
        );
        println!(
            "   Min latency:         {:.2} µs",
            self.metrics.min_latency_ns as f64 / 1000.0
        );
        println!(
            "   Max latency:         {:.2} µs",
            self.metrics.max_latency_ns as f64 / 1000.0
        );
        println!(
            "   Total GPU time:      {:.2} ms",
            self.metrics.total_gpu_time_ns as f64 / 1_000_000.0
        );
    }
}

impl Default for GpuScheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Batch de dispatches para ejecución atómica
#[derive(Debug, Clone)]
pub struct DispatchBatch {
    pub dispatches: Vec<Dispatch>,
    pub sync_after: bool,
}

impl DispatchBatch {
    pub fn new() -> Self {
        DispatchBatch {
            dispatches: Vec::new(),
            sync_after: true,
        }
    }

    pub fn add(&mut self, dispatch: Dispatch) -> &mut Self {
        self.dispatches.push(dispatch);
        self
    }

    pub fn no_sync(mut self) -> Self {
        self.sync_after = false;
        self
    }

    /// Total de workgroups en el batch
    pub fn total_workgroups(&self) -> u64 {
        self.dispatches.iter().map(|d| d.total_invocations()).sum()
    }
}

impl Default for DispatchBatch {
    fn default() -> Self {
        Self::new()
    }
}

/// Command buffer para GPU (lista de comandos pre-grabados)
#[derive(Debug, Clone)]
pub struct CommandBuffer {
    pub commands: Vec<GpuCommand>,
    pub recorded: bool,
}

/// Comandos GPU de bajo nivel
#[derive(Debug, Clone)]
pub enum GpuCommand {
    BindShader(u32),
    BindBuffer { slot: u32, buffer_id: u32 },
    PushConstants(Vec<u8>),
    Dispatch { x: u32, y: u32, z: u32 },
    Barrier,
    CopyBuffer { src: u32, dst: u32, size: u64 },
}

impl CommandBuffer {
    pub fn new() -> Self {
        CommandBuffer {
            commands: Vec::new(),
            recorded: false,
        }
    }

    pub fn bind_shader(&mut self, shader_id: u32) -> &mut Self {
        self.commands.push(GpuCommand::BindShader(shader_id));
        self
    }

    pub fn bind_buffer(&mut self, slot: u32, buffer_id: u32) -> &mut Self {
        self.commands
            .push(GpuCommand::BindBuffer { slot, buffer_id });
        self
    }

    pub fn push_constants(&mut self, data: Vec<u8>) -> &mut Self {
        self.commands.push(GpuCommand::PushConstants(data));
        self
    }

    pub fn dispatch(&mut self, x: u32, y: u32, z: u32) -> &mut Self {
        self.commands.push(GpuCommand::Dispatch { x, y, z });
        self
    }

    pub fn barrier(&mut self) -> &mut Self {
        self.commands.push(GpuCommand::Barrier);
        self
    }

    pub fn copy_buffer(&mut self, src: u32, dst: u32, size: u64) -> &mut Self {
        self.commands
            .push(GpuCommand::CopyBuffer { src, dst, size });
        self
    }

    pub fn finish(&mut self) -> &Self {
        self.recorded = true;
        self
    }

    /// Serializa a bytes para envío directo
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        for cmd in &self.commands {
            match cmd {
                GpuCommand::BindShader(id) => {
                    bytes.push(0x01);
                    bytes.extend_from_slice(&id.to_le_bytes());
                }
                GpuCommand::BindBuffer { slot, buffer_id } => {
                    bytes.push(0x02);
                    bytes.extend_from_slice(&slot.to_le_bytes());
                    bytes.extend_from_slice(&buffer_id.to_le_bytes());
                }
                GpuCommand::PushConstants(data) => {
                    bytes.push(0x03);
                    bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
                    bytes.extend_from_slice(data);
                }
                GpuCommand::Dispatch { x, y, z } => {
                    bytes.push(0x04);
                    bytes.extend_from_slice(&x.to_le_bytes());
                    bytes.extend_from_slice(&y.to_le_bytes());
                    bytes.extend_from_slice(&z.to_le_bytes());
                }
                GpuCommand::Barrier => {
                    bytes.push(0x05);
                }
                GpuCommand::CopyBuffer { src, dst, size } => {
                    bytes.push(0x06);
                    bytes.extend_from_slice(&src.to_le_bytes());
                    bytes.extend_from_slice(&dst.to_le_bytes());
                    bytes.extend_from_slice(&size.to_le_bytes());
                }
            }
        }

        bytes
    }
}

impl Default for CommandBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatch_creation() {
        let dispatch = Dispatch::new(0, (64, 64, 1))
            .with_buffers(vec![0, 1, 2])
            .with_push_constants(vec![0, 0, 0, 0]);

        assert_eq!(dispatch.total_invocations(), 64 * 64);
        assert_eq!(dispatch.buffer_ids.len(), 3);
    }

    #[test]
    fn test_scheduler_submit() {
        let mut scheduler = GpuScheduler::new();

        let id1 = scheduler.submit(Dispatch::new(0, (32, 32, 1)));
        let id2 = scheduler.submit(Dispatch::new(1, (64, 64, 1)));

        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(scheduler.pending_count(), 2);
    }

    #[test]
    fn test_scheduler_dependencies() {
        let mut scheduler = GpuScheduler::new();

        let id1 = scheduler.submit(Dispatch::new(0, (32, 32, 1)));
        let id2 = scheduler.submit(Dispatch::new(1, (64, 64, 1)).with_dependency(id1));

        // Solo id1 debería estar listo
        let ready = scheduler.get_ready_dispatches();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0], id1);

        // Completar id1
        scheduler.mark_submitted(id1);
        scheduler.mark_completed(id1);

        // Ahora id2 debería estar listo
        let ready = scheduler.get_ready_dispatches();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0], id2);
    }

    #[test]
    fn test_command_buffer() {
        let mut cmd = CommandBuffer::new();
        cmd.bind_shader(0)
            .bind_buffer(0, 1)
            .bind_buffer(1, 2)
            .dispatch(64, 64, 1)
            .barrier()
            .finish();

        assert!(cmd.recorded);
        assert_eq!(cmd.commands.len(), 5);

        let bytes = cmd.to_bytes();
        assert!(bytes.len() > 0);
    }
}
