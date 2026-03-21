// ADead-BIB - Vulkan Runtime
// Ejecución REAL de compute shaders en GPU
// Nivel militar: máximo control, zero-copy, determinista
//
// Autor: Eddi Andreé Salazar Matos

use ash::vk;
use std::ffi::CStr;
use std::time::Instant;

/// Runtime Vulkan para ejecución real de shaders
pub struct VulkanRuntime {
    entry: ash::Entry,
    instance: ash::Instance,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    queue: vk::Queue,
    queue_family_index: u32,
    command_pool: vk::CommandPool,
    /// Propiedades del dispositivo
    pub device_props: DeviceProperties,
    /// Métricas de ejecución
    pub metrics: RuntimeMetrics,
}

/// Propiedades del dispositivo
#[derive(Debug, Clone)]
pub struct DeviceProperties {
    pub name: String,
    pub vendor_id: u32,
    pub device_type: vk::PhysicalDeviceType,
    pub api_version: u32,
    pub max_compute_workgroup_size: [u32; 3],
    pub max_compute_workgroup_invocations: u32,
    pub max_compute_shared_memory: u32,
}

/// Métricas de runtime
#[derive(Debug, Clone, Default)]
pub struct RuntimeMetrics {
    pub dispatches: u64,
    pub total_time_ns: u64,
    pub min_time_ns: u64,
    pub max_time_ns: u64,
}

impl VulkanRuntime {
    /// Crea un nuevo runtime Vulkan
    pub unsafe fn new() -> Result<Self, String> {
        // Cargar Vulkan
        let entry = ash::Entry::load().map_err(|e| format!("Failed to load Vulkan: {:?}", e))?;

        // Crear instancia
        let app_info = vk::ApplicationInfo::default()
            .application_name(CStr::from_bytes_with_nul(b"ADead-BIB\0").unwrap())
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_name(CStr::from_bytes_with_nul(b"ADead Engine\0").unwrap())
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(vk::API_VERSION_1_3);

        let create_info = vk::InstanceCreateInfo::default().application_info(&app_info);

        let instance = entry
            .create_instance(&create_info, None)
            .map_err(|e| format!("Failed to create instance: {:?}", e))?;

        // Enumerar dispositivos físicos
        let physical_devices = instance
            .enumerate_physical_devices()
            .map_err(|e| format!("Failed to enumerate devices: {:?}", e))?;

        if physical_devices.is_empty() {
            return Err("No Vulkan devices found".to_string());
        }

        // Seleccionar GPU discreta (preferir NVIDIA)
        let physical_device = Self::select_best_device(&instance, &physical_devices);

        // Obtener propiedades
        let props = instance.get_physical_device_properties(physical_device);
        let device_name = CStr::from_ptr(props.device_name.as_ptr())
            .to_string_lossy()
            .to_string();

        // Buscar queue family de compute
        let queue_families = instance.get_physical_device_queue_family_properties(physical_device);
        let queue_family_index = queue_families
            .iter()
            .position(|qf| qf.queue_flags.contains(vk::QueueFlags::COMPUTE))
            .ok_or("No compute queue found")? as u32;

        // Crear dispositivo lógico
        let queue_priorities = [1.0f32];
        let queue_create_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family_index)
            .queue_priorities(&queue_priorities);

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(std::slice::from_ref(&queue_create_info));

        let device = instance
            .create_device(physical_device, &device_create_info, None)
            .map_err(|e| format!("Failed to create device: {:?}", e))?;

        // Obtener queue
        let queue = device.get_device_queue(queue_family_index, 0);

        // Crear command pool
        let pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        let command_pool = device
            .create_command_pool(&pool_info, None)
            .map_err(|e| format!("Failed to create command pool: {:?}", e))?;

        // Obtener límites de compute
        let limits = props.limits;

        let device_props = DeviceProperties {
            name: device_name,
            vendor_id: props.vendor_id,
            device_type: props.device_type,
            api_version: props.api_version,
            max_compute_workgroup_size: limits.max_compute_work_group_size,
            max_compute_workgroup_invocations: limits.max_compute_work_group_invocations,
            max_compute_shared_memory: limits.max_compute_shared_memory_size,
        };

