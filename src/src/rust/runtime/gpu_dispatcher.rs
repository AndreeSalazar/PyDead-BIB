// ADead-BIB HEX - GPU Dispatcher
// Host Determinista que Gobierna Ejecución GPU
// Cost Model: bytes vs FLOPs → decisión automática CPU↔GPU

/// Umbral mínimo de elementos para considerar GPU
/// Basado en benchmark real RTX 3060:
/// - < 100K: CPU gana (overhead PCIe)
/// - > 100K: GPU kernel gana
/// - Pero transferencias dominan si datos no persisten
pub const GPU_THRESHOLD_ELEMENTS: usize = 100_000;

/// Umbral de bytes para transferencia PCIe
/// PCIe 3.0 x16: ~12 GB/s teórico, ~10 GB/s real
/// Si transferencia > 1ms, considerar persistencia
pub const PCIE_TRANSFER_THRESHOLD_BYTES: usize = 10_000_000; // 10 MB

/// Ratio mínimo FLOPs/Bytes para justificar GPU
/// Si hay pocos FLOPs por byte transferido, CPU gana
/// Ejemplo: VectorAdd = 1 FLOP / 12 bytes = 0.08 (muy bajo)
/// MatMul NxN = 2N³ FLOPs / 3N² bytes = 0.67N (escala con N)
pub const MIN_FLOPS_PER_BYTE: f64 = 0.5;

/// Estado de los datos en el sistema
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DataLocation {
    /// Datos en RAM del host
    Host,
    /// Datos en VRAM de GPU
    Device,
    /// Datos en ambos (sincronizados)
    Both,
    /// Datos en GPU, host desactualizado
    DeviceDirty,
}

/// Decisión del dispatcher
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExecutionTarget {
    /// Ejecutar en CPU
    CPU,
    /// Ejecutar en GPU (datos ya en VRAM)
    GPU,
    /// Transferir a GPU, ejecutar, mantener en VRAM
    GPUWithTransfer,
    /// Transferir a GPU, ejecutar, traer de vuelta
    GPURoundTrip,
}

/// Razón de la decisión (para debugging/logging)
#[derive(Debug, Clone)]
pub enum DecisionReason {
    /// Datos muy pequeños, overhead domina
    TooSmall { elements: usize, threshold: usize },
    /// Datos ya en GPU, ejecutar ahí
    DataAlreadyOnDevice,
    /// Suficientes FLOPs para justificar transferencia
    HighComputeIntensity { flops_per_byte: f64 },
    /// Pocos FLOPs, transferencia no vale la pena
    LowComputeIntensity { flops_per_byte: f64 },
    /// Datos persistirán, vale la pena transferir
    WillPersist,
    /// GPU no disponible
    NoGPU,
}

/// Cost Model para operaciones
#[derive(Debug, Clone)]
pub struct OperationCost {
    /// Nombre de la operación
    pub name: String,
    /// Número de elementos
    pub elements: usize,
    /// Bytes por elemento
    pub bytes_per_element: usize,
    /// FLOPs por elemento
    pub flops_per_element: usize,
    /// ¿Los datos persistirán en GPU después?
    pub will_persist: bool,
    /// Ubicación actual de los datos
    pub data_location: DataLocation,
}

impl OperationCost {
    /// Calcula bytes totales
    pub fn total_bytes(&self) -> usize {
        self.elements * self.bytes_per_element
    }

    /// Calcula FLOPs totales
    pub fn total_flops(&self) -> usize {
        self.elements * self.flops_per_element
    }

    /// Calcula ratio FLOPs/Byte
    pub fn flops_per_byte(&self) -> f64 {
        if self.bytes_per_element == 0 {
            return 0.0;
        }
        self.flops_per_element as f64 / self.bytes_per_element as f64
    }

    /// Estima tiempo de transferencia H2D en microsegundos
    /// Basado en PCIe 3.0 x16: ~10 GB/s
    pub fn estimate_h2d_us(&self) -> f64 {
        let bytes = self.total_bytes() as f64;
        let bandwidth = 10_000_000_000.0; // 10 GB/s
        (bytes / bandwidth) * 1_000_000.0
    }

