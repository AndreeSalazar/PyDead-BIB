use ash::vk;
use std::ffi::{CString, CStr};
use std::collections::HashMap;
use pyd_core::TensorMeta;
use anyhow::{Result, anyhow};

// Include the SPIR-V shader at compile time
const SHADER_CODE: &[u8] = include_bytes!("../../../shaders/matmul.comp.spv");

/// Estructura que representa un buffer de tensor en la GPU
struct TensorBuffer {
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    size: vk::DeviceSize,
    meta: TensorMeta,
}

/// Estructura principal que encapsula el contexto de Vulkan para PyDead-BIB
pub struct VulkanEngine {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub queue: vk::Queue,
    pub queue_family_index: u32,
    pub command_pool: vk::CommandPool,
    
    // Almacenamiento de tensores en la GPU
    tensors: HashMap<String, TensorBuffer>,
}

impl VulkanEngine {
    /// Inicializa Vulkan, busca una cola de cómputo y crea el dispositivo lógico
    pub fn new() -> Result<Self> {
        // 1. Cargar la entrada de Vulkan
        let entry = unsafe { ash::Entry::load() }
            .map_err(|e| anyhow!("Failed to load Vulkan entry: {:?}", e))?;

        // 2. Información de la aplicación
        let app_name = CString::new("PyDead-BIB").unwrap();
        let app_info = vk::ApplicationInfo::default()
            .application_name(app_name.as_c_str())
            .application_version(vk::API_VERSION_1_0)
            .api_version(vk::API_VERSION_1_0);

        // 3. Crear la Instancia
        let instance_info = vk::InstanceCreateInfo::default().application_info(&app_info);
        let instance = unsafe { entry.create_instance(&instance_info, None) }
            .map_err(|e| anyhow!("Failed to create Vulkan instance: {:?}", e))?;

        // 4. Seleccionar Dispositivo Físico (GPU)
        let pdevices = unsafe { instance.enumerate_physical_devices() }?;
        let physical_device = pdevices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No se encontraron dispositivos físicos Vulkan"))?;

        // 5. Buscar una familia de colas que soporte COMPUTE
        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
        
        let queue_family_index = queue_families
            .iter()
            .position(|q| q.queue_flags.contains(vk::QueueFlags::COMPUTE))
            .ok_or_else(|| anyhow!("No se encontró una familia de colas con soporte COMPUTE"))? as u32;

        // 6. Crear Dispositivo Lógico
        let queue_priorities = [1.0f32];
        let queue_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family_index)
            .queue_priorities(&queue_priorities);

        let queue_create_infos = [queue_info];
        let device_info = vk::DeviceCreateInfo::default().queue_create_infos(&queue_create_infos);
        let device = unsafe { instance.create_device(physical_device, &device_info, None) }?;
        
        // 7. Obtener la cola de cómputo
        let queue = unsafe { device.get_device_queue(queue_family_index, 0) };

        // 8. Crear Command Pool
        let pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let command_pool = unsafe { device.create_command_pool(&pool_info, None) }?;

