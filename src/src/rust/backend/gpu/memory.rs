// ADead-BIB - GPU Memory Management
// Memoria explícita: buffers persistentes, ring buffers, zero-copy
// Sin abstracciones innecesarias - control total
//
// Filosofía: "¿Dónde viven los datos? ¿Quién sincroniza? ¿Cómo minimizar transferencias?"
//
// Autor: Eddi Andreé Salazar Matos

use std::collections::HashMap;

/// Tipo de memoria GPU
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    /// Memoria de dispositivo (VRAM) - Más rápida para GPU
    DeviceLocal,
    /// Memoria visible por host - Para transferencias CPU↔GPU
    HostVisible,
    /// Memoria coherente - Sin necesidad de flush/invalidate
    HostCoherent,
    /// Memoria cacheada por host - Mejor para lecturas desde CPU
    HostCached,
}

/// Uso del buffer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferUsage {
    /// Buffer de entrada (solo lectura en GPU)
    StorageRead,
    /// Buffer de salida (solo escritura en GPU)
    StorageWrite,
    /// Buffer de entrada/salida
    StorageReadWrite,
    /// Buffer uniforme (constantes)
    Uniform,
    /// Buffer de staging (transferencias)
    Staging,
    /// Buffer de índices
    Index,
    /// Buffer de vértices
    Vertex,
}

/// Descriptor de buffer
#[derive(Debug, Clone)]
pub struct BufferDescriptor {
    pub id: u32,
    pub size: u64,
    pub memory_type: MemoryType,
    pub usage: BufferUsage,
    pub name: String,
    /// Offset en el heap de memoria
    pub offset: u64,
    /// Alineación requerida
    pub alignment: u64,
    /// Mapeado a CPU
    pub mapped: bool,
}

impl BufferDescriptor {
    pub fn new(id: u32, size: u64, usage: BufferUsage) -> Self {
        BufferDescriptor {
            id,
            size,
            memory_type: MemoryType::DeviceLocal,
            usage,
            name: format!("buffer_{}", id),
            offset: 0,
            alignment: 256, // Alineación típica para storage buffers
            mapped: false,
        }
    }

    pub fn with_memory_type(mut self, mem_type: MemoryType) -> Self {
        self.memory_type = mem_type;
        self
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn with_alignment(mut self, alignment: u64) -> Self {
        self.alignment = alignment;
        self
    }

    /// Tamaño alineado
    pub fn aligned_size(&self) -> u64 {
        (self.size + self.alignment - 1) & !(self.alignment - 1)
    }
}

/// Allocator de memoria GPU - Simple bump allocator
pub struct GpuAllocator {
    /// Buffers registrados
    buffers: HashMap<u32, BufferDescriptor>,
    /// Siguiente ID de buffer
    next_id: u32,
    /// Offset actual en el heap de dispositivo
    device_offset: u64,
    /// Offset actual en el heap de host
    host_offset: u64,
    /// Tamaño total del heap de dispositivo
    pub device_heap_size: u64,
    /// Tamaño total del heap de host
    pub host_heap_size: u64,
    /// Métricas
    pub metrics: MemoryMetrics,
}

/// Métricas de memoria
#[derive(Debug, Clone, Default)]
pub struct MemoryMetrics {
    /// Bytes asignados en dispositivo
    pub device_allocated: u64,
    /// Bytes asignados en host
    pub host_allocated: u64,
    /// Número de buffers
    pub buffer_count: u32,
    /// Transferencias CPU→GPU
    pub uploads: u64,
    /// Transferencias GPU→CPU
    pub downloads: u64,
    /// Bytes transferidos CPU→GPU
    pub bytes_uploaded: u64,
    /// Bytes transferidos GPU→CPU
    pub bytes_downloaded: u64,
}

impl GpuAllocator {
    pub fn new(device_heap_size: u64, host_heap_size: u64) -> Self {
        GpuAllocator {
            buffers: HashMap::new(),
            next_id: 0,
            device_offset: 0,
            host_offset: 0,
            device_heap_size,
            host_heap_size,
            metrics: MemoryMetrics::default(),
        }
    }

    /// Crea un buffer con tamaño y uso específicos
    pub fn create_buffer(&mut self, size: u64, usage: BufferUsage) -> Result<u32, &'static str> {
        let id = self.next_id;
        self.next_id += 1;

        let mut desc = BufferDescriptor::new(id, size, usage);

