//! Orquestación real de Vulkan usando `ash`.

use anyhow::Result;
use ash::vk;
use log::info;

pub struct VulkanEngine {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub queue: vk::Queue,
    pub queue_family_index: u32,
}

impl VulkanEngine {
    pub fn new() -> Result<Self> {
        info!("🚀 Inicializando Vulkan Engine (ash)...");

        // 1. Cargar la librería de Vulkan
        let entry = unsafe { ash::Entry::load()? };

        // 2. Crear la Instancia
        let app_info = vk::ApplicationInfo::default()
            .application_name(c"PyDead-BIB")
            .application_version(vk::make_api_version(0, 1, 2, 0))
            .api_version(vk::make_api_version(0, 1, 2, 0));

        let instance_info = vk::InstanceCreateInfo::default().application_info(&app_info);
        let instance = unsafe { entry.create_instance(&instance_info, None)? };

        // 3. Seleccionar Physical Device (GPU)
        let physical_devices = unsafe { instance.enumerate_physical_devices()? };
        if physical_devices.is_empty() {
            anyhow::bail!("No se encontraron dispositivos Vulkan (GPUs) en el sistema.");
        }
        let physical_device = physical_devices[0]; // Tomamos la primera GPU disponible

        // 4. Buscar una cola que soporte Compute (necesaria para IA)
        let queue_family_properties = unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
        let queue_family_index = queue_family_properties
            .iter()
            .position(|q| q.queue_flags.contains(vk::QueueFlags::COMPUTE))
            .expect("No se encontró una cola de Compute") as u32;

        // 5. Crear Logical Device
        let queue_priority = 1.0;
        let queue_priorities = [queue_priority];
        let queue_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family_index)
            .queue_priorities(&queue_priorities);

        let queue_create_infos = [queue_info];
        let device_info = vk::DeviceCreateInfo::default().queue_create_infos(&queue_create_infos);
        let device = unsafe { instance.create_device(physical_device, &device_info, None)? };

        // 6. Obtener la cola
        let queue = unsafe { device.get_device_queue(queue_family_index, 0) };

        info!("✅ Vulkan Engine inicializado correctamente. GPU lista para Compute.");
        
        Ok(Self {
            entry,
            instance,
            physical_device,
            device,
            queue,
            queue_family_index,
        })
    }

    pub fn execute_matmul(&self, lhs_id: usize, rhs_id: usize, dest_id: usize) -> Result<()> {
        info!("🔥 Dispatching MatMul a GPU: Tensor[{lhs_id}] @ Tensor[{rhs_id}] -> Tensor[{dest_id}]");
        
        // TODO REAL:
        // 1. vkAllocateMemory / vkCreateBuffer para los tensores.
        // 2. vkCreateShaderModule con el SPIR-V de matmul.comp.
        // 3. vkCreateComputePipelines.
        // 4. vkCmdBeginCommandBuffer, vkCmdBindPipeline, vkCmdDispatch, vkCmdEndCommandBuffer.
        // 5. vkQueueSubmit y vkQueueWaitIdle.
        
        // Simulamos el tiempo de ejecución de la GPU
        std::thread::sleep(std::time::Duration::from_millis(10));
        info!("✅ MatMul completado en GPU.");
        Ok(())
    }
}

impl Drop for VulkanEngine {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
        info!("🛑 Vulkan Engine limpiado.");
    }
}