        Ok(Self {
            entry,
            instance,
            physical_device,
            device,
            queue,
            queue_family_index,
            command_pool,
            tensors: HashMap::new(),
        })
    }

    /// Helper para encontrar el tipo de memoria adecuado
    unsafe fn find_memory_type(
        &self,
        type_filter: u32,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<u32> {
        let mem_props = self.instance.get_physical_device_memory_properties(self.physical_device);
        for i in 0..mem_props.memory_type_count {
            if (type_filter & (1 << i)) != 0 
                && mem_props.memory_types[i as usize].property_flags.contains(properties) 
            {
                return Ok(i);
            }
        }
        Err(anyhow!("Failed to find suitable memory type"))
    }

    /// Helper para crear un buffer y asignar memoria
    unsafe fn create_buffer(
        &self,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<(vk::Buffer, vk::DeviceMemory)> {
        let buffer_info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        
        let buffer = self.device.create_buffer(&buffer_info, None)?;
        let mem_reqs = self.device.get_buffer_memory_requirements(buffer);
        
        let memory_type = self.find_memory_type(mem_reqs.memory_type_bits, properties)?;
        
        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_reqs.size)
            .memory_type_index(memory_type);
        
        let memory = self.device.allocate_memory(&alloc_info, None)?;
        self.device.bind_buffer_memory(buffer, memory, 0)?;
        
        Ok((buffer, memory))
    }

    /// Crea un buffer en la GPU para un tensor y copia los datos iniciales
    pub fn create_tensor_buffer(&mut self, meta: &TensorMeta, data: &[f32]) -> Result<()> {
        let size = (data.len() * std::mem::size_of::<f32>()) as vk::DeviceSize;
        
        unsafe {
            // Creamos el buffer con HOST_VISIBLE y HOST_COHERENT para poder mapearlo fácilmente
            let (buffer, memory) = self.create_buffer(
                size,
                vk::BufferUsageFlags::STORAGE_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            
            // Mapeamos la memoria, copiamos los datos y desmapeamos
            let ptr = self.device.map_memory(memory, 0, size, vk::MemoryMapFlags::empty())?;
            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut f32, data.len());
            self.device.unmap_memory(memory);
            
            self.tensors.insert(meta.name.clone(), TensorBuffer {
                buffer,
                memory,
                size,
                meta: meta.clone(),
            });
        }
        
        println!("   📦 [Vulkan] Allocated GPU buffer for tensor '{}' (shape: {:?})", meta.name, meta.shape);
        Ok(())
    }

    /// Ejecuta una multiplicación de matrices (MatMul) usando un Compute Shader en la GPU
    pub fn execute_matmul(&mut self, lhs_name: &str, rhs_name: &str, dest_name: &str) -> Result<()> {
        let lhs_buf = self.tensors.get(lhs_name).ok_or_else(|| anyhow!("Tensor '{}' not found", lhs_name))?;
        let rhs_buf = self.tensors.get(rhs_name).ok_or_else(|| anyhow!("Tensor '{}' not found", rhs_name))?;
        let dest_buf = self.tensors.get(dest_name).ok_or_else(|| anyhow!("Tensor '{}' not found", dest_name))?;
        
        let lhs_handle = lhs_buf.buffer;
        let rhs_handle = rhs_buf.buffer;
        let dest_handle = dest_buf.buffer;
        
        // Calculamos las dimensiones de dispatch basadas en la forma del tensor
        // NOTA: El shader actual (matmul.comp) está hardcodeado para matrices 2x2.
        // Para soportar matrices NxM, el shader debería recibir las dimensiones como push constants o uniforms.
        // El shader tiene local_size_x = 2, local_size_y = 2
        let cols = dest_buf.meta.shape.last().copied().unwrap_or(1) as u32;
        let rows = dest_buf.meta.shape.get(dest_buf.meta.shape.len().wrapping_sub(2)).copied().unwrap_or(1) as u32;
        
        let dispatch_x = (cols + 1) / 2;
        let dispatch_y = (rows + 1) / 2;
        
        unsafe {
            // 1. Crear Shader Module
            let shader_code = std::slice::from_raw_parts(
                SHADER_CODE.as_ptr() as *const u32,
                SHADER_CODE.len() / std::mem::size_of::<u32>(),
            );
            let shader_module_info = vk::ShaderModuleCreateInfo::default().code(shader_code);
            let shader_module = self.device.create_shader_module(&shader_module_info, None)?;
            
            // 2. Crear Descriptor Set Layout
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
            let descriptor_set_layout = self.device.create_descriptor_set_layout(&layout_info, None)?;
            
            // 3. Crear Pipeline Layout
            let pipeline_layouts = [descriptor_set_layout];
            let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
                .set_layouts(&pipeline_layouts);
            let pipeline_layout = self.device.create_pipeline_layout(&pipeline_layout_info, None)?;
            
            // 4. Crear Compute Pipeline
            let stage = vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::COMPUTE)
                .module(shader_module)
                .name(CStr::from_bytes_with_nul(b"main\0").unwrap());
            
            let pipeline_info = vk::ComputePipelineCreateInfo::default()
                .stage(stage)
                .layout(pipeline_layout);
            
            let pipelines = self.device.create_compute_pipelines(
                vk::PipelineCache::null(),
                &[pipeline_info],
                None,
            ).map_err(|(_, e)| anyhow!("Failed to create compute pipeline: {:?}", e))?;
            let pipeline = pipelines[0];
            
            // 5. Crear Descriptor Pool
            let pool_sizes = [
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(3),
            ];
            let pool_info = vk::DescriptorPoolCreateInfo::default()
                .pool_sizes(&pool_sizes)
                .max_sets(1);
            let descriptor_pool = self.device.create_descriptor_pool(&pool_info, None)?;
            
            // 6. Allocate Descriptor Set
            let alloc_layouts = [descriptor_set_layout];
            let alloc_info = vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(descriptor_pool)
                .set_layouts(&alloc_layouts);
            let descriptor_sets = self.device.allocate_descriptor_sets(&alloc_info)?;
            let descriptor_set = descriptor_sets[0];
            
            // 7. Update Descriptor Set
            let buffer_infos = [
                vk::DescriptorBufferInfo::default()
                    .buffer(lhs_handle)
                    .offset(0)
                    .range(vk::WHOLE_SIZE),
                vk::DescriptorBufferInfo::default()
                    .buffer(rhs_handle)
                    .offset(0)
                    .range(vk::WHOLE_SIZE),
                vk::DescriptorBufferInfo::default()
                    .buffer(dest_handle)
                    .offset(0)
                    .range(vk::WHOLE_SIZE),
            ];
            
            let descriptor_writes = [
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&buffer_infos[0..1]),
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_set)
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&buffer_infos[1..2]),
                vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_set)
                    .dst_binding(2)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&buffer_infos[2..3]),
            ];
            
            self.device.update_descriptor_sets(&descriptor_writes, &[]);
            
            // 8. Allocate Command Buffer
            let cmd_alloc_info = vk::CommandBufferAllocateInfo::default()
                .command_pool(self.command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1);
            let cmd_buffers = self.device.allocate_command_buffers(&cmd_alloc_info)?;
            let cmd_buffer = cmd_buffers[0];
            
            // 9. Record Command Buffer
            let cmd_begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.device.begin_command_buffer(cmd_buffer, &cmd_begin_info)?;
            
            self.device.cmd_bind_pipeline(cmd_buffer, vk::PipelineBindPoint::COMPUTE, pipeline);
            self.device.cmd_bind_descriptor_sets(
                cmd_buffer,
                vk::PipelineBindPoint::COMPUTE,
                pipeline_layout,
                0,
                &[descriptor_set],
                &[],
            );
            
            self.device.cmd_dispatch(cmd_buffer, dispatch_x, dispatch_y, 1);
            
            self.device.end_command_buffer(cmd_buffer)?;
            
            // 10. Submit Command Buffer
            let submit_cmd_buffers = [cmd_buffer];
            let submit_info = vk::SubmitInfo::default().command_buffers(&submit_cmd_buffers);
            self.device.queue_submit(self.queue, &[submit_info], vk::Fence::null())?;
            self.device.queue_wait_idle(self.queue)?;
            
            // 11. Clean up temporary objects
            self.device.free_command_buffers(self.command_pool, &[cmd_buffer]);
            self.device.destroy_descriptor_pool(descriptor_pool, None);
            self.device.destroy_pipeline(pipeline, None);
            self.device.destroy_pipeline_layout(pipeline_layout, None);
            self.device.destroy_descriptor_set_layout(descriptor_set_layout, None);
            self.device.destroy_shader_module(shader_module, None);
        }
        
        println!("   🔥 [Vulkan] Dispatched MatMul Compute Shader: {} = {} @ {}", dest_name, lhs_name, rhs_name);
        Ok(())
    }

    /// Lee los datos de un tensor desde la memoria de la GPU
    pub fn read_tensor(&mut self, name: &str) -> Result<Vec<f32>> {
        let buf = self.tensors.get(name).ok_or_else(|| anyhow!("Tensor '{}' not found", name))?;
        let size = buf.size;
        let num_elements = size as usize / std::mem::size_of::<f32>();
        let mut result = vec![0.0f32; num_elements];
        
        unsafe {
            let ptr = self.device.map_memory(buf.memory, 0, size, vk::MemoryMapFlags::empty())?;
            std::ptr::copy_nonoverlapping(ptr as *const f32, result.as_mut_ptr(), num_elements);
            self.device.unmap_memory(buf.memory);
        }
        
        Ok(result)
    }
}

impl Drop for VulkanEngine {
    fn drop(&mut self) {
        unsafe {
            // Clean up tensors
            for (_, buf) in self.tensors.drain() {
                self.device.destroy_buffer(buf.buffer, None);
                self.device.free_memory(buf.memory, None);
            }
            
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}
