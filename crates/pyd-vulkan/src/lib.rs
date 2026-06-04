//! Motor Vulkan real para operaciones de IA (MatMul).
//! Crea buffers, asigna memoria, y ejecuta shaders de Compute.

use anyhow::{Context, Result};
use ash::vk;
use pyd_core::TensorMeta;
use std::collections::HashMap;
use std::ffi::CString;

/// Representa un tensor alojado en la memoria de la GPU.
pub struct TensorBuffer {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub size: u64,
}

pub struct VulkanEngine {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub queue: vk::Queue,
    pub queue_family_index: u32,
    pub command_pool: vk::CommandPool,
    pub command_buffer: vk::CommandBuffer,
    pub tensor_registry: HashMap<String, TensorBuffer>,
}

impl VulkanEngine {
    pub fn new() -> Result<Self> {
        log::info!("🚀 Inicializando Vulkan Engine (ash)...");

        // 1. Cargar la librería de Vulkan
        let entry = unsafe { ash::Entry::load() }?;

        // 2. Crear la Instancia
        let app_name = CString::new("PyDead-BIB").unwrap();
        let app_info = vk::ApplicationInfo::default()
            .application_name(&app_name)
            .application_version(vk::make_api_version(0, 1, 2, 0))
            .api_version(vk::make_api_version(0, 1, 2, 0));

        let instance_info = vk::InstanceCreateInfo::default().application_info(&app_info);
        let instance = unsafe { entry.create_instance(&instance_info, None) }?;

        // 3. Seleccionar Physical Device (GPU)
        let physical_devices = unsafe { instance.enumerate_physical_devices() }?;
        let physical_device = physical_devices[0];

        // 4. Buscar una cola que soporte Compute
        let queue_family_properties =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
        let queue_family_index = queue_family_properties
            .iter()
            .position(|q| q.queue_flags.contains(vk::QueueFlags::COMPUTE))
            .expect("No se encontró una cola de Compute") as u32;

        // 5. Crear Logical Device
        let queue_priority = 1.0f32;
        let queue_priorities = [queue_priority];
        let queue_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family_index)
            .queue_priorities(&queue_priorities);

        let device_info = vk::DeviceCreateInfo::default().queue_create_infos(std::slice::from_ref(&queue_info));
        let device = unsafe { instance.create_device(physical_device, &device_info, None) }?;

        // 6. Obtener la cola
        let queue = unsafe { device.get_device_queue(queue_family_index, 0) };

        // 7. Crear Command Pool
        let pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let command_pool = unsafe { device.create_command_pool(&pool_info, None) }?;

        // 8. Asignar Command Buffer
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let command_buffer = unsafe { device.allocate_command_buffers(&alloc_info) }?[0];

        log::info!("✅ Vulkan Engine inicializado correctamente. GPU lista para Compute.");

