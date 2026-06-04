use ash::vk;
use std::ffi::CString;
use std::collections::HashMap;
use pyd_core::{TensorMeta, TensorDType};
use anyhow::{Result, anyhow};

/// Estructura principal que encapsula el contexto de Vulkan para PyDead-BIB
pub struct VulkanEngine {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub queue: vk::Queue,
    pub queue_family_index: u32,
    pub command_pool: vk::CommandPool,
    
    // Almacenamiento temporal en CPU para tensores hasta que los buffers de GPU estén completos
    tensors: HashMap<String, (TensorMeta, Vec<f32>)>,
}

impl VulkanEngine {
    /// Inicializa Vulkan, busca una cola de cómputo y crea el dispositivo lógico
    pub fn new() -> Result<Self> {
        // 1. Cargar la entrada de Vulkan
        let entry = unsafe { ash::Entry::load() }
            .map_err(|e| anyhow!("Failed to load Vulkan entry: {:?}", e))?;

        // 2. Información de la aplicación (Sintaxis correcta con ::default())
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

        // 6. Crear Dispositivo Lógico (Extendiendo vida de arrays temporales)
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

    /// Crea un buffer para un tensor (actualmente almacenado en CPU como stub)
    pub fn create_tensor_buffer(&mut self, meta: &TensorMeta, data: &[f32]) -> Result<()> {
        println!("   📦 [Vulkan Stub] Allocating memory for tensor '{}' (shape: {:?})", meta.name, meta.shape);
        self.tensors.insert(meta.name.clone(), (meta.clone(), data.to_vec()));
        Ok(())
    }

    /// Ejecuta una multiplicación de matrices (MatMul)
    /// NOTA: Esta es una implementación CPU fallback para validar el pipeline.
    /// En el futuro, esto dispatchará un compute shader en la GPU.
    pub fn execute_matmul(&mut self, lhs_name: &str, rhs_name: &str, dest_name: &str) -> Result<()> {
        println!("   🔥 [Vulkan Stub] Executing MatMul: {} = {} @ {}", dest_name, lhs_name, rhs_name);
        
        let lhs = self.tensors.get(lhs_name)
            .ok_or_else(|| anyhow!("Tensor '{}' not found", lhs_name))?.1.clone();
        let rhs = self.tensors.get(rhs_name)
            .ok_or_else(|| anyhow!("Tensor '{}' not found", rhs_name))?.1.clone();
        
        // Asumimos matrices 2x2 para este stub
        // A = [[lhs[0], lhs[1]], [lhs[2], lhs[3]]]
        // B = [[rhs[0], rhs[1]], [rhs[2], rhs[3]]]
        // C = A @ B
        let c0 = lhs[0]*rhs[0] + lhs[1]*rhs[2];
        let c1 = lhs[0]*rhs[1] + lhs[1]*rhs[3];
        let c2 = lhs[2]*rhs[0] + lhs[3]*rhs[2];
        let c3 = lhs[2]*rhs[1] + lhs[3]*rhs[3];
        
        let result = vec![c0, c1, c2, c3];
        
        // Guardar resultado en el tensor de destino
        if let Some((meta, _)) = self.tensors.get_mut(dest_name) {
            let dest_meta = meta.clone();
            self.tensors.insert(dest_name.to_string(), (dest_meta, result));
        } else {
            // Si el destino no existe, crearlo con metadatos por defecto
            let meta = TensorMeta {
                name: dest_name.to_string(),
                shape: vec![2, 2],
                dtype: TensorDType::Float32,
            };
            self.tensors.insert(dest_name.to_string(), (meta, result));
        }
        
        Ok(())
    }

    /// Lee los datos de un tensor desde la memoria (CPU fallback)
    pub fn read_tensor(&self, name: &str) -> Result<Vec<f32>> {
        self.tensors.get(name)
            .map(|(_, data)| data.clone())
            .ok_or_else(|| anyhow!("Tensor '{}' not found", name))
    }
}

impl Drop for VulkanEngine {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}