    /// Estima tiempo de kernel GPU en microsegundos
    /// Basado en benchmark RTX 3060: ~300 GFLOPS para operaciones simples
    pub fn estimate_kernel_us(&self) -> f64 {
        let flops = self.total_flops() as f64;
        let throughput = 300_000_000_000.0; // 300 GFLOPS conservador
        (flops / throughput) * 1_000_000.0
    }

    /// Estima tiempo CPU en microsegundos
    /// Basado en benchmark: ~1 GFLOP/s para operaciones simples
    pub fn estimate_cpu_us(&self) -> f64 {
        let flops = self.total_flops() as f64;
        let throughput = 1_000_000_000.0; // 1 GFLOPS conservador
        (flops / throughput) * 1_000_000.0
    }
}

/// GPU Dispatcher - El cerebro de ADead-BIB HEX
pub struct GpuDispatcher {
    /// ¿GPU disponible?
    gpu_available: bool,
    /// Umbral de elementos
    threshold_elements: usize,
    /// Historial de decisiones (para aprendizaje futuro)
    decision_history: Vec<(OperationCost, ExecutionTarget, DecisionReason)>,
}

impl GpuDispatcher {
    pub fn new() -> Self {
        Self {
            gpu_available: Self::detect_gpu(),
            threshold_elements: GPU_THRESHOLD_ELEMENTS,
            decision_history: Vec::new(),
        }
    }

    /// Detecta si hay GPU CUDA disponible
    fn detect_gpu() -> bool {
        // En producción, esto llamaría a cudaGetDeviceCount
        // Por ahora, asumimos que hay GPU si estamos en este módulo
        true
    }

    /// Decide dónde ejecutar una operación
    pub fn decide(&mut self, cost: &OperationCost) -> (ExecutionTarget, DecisionReason) {
        // 1. ¿GPU disponible?
        if !self.gpu_available {
            return (ExecutionTarget::CPU, DecisionReason::NoGPU);
        }

        // 2. ¿Datos ya en GPU?
        if cost.data_location == DataLocation::Device || cost.data_location == DataLocation::Both {
            return (ExecutionTarget::GPU, DecisionReason::DataAlreadyOnDevice);
        }

        // 3. ¿Suficientes elementos?
        if cost.elements < self.threshold_elements {
            return (
                ExecutionTarget::CPU,
                DecisionReason::TooSmall {
                    elements: cost.elements,
                    threshold: self.threshold_elements,
                },
            );
        }

        // 4. ¿Suficiente intensidad computacional?
        let fpb = cost.flops_per_byte();
        if fpb < MIN_FLOPS_PER_BYTE && !cost.will_persist {
            return (
                ExecutionTarget::CPU,
                DecisionReason::LowComputeIntensity {
                    flops_per_byte: fpb,
                },
            );
        }

        // 5. ¿Los datos persistirán?
        if cost.will_persist {
            return (
                ExecutionTarget::GPUWithTransfer,
                DecisionReason::WillPersist,
            );
        }

        // 6. Comparar tiempos estimados
        let cpu_time = cost.estimate_cpu_us();
        let gpu_time = cost.estimate_h2d_us() * 2.0 + cost.estimate_kernel_us(); // H2D + kernel + D2H

        if gpu_time < cpu_time {
            (
                ExecutionTarget::GPURoundTrip,
                DecisionReason::HighComputeIntensity {
                    flops_per_byte: fpb,
                },
            )
        } else {
            (
                ExecutionTarget::CPU,
                DecisionReason::LowComputeIntensity {
                    flops_per_byte: fpb,
                },
            )
        }
    }

    /// Registra una decisión para análisis
    pub fn log_decision(
        &mut self,
        cost: OperationCost,
        target: ExecutionTarget,
        reason: DecisionReason,
    ) {
        self.decision_history.push((cost, target, reason));
    }