        // Determinar tipo de memoria según uso
        desc.memory_type = match usage {
            BufferUsage::Staging => MemoryType::HostVisible,
            BufferUsage::Uniform => MemoryType::HostCoherent,
            _ => MemoryType::DeviceLocal,
        };

        // Asignar offset
        let aligned_size = desc.aligned_size();
        match desc.memory_type {
            MemoryType::DeviceLocal => {
                if self.device_offset + aligned_size > self.device_heap_size {
                    return Err("Device heap exhausted");
                }
                desc.offset = self.device_offset;
                self.device_offset += aligned_size;
                self.metrics.device_allocated += aligned_size;
            }
            _ => {
                if self.host_offset + aligned_size > self.host_heap_size {
                    return Err("Host heap exhausted");
                }
                desc.offset = self.host_offset;
                self.host_offset += aligned_size;
                self.metrics.host_allocated += aligned_size;
            }
        }

        self.buffers.insert(id, desc);
        self.metrics.buffer_count += 1;

        Ok(id)
    }

    /// Crea un buffer con nombre
    pub fn create_named_buffer(
        &mut self,
        name: &str,
        size: u64,
        usage: BufferUsage,
    ) -> Result<u32, &'static str> {
        let id = self.create_buffer(size, usage)?;
        if let Some(buf) = self.buffers.get_mut(&id) {
            buf.name = name.to_string();
        }
        Ok(id)
    }

    /// Obtiene descriptor de buffer
    pub fn get_buffer(&self, id: u32) -> Option<&BufferDescriptor> {
        self.buffers.get(&id)
    }

    /// Libera un buffer
    pub fn free_buffer(&mut self, id: u32) -> bool {
        if let Some(desc) = self.buffers.remove(&id) {
            self.metrics.buffer_count -= 1;
            match desc.memory_type {
                MemoryType::DeviceLocal => {
                    self.metrics.device_allocated -= desc.aligned_size();
                }
                _ => {
                    self.metrics.host_allocated -= desc.aligned_size();
                }
            }
            true
        } else {
            false
        }
    }

    /// Registra una transferencia CPU→GPU
    pub fn record_upload(&mut self, bytes: u64) {
        self.metrics.uploads += 1;
        self.metrics.bytes_uploaded += bytes;
    }

    /// Registra una transferencia GPU→CPU
    pub fn record_download(&mut self, bytes: u64) {
        self.metrics.downloads += 1;
        self.metrics.bytes_downloaded += bytes;
    }

    /// Imprime métricas
    pub fn print_metrics(&self) {
        println!("📊 GPU Memory Metrics:");
        println!(
            "   Device allocated:  {:.2} MB / {:.2} MB",
            self.metrics.device_allocated as f64 / 1_048_576.0,
            self.device_heap_size as f64 / 1_048_576.0
        );
        println!(
            "   Host allocated:    {:.2} MB / {:.2} MB",
            self.metrics.host_allocated as f64 / 1_048_576.0,
            self.host_heap_size as f64 / 1_048_576.0
        );
        println!("   Buffer count:      {}", self.metrics.buffer_count);
        println!(
            "   Uploads:           {} ({:.2} MB)",
            self.metrics.uploads,
            self.metrics.bytes_uploaded as f64 / 1_048_576.0
        );
        println!(
            "   Downloads:         {} ({:.2} MB)",
            self.metrics.downloads,
            self.metrics.bytes_downloaded as f64 / 1_048_576.0
        );
    }
}

impl Default for GpuAllocator {
    fn default() -> Self {
        // Default: 256 MB device, 64 MB host
        Self::new(256 * 1024 * 1024, 64 * 1024 * 1024)
    }
}

/// Ring buffer para streaming de datos a GPU
pub struct RingBuffer {
    pub buffer_id: u32,
    pub size: u64,
    pub write_offset: u64,
    pub read_offset: u64,
    /// Frames en vuelo (para sincronización)
    pub frames_in_flight: u32,
    /// Tamaño por frame
    pub frame_size: u64,
}

impl RingBuffer {
    pub fn new(buffer_id: u32, size: u64, frames_in_flight: u32) -> Self {
        let frame_size = size / frames_in_flight as u64;
        RingBuffer {
            buffer_id,
            size,
            write_offset: 0,
            read_offset: 0,
            frames_in_flight,
            frame_size,
        }
    }

    /// Obtiene offset para escribir el siguiente frame
    pub fn next_write_offset(&mut self) -> u64 {
        let offset = self.write_offset;
        self.write_offset = (self.write_offset + self.frame_size) % self.size;
        offset
    }