        Ok(VulkanRuntime {
            entry,
            instance,
            physical_device,
            device,
            queue,
            queue_family_index,
            command_pool,
            device_props,
            metrics: RuntimeMetrics::default(),
        })
    }

    /// Selecciona el mejor dispositivo (preferir GPU discreta NVIDIA)
    unsafe fn select_best_device(
        instance: &ash::Instance,
        devices: &[vk::PhysicalDevice],
    ) -> vk::PhysicalDevice {
        let mut best_device = devices[0];
        let mut best_score = 0u32;

        for &device in devices {
            let props = instance.get_physical_device_properties(device);
            let mut score = 0u32;

            // Preferir GPU discreta
            if props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
                score += 1000;
            }

            // Preferir NVIDIA (0x10DE)
            if props.vendor_id == 0x10DE {
                score += 500;
            }

            // Preferir AMD (0x1002)
            if props.vendor_id == 0x1002 {
                score += 400;
            }

            // Bonus por más memoria
            let mem_props = instance.get_physical_device_memory_properties(device);
            for i in 0..mem_props.memory_heap_count {
                let heap = mem_props.memory_heaps[i as usize];
                if heap.flags.contains(vk::MemoryHeapFlags::DEVICE_LOCAL) {
                    score += (heap.size / (1024 * 1024 * 1024)) as u32; // GB
                }
            }

            if score > best_score {
                best_score = score;
                best_device = device;
            }
        }

        best_device
    }

    /// Crea un buffer de GPU
    pub unsafe fn create_buffer(
        &self,
        size: u64,
        usage: vk::BufferUsageFlags,
    ) -> Result<(vk::Buffer, vk::DeviceMemory), String> {
        let buffer_info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = self
            .device
            .create_buffer(&buffer_info, None)
            .map_err(|e| format!("Failed to create buffer: {:?}", e))?;

        let mem_requirements = self.device.get_buffer_memory_requirements(buffer);

        let mem_props = self
            .instance
            .get_physical_device_memory_properties(self.physical_device);
        let memory_type_index = Self::find_memory_type(
            &mem_props,
            mem_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )
        .ok_or("No suitable memory type")?;

        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_requirements.size)
            .memory_type_index(memory_type_index);

        let memory = self
            .device
            .allocate_memory(&alloc_info, None)
            .map_err(|e| format!("Failed to allocate memory: {:?}", e))?;

        self.device
            .bind_buffer_memory(buffer, memory, 0)
            .map_err(|e| format!("Failed to bind memory: {:?}", e))?;

        Ok((buffer, memory))
    }

    /// Crea un buffer con datos iniciales
    pub unsafe fn create_buffer_with_data<T: Copy>(
        &self,
        data: &[T],
    ) -> Result<(vk::Buffer, vk::DeviceMemory), String> {
        let size = (data.len() * std::mem::size_of::<T>()) as u64;

        // Crear staging buffer (host visible)
        let staging_info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let staging_buffer = self
            .device
            .create_buffer(&staging_info, None)
            .map_err(|e| format!("Failed to create staging buffer: {:?}", e))?;

        let staging_mem_req = self.device.get_buffer_memory_requirements(staging_buffer);
        let mem_props = self
            .instance
            .get_physical_device_memory_properties(self.physical_device);

        let staging_mem_type = Self::find_memory_type(
            &mem_props,
            staging_mem_req.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )
        .ok_or("No host visible memory")?;

        let staging_alloc = vk::MemoryAllocateInfo::default()
            .allocation_size(staging_mem_req.size)
            .memory_type_index(staging_mem_type);

        let staging_memory = self
            .device
            .allocate_memory(&staging_alloc, None)
            .map_err(|e| format!("Failed to allocate staging memory: {:?}", e))?;

        self.device
            .bind_buffer_memory(staging_buffer, staging_memory, 0)
            .map_err(|e| format!("Failed to bind staging memory: {:?}", e))?;

        // Copiar datos al staging buffer
        let ptr = self
            .device
            .map_memory(staging_memory, 0, size, vk::MemoryMapFlags::empty())
            .map_err(|e| format!("Failed to map memory: {:?}", e))?;
        std::ptr::copy_nonoverlapping(data.as_ptr() as *const u8, ptr as *mut u8, size as usize);
        self.device.unmap_memory(staging_memory);

        // Crear buffer de dispositivo
        let (device_buffer, device_memory) = self.create_buffer(
            size,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
        )?;

        // Copiar de staging a device
        self.copy_buffer(staging_buffer, device_buffer, size)?;

        // Limpiar staging
        self.device.destroy_buffer(staging_buffer, None);
        self.device.free_memory(staging_memory, None);

        Ok((device_buffer, device_memory))
    }

    /// Copia buffer
    unsafe fn copy_buffer(
        &self,
        src: vk::Buffer,
        dst: vk::Buffer,
        size: u64,
    ) -> Result<(), String> {
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let cmd_buffer = self
            .device
            .allocate_command_buffers(&alloc_info)
            .map_err(|e| format!("Failed to allocate command buffer: {:?}", e))?[0];

        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        self.device
            .begin_command_buffer(cmd_buffer, &begin_info)
            .map_err(|e| format!("Failed to begin command buffer: {:?}", e))?;

        let copy_region = vk::BufferCopy::default().size(size);
        self.device
            .cmd_copy_buffer(cmd_buffer, src, dst, &[copy_region]);

        self.device
            .end_command_buffer(cmd_buffer)
            .map_err(|e| format!("Failed to end command buffer: {:?}", e))?;

        let cmd_buffers = [cmd_buffer];
        let submit_info = vk::SubmitInfo::default().command_buffers(&cmd_buffers);

        let submit_infos = [submit_info];
        self.device
            .queue_submit(self.queue, &submit_infos, vk::Fence::null())
            .map_err(|e| format!("Failed to submit: {:?}", e))?;

        self.device
            .queue_wait_idle(self.queue)
            .map_err(|e| format!("Failed to wait: {:?}", e))?;

        self.device
            .free_command_buffers(self.command_pool, &[cmd_buffer]);

        Ok(())
    }

    /// Encuentra tipo de memoria adecuado
    fn find_memory_type(
        mem_props: &vk::PhysicalDeviceMemoryProperties,
        type_filter: u32,
        properties: vk::MemoryPropertyFlags,
    ) -> Option<u32> {
        for i in 0..mem_props.memory_type_count {
            if (type_filter & (1 << i)) != 0
                && mem_props.memory_types[i as usize]
                    .property_flags
                    .contains(properties)
            {
                return Some(i);
            }
        }
        None
    }

    /// Crea shader module desde SPIR-V
    pub unsafe fn create_shader_module(&self, spirv: &[u8]) -> Result<vk::ShaderModule, String> {
        // Convertir bytes a u32 (SPIR-V es little-endian u32)
        let code: Vec<u32> = spirv
            .chunks(4)
            .map(|chunk| {
                let mut bytes = [0u8; 4];
                bytes[..chunk.len()].copy_from_slice(chunk);
                u32::from_le_bytes(bytes)
            })
            .collect();

        let create_info = vk::ShaderModuleCreateInfo::default().code(&code);

        self.device
            .create_shader_module(&create_info, None)
            .map_err(|e| format!("Failed to create shader module: {:?}", e))
    }

    /// Crea compute pipeline
    pub unsafe fn create_compute_pipeline(
        &self,
        shader_module: vk::ShaderModule,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> Result<(vk::Pipeline, vk::PipelineLayout), String> {
        let set_layouts = [descriptor_set_layout];
        let layout_info = vk::PipelineLayoutCreateInfo::default().set_layouts(&set_layouts);

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&layout_info, None)
            .map_err(|e| format!("Failed to create pipeline layout: {:?}", e))?;

        let entry_name = CStr::from_bytes_with_nul(b"main\0").unwrap();
        let stage_info = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(shader_module)
            .name(entry_name);

        let pipeline_infos = [vk::ComputePipelineCreateInfo::default()
            .stage(stage_info)
            .layout(pipeline_layout)];

        let pipelines = self
            .device
            .create_compute_pipelines(vk::PipelineCache::null(), &pipeline_infos, None)
            .map_err(|e| format!("Failed to create compute pipeline: {:?}", e.1))?;

        Ok((pipelines[0], pipeline_layout))
    }

    /// Ejecuta compute shader
    pub unsafe fn dispatch_compute(
        &mut self,
        pipeline: vk::Pipeline,
        pipeline_layout: vk::PipelineLayout,
        descriptor_set: vk::DescriptorSet,
        workgroups: (u32, u32, u32),
    ) -> Result<u64, String> {
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let cmd_buffer = self
            .device
            .allocate_command_buffers(&alloc_info)
            .map_err(|e| format!("Failed to allocate command buffer: {:?}", e))?[0];

        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        self.device
            .begin_command_buffer(cmd_buffer, &begin_info)
            .map_err(|e| format!("Failed to begin command buffer: {:?}", e))?;

        self.device
            .cmd_bind_pipeline(cmd_buffer, vk::PipelineBindPoint::COMPUTE, pipeline);
        self.device.cmd_bind_descriptor_sets(
            cmd_buffer,
            vk::PipelineBindPoint::COMPUTE,
            pipeline_layout,
            0,
            &[descriptor_set],
            &[],
        );

        self.device
            .cmd_dispatch(cmd_buffer, workgroups.0, workgroups.1, workgroups.2);

        self.device
            .end_command_buffer(cmd_buffer)
            .map_err(|e| format!("Failed to end command buffer: {:?}", e))?;

        let cmd_buffers = [cmd_buffer];
        let submit_info = vk::SubmitInfo::default().command_buffers(&cmd_buffers);

        let start = Instant::now();

        let submit_infos = [submit_info];
        self.device
            .queue_submit(self.queue, &submit_infos, vk::Fence::null())
            .map_err(|e| format!("Failed to submit: {:?}", e))?;

        self.device
            .queue_wait_idle(self.queue)
            .map_err(|e| format!("Failed to wait: {:?}", e))?;

        let elapsed_ns = start.elapsed().as_nanos() as u64;

        // Actualizar métricas
        self.metrics.dispatches += 1;
        self.metrics.total_time_ns += elapsed_ns;
        if self.metrics.min_time_ns == 0 || elapsed_ns < self.metrics.min_time_ns {
            self.metrics.min_time_ns = elapsed_ns;
        }
        if elapsed_ns > self.metrics.max_time_ns {
            self.metrics.max_time_ns = elapsed_ns;
        }

        self.device
            .free_command_buffers(self.command_pool, &[cmd_buffer]);

        Ok(elapsed_ns)
    }

    /// Imprime info del dispositivo
    pub fn print_device_info(&self) {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║                 VULKAN RUNTIME INITIALIZED                    ║");
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ Device:     {:<48} ║", self.device_props.name);
        println!(
            "║ Vendor ID:  0x{:04X}                                          ║",
            self.device_props.vendor_id
        );
        println!(
            "║ Type:       {:?}                               ║",
            self.device_props.device_type
        );
        println!(
            "║ API:        {}.{}.{}                                           ║",
            vk::api_version_major(self.device_props.api_version),
            vk::api_version_minor(self.device_props.api_version),
            vk::api_version_patch(self.device_props.api_version)
        );
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!(
            "║ Max Workgroup Size:  {:?}                     ║",
            self.device_props.max_compute_workgroup_size
        );
        println!(
            "║ Max Invocations:     {}                                    ║",
            self.device_props.max_compute_workgroup_invocations
        );
        println!(
            "║ Shared Memory:       {} KB                                 ║",
            self.device_props.max_compute_shared_memory / 1024
        );
        println!("╚══════════════════════════════════════════════════════════════╝");
    }

    /// Imprime métricas
    pub fn print_metrics(&self) {
        let avg_ns = if self.metrics.dispatches > 0 {
            self.metrics.total_time_ns / self.metrics.dispatches
        } else {
            0
        };

        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║                    RUNTIME METRICS                            ║");
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!(
            "║ Dispatches:    {}                                            ║",
            self.metrics.dispatches
        );
        println!(
            "║ Total time:    {:.3} ms                                      ║",
            self.metrics.total_time_ns as f64 / 1_000_000.0
        );
        println!(
            "║ Avg time:      {:.3} µs                                      ║",
            avg_ns as f64 / 1_000.0
        );
        println!(
            "║ Min time:      {:.3} µs                                      ║",
            self.metrics.min_time_ns as f64 / 1_000.0
        );
        println!(
            "║ Max time:      {:.3} µs                                      ║",
            self.metrics.max_time_ns as f64 / 1_000.0
        );
        println!("╚══════════════════════════════════════════════════════════════╝");
    }
}

impl Drop for VulkanRuntime {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

/// Inicializa Vulkan y muestra info
pub fn init_vulkan() -> Result<VulkanRuntime, String> {
    unsafe { VulkanRuntime::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vulkan_init() {
        // Solo probar si Vulkan está disponible
        if let Ok(runtime) = unsafe { VulkanRuntime::new() } {
            runtime.print_device_info();
            assert!(!runtime.device_props.name.is_empty());
        }
    }
}