    /// Imprime resumen de decisiones
    pub fn print_summary(&self) {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║  ADead-BIB HEX - GPU Dispatcher Summary                      ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();

        let mut cpu_count = 0;
        let mut gpu_count = 0;

        for (cost, target, reason) in &self.decision_history {
            match target {
                ExecutionTarget::CPU => cpu_count += 1,
                _ => gpu_count += 1,
            }
            println!(
                "  {} ({} elements) → {:?}",
                cost.name, cost.elements, target
            );
            println!("    Reason: {:?}", reason);
        }

        println!();
        println!("  Total: {} CPU, {} GPU", cpu_count, gpu_count);
    }
}

impl Default for GpuDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Operaciones predefinidas con su cost model
pub mod operations {
    use super::*;

    /// VectorAdd: C = A + B
    /// 1 FLOP por elemento, 12 bytes por elemento (3 floats)
    pub fn vector_add(n: usize, location: DataLocation, persist: bool) -> OperationCost {
        OperationCost {
            name: "VectorAdd".to_string(),
            elements: n,
            bytes_per_element: 12, // 3 floats * 4 bytes
            flops_per_element: 1,
            will_persist: persist,
            data_location: location,
        }
    }

    /// SAXPY: Y = a*X + Y
    /// 2 FLOPs por elemento (mul + add), 8 bytes por elemento (2 floats)
    pub fn saxpy(n: usize, location: DataLocation, persist: bool) -> OperationCost {
        OperationCost {
            name: "SAXPY".to_string(),
            elements: n,
            bytes_per_element: 8, // 2 floats * 4 bytes
            flops_per_element: 2,
            will_persist: persist,
            data_location: location,
        }
    }

    /// MatMul: C = A * B (NxN matrices)
    /// 2N FLOPs por elemento de C, 12 bytes por elemento
    pub fn matmul(n: usize, location: DataLocation, persist: bool) -> OperationCost {
        OperationCost {
            name: "MatMul".to_string(),
            elements: n * n,          // Elementos en matriz resultado
            bytes_per_element: 12,    // 3 matrices * 4 bytes / elemento
            flops_per_element: 2 * n, // 2N FLOPs por elemento
            will_persist: persist,
            data_location: location,
        }
    }

    /// Reduction: sum(A)
    /// 1 FLOP por elemento, 4 bytes por elemento
    pub fn reduction(n: usize, location: DataLocation) -> OperationCost {
        OperationCost {
            name: "Reduction".to_string(),
            elements: n,
            bytes_per_element: 4,
            flops_per_element: 1,
            will_persist: false, // Resultado es un escalar
            data_location: location,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_data_goes_to_cpu() {
        let mut dispatcher = GpuDispatcher::new();
        let cost = operations::vector_add(1000, DataLocation::Host, false);
        let (target, _) = dispatcher.decide(&cost);
        assert_eq!(target, ExecutionTarget::CPU);
    }

    #[test]
    fn test_data_on_device_stays_on_device() {
        let mut dispatcher = GpuDispatcher::new();
        let cost = operations::vector_add(1000, DataLocation::Device, false);
        let (target, _) = dispatcher.decide(&cost);
        assert_eq!(target, ExecutionTarget::GPU);
    }

    #[test]
    fn test_matmul_large_goes_to_gpu() {
        let mut dispatcher = GpuDispatcher::new();
        // MatMul 512x512 tiene alta intensidad computacional
        let cost = operations::matmul(512, DataLocation::Host, true);
        let (target, _) = dispatcher.decide(&cost);
        assert!(matches!(
            target,
            ExecutionTarget::GPUWithTransfer | ExecutionTarget::GPURoundTrip
        ));
    }

    #[test]
    fn test_vectoradd_large_with_persist() {
        let mut dispatcher = GpuDispatcher::new();
        let cost = operations::vector_add(10_000_000, DataLocation::Host, true);
        let (target, _) = dispatcher.decide(&cost);
        assert_eq!(target, ExecutionTarget::GPUWithTransfer);
    }
}