    /// Avanza el offset de lectura
    pub fn advance_read(&mut self) {
        self.read_offset = (self.read_offset + self.frame_size) % self.size;
    }

    /// Espacio disponible para escritura
    pub fn available_space(&self) -> u64 {
        if self.write_offset >= self.read_offset {
            self.size - (self.write_offset - self.read_offset)
        } else {
            self.read_offset - self.write_offset
        }
    }
}

/// Staging buffer para transferencias eficientes
pub struct StagingBuffer {
    pub buffer_id: u32,
    pub size: u64,
    pub offset: u64,
}

impl StagingBuffer {
    pub fn new(buffer_id: u32, size: u64) -> Self {
        StagingBuffer {
            buffer_id,
            size,
            offset: 0,
        }
    }

    /// Reserva espacio en el staging buffer
    pub fn allocate(&mut self, size: u64, alignment: u64) -> Option<u64> {
        let aligned_offset = (self.offset + alignment - 1) & !(alignment - 1);
        if aligned_offset + size > self.size {
            return None;
        }
        self.offset = aligned_offset + size;
        Some(aligned_offset)
    }

    /// Resetea el staging buffer
    pub fn reset(&mut self) {
        self.offset = 0;
    }
}

/// Transfer command para copias de memoria
#[derive(Debug, Clone)]
pub struct TransferCommand {
    pub src_buffer: u32,
    pub src_offset: u64,
    pub dst_buffer: u32,
    pub dst_offset: u64,
    pub size: u64,
}

impl TransferCommand {
    pub fn new(src: u32, dst: u32, size: u64) -> Self {
        TransferCommand {
            src_buffer: src,
            src_offset: 0,
            dst_buffer: dst,
            dst_offset: 0,
            size,
        }
    }

    pub fn with_offsets(mut self, src_offset: u64, dst_offset: u64) -> Self {
        self.src_offset = src_offset;
        self.dst_offset = dst_offset;
        self
    }
}

/// Memory barrier para sincronización
#[derive(Debug, Clone, Copy)]
pub enum MemoryBarrier {
    /// Barrera completa (all stages)
    Full,
    /// Barrera de compute
    Compute,
    /// Barrera de transferencia
    Transfer,
    /// Barrera de host
    Host,
}

/// Zero-copy buffer descriptor (para memoria compartida CPU-GPU)
#[derive(Debug, Clone)]
pub struct ZeroCopyBuffer {
    pub buffer_id: u32,
    pub size: u64,
    /// Puntero a memoria (simulado como offset)
    pub host_ptr: u64,
    /// Coherente (no necesita flush)
    pub coherent: bool,
}

impl ZeroCopyBuffer {
    pub fn new(buffer_id: u32, size: u64, coherent: bool) -> Self {
        ZeroCopyBuffer {
            buffer_id,
            size,
            host_ptr: 0,
            coherent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocator_create_buffer() {
        let mut alloc = GpuAllocator::new(1024 * 1024, 256 * 1024);

        let id = alloc
            .create_buffer(1024, BufferUsage::StorageReadWrite)
            .unwrap();
        assert_eq!(id, 0);

        let buf = alloc.get_buffer(id).unwrap();
        assert_eq!(buf.size, 1024);
        assert_eq!(buf.memory_type, MemoryType::DeviceLocal);
    }

    #[test]
    fn test_allocator_staging() {
        let mut alloc = GpuAllocator::new(1024 * 1024, 256 * 1024);

        let id = alloc.create_buffer(4096, BufferUsage::Staging).unwrap();
        let buf = alloc.get_buffer(id).unwrap();

        assert_eq!(buf.memory_type, MemoryType::HostVisible);
    }

    #[test]
    fn test_ring_buffer() {
        let mut ring = RingBuffer::new(0, 1024, 3);

        let off1 = ring.next_write_offset();
        let off2 = ring.next_write_offset();
        let off3 = ring.next_write_offset();

        assert_eq!(off1, 0);
        assert!(off2 > off1);
        assert!(off3 > off2);
    }

    #[test]
    fn test_staging_buffer() {
        let mut staging = StagingBuffer::new(0, 4096);

        let off1 = staging.allocate(256, 64).unwrap();
        let off2 = staging.allocate(512, 256).unwrap();

        assert_eq!(off1, 0);
        assert_eq!(off2 % 256, 0); // Alineado
    }
}