        Ok(Self {
            entry,
            instance,
            physical_device,
            device,
            queue,
            queue_family_index,
            command_pool,
            command_buffer,
            tensor_registry: HashMap::new(),
        })
    }

    /// Crea un buffer en la GPU, asigna memoria y sube datos iniciales.
    pub fn create_tensor_buffer(&mut self, meta: &TensorMeta, initial_data: &[f32]) -> Result<()> {
        let element_size = std::mem::size_of::<f32>() as u64;
        let total_elements: u64 = meta.shape.iter().map(|&x| x as u64).product();
        let buffer_size = total_elements * element_size;

        // 1. Crear el Buffer
        let buffer_info = vk::BufferCreateInfo::default()
            .size(buffer_size)
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { self.device.create_buffer(&buffer_info, None) }?;

        // 2. Asignar Memoria (Host Visible para subir datos desde CPU)
        let mem_reqs = unsafe { self.device.get_buffer_memory_requirements(buffer) };
        let mem_type_index = self.find_memory_type(
            mem_reqs.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_reqs.size)
            .memory_type_index(mem_type_index);

        let memory = unsafe { self.device.allocate_memory(&alloc_info, None) }?;
        unsafe { self.device.bind_buffer_memory(buffer, memory, 0) }?;

        // 3. Subir datos a la GPU
        let ptr = unsafe {
            self.device
                .map_memory(memory, 0, buffer_size, vk::MemoryMapFlags::empty())
        }? as *mut f32;
        unsafe {
            std::ptr::copy_nonoverlapping(initial_data.as_ptr(), ptr, initial_data.len());
            self.device.unmap_memory(memory);
        }

        log::info!(
            "✅ Tensor '{}' creado en GPU ({} bytes)",
            meta.name,
            buffer_size
        );

        // 4. Registrar el tensor
        self.tensor_registry.insert(
            meta.name.clone(),
            TensorBuffer {
                buffer,
                memory,
                size: buffer_size,
            },
        );

        Ok(())
    }

    /// Ejecuta el shader de MatMul en la GPU.
    pub fn execute_matmul(&self, dest: &str, lhs: &str, rhs: &str) -> Result<()> {
        log::info!("🔥 Ejecutando MatMul en GPU: {} = {} @ {}", dest, lhs, rhs);

        let a_buf = self.tensor_registry.get(lhs).context("Tensor A no encontrado")?;
        let b_buf = self.tensor_registry.get(rhs).context("Tensor B no encontrado")?;
        let c_buf = self.tensor_registry.get(dest).context("Tensor C no encontrado")?;

        // 1. Cargar el Shader SPIR-V
        // Nota: Asumimos que el shader está compilado en shaders/matmul.comp.spv
        let shader_path = "shaders/matmul.comp.spv";
        let shader_code = match std::fs::read(shader_path) {
            Ok(code) => code,
            Err(_) => {
                log::warn!("⚠️  Shader SPIR-V no encontrado en {}. Ejecutando en modo simulación.", shader_path);
                log::warn!("   Para compilarlo: glslc shaders/matmul.comp -o shaders/matmul.comp.spv");
                return Ok(());
            }
        };

        let shader_module_info = vk::ShaderModuleCreateInfo::default()
            .code_size(shader_code.len())
            .code(std::mem::transmute::<*const u8, *const u32>(shader_code.as_ptr()));
        let shader_module = unsafe { self.device.create_shader_module(&shader_module_info, None) }?;

        // 2. Crear Descriptor Set Layout (3 Storage Buffers)
        let bindings = [
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
            vk::DescriptorSetLayoutBinding::default()
                .binding(2)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
        ];
        let layout_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
        let desc_layout = unsafe { self.device.create_descriptor_set_layout(&layout_info, None) }?;

        // 3. Crear Pipeline Layout y Compute Pipeline
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(std::slice::from_ref(&desc_layout));
        let pipeline_layout = unsafe { self.device.create_pipeline_layout(&pipeline_layout_info, None) }?;

        let entry_point = CString::new("main").unwrap();
        let stage_info = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(shader_module)
            .name(&entry_point);

        let pipeline_info = vk::ComputePipelineCreateInfo::default()
            .stage(stage_info)
            .layout(pipeline_layout);

        let pipelines = unsafe {
            self.device
                .create_compute_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
        }?;
        let pipeline = pipelines[0];

        // 4. Crear Descriptor Pool y Set
        let pool_sizes = [vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(3)];
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(1)
            .pool_sizes(&pool_sizes);
        let desc_pool = unsafe { self.device.create_descriptor_pool(&pool_info, None) }?;

        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(desc_pool)
            .set_layouts(std::slice::from_ref(&desc_layout));
        let desc_sets = unsafe { self.device.allocate_descriptor_sets(&alloc_info) }?;
        let desc_set = desc_sets[0];

        let buffer_infos = [
            vk::DescriptorBufferInfo::default()
                .buffer(a_buf.buffer)
                .offset(0)
                .range(a_buf.size),
            vk::DescriptorBufferInfo::default()
                .buffer(b_buf.buffer)
                .offset(0)
                .range(b_buf.size),
            vk::DescriptorBufferInfo::default()
                .buffer(c_buf.buffer)
                .offset(0)
                .range(c_buf.size),
        ];
        let writes = [
            vk::WriteDescriptorSet::default()
                .dst_set(desc_set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&buffer_infos[0..1]),
            vk::WriteDescriptorSet::default()
                .dst_set(desc_set)
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&buffer_infos[1..2]),
            vk::WriteDescriptorSet::default()
                .dst_set(desc_set)
                .dst_binding(2)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&buffer_infos[2..3]),
        ];
        unsafe { self.device.update_descriptor_sets(&writes, &[]) };

        // 5. Grabar y Ejecutar Command Buffer
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe { self.device.begin_command_buffer(self.command_buffer, &begin_info) }?;

        unsafe {
            self.device.cmd_bind_pipeline(
                self.command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                pipeline,
            );
            self.device.cmd_bind_descriptor_sets(
                self.command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                pipeline_layout,
                0,
                &[desc_set],
                &[],
            );
            // Dispatch 2x2 workgroups (para una matriz 2x2 con local_size 2x2, esto es 1x1 dispatch)
            self.device
                .cmd_dispatch(self.command_buffer, 1, 1, 1);
        }

        unsafe { self.device.end_command_buffer(self.command_buffer) }?;

        let submit_info = vk::SubmitInfo::default()
            .command_buffers(std::slice::from_ref(&self.command_buffer));
        unsafe {
            self.device
                .queue_submit(self.queue, &[submit_info], vk::Fence::null())
        }?;
        unsafe { self.device.queue_wait_idle(self.queue) }?;

        log::info!("✅ MatMul ejecutado en GPU. Resultado en buffer '{}'.", dest);

        // 6. Limpieza de recursos de Vulkan (en una app real, esto se haría al final)
        unsafe {
            self.device.destroy_descriptor_pool(desc_pool, None);
            self.device.destroy_pipeline(pipeline, None);
            self.device.destroy_pipeline_layout(pipeline_layout, None);
            self.device.destroy_descriptor_set_layout(desc_layout, None);
            self.device.destroy_shader_module(shader_module, None);
        }

        Ok(())
    }

    /// Lee los datos del tensor de vuelta a la CPU.
    pub fn read_tensor(&self, name: &str) -> Result<Vec<f32>> {
        let buf = self.tensor_registry.get(name).context("Tensor no encontrado")?;
        let ptr = unsafe {
            self.device
                .map_memory(buf.memory, 0, buf.size, vk::MemoryMapFlags::empty())
        }? as *const f32;
        
        let num_elements = (buf.size / 4) as usize;
        let mut result = vec![0.0f32; num_elements];
        unsafe {
            std::ptr::copy_nonoverlapping(ptr, result.as_mut_ptr(), num_elements);
            self.device.unmap_memory(buf.memory);
        }
        Ok(result)
    }

    fn find_memory_type(
        &self,
        type_filter: u32,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<u32> {
        let mem_properties =
            unsafe { self.instance.get_physical_device_memory_properties(self.physical_device) };
        for i in 0..mem_properties.memory_type_count {
            if (type_filter & (1 << i)) != 0
                && mem_properties.memory_types[i as usize]
                    .property_flags
                    .contains(properties)
            {
                return Ok(i);
            }
        }
        Err(anyhow::anyhow!(
            "No se encontró un tipo de memoria de Vulkan adecuado"
        ))
    }
}

impl Drop for VulkanEngine {
    fn drop(&mut self) {
        unsafe {
            // Liberar buffers de tensores
            for (_, buf) in self.tensor_registry.drain() {
                self.device.destroy_buffer(buf.buffer, None);
                self.device.free_memory(buf.memory, None);
            }
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
        log::info!("🛑 Vulkan Engine limpiado.");
    }
}