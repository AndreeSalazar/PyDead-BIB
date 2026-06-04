//! Punto de entrada unificado de PyDead-BIB v6.0

use anyhow::Result;
use pyd_gc::GcArena;
use pyd_parser::parse_and_lower;
use pyd_vulkan::VulkanEngine;

fn main() -> Result<()> {
    env_logger::init();
    println!("💀🦈 PyDead-BIB v6.0 'Vulkan Core' - Construyendo el futuro...");

    // 1. Inicializar el Garbage Collector
    let mut gc = GcArena::new();
    println!("✅ GC Nativo inicializado.");

    // 2. Inicializar el Motor Vulkan (Requiere drivers de Vulkan instalados)
    let vk_engine = match VulkanEngine::new() {
        Ok(engine) => {
            println!("✅ Motor Vulkan inicializado correctamente.");
            Some(engine)
        }
        Err(e) => {
            eprintln!("⚠️  No se pudo inicializar Vulkan: {}. Ejecutando en modo simulación.", e);
            None
        }
    };

    // 3. Código Python de prueba (IA: Multiplicación de matrices)
    let python_code = r#"
# Definimos tensores (simplificado para el ejemplo de parsing)
a = Tensor([2, 2])
b = Tensor([2, 2])
# Operación de IA nativa
c = a @ b
"#;

    println!("\n📜 Código Python a compilar:\n{}", python_code);

    // 4. Frontend: Parsear y bajar a IR
    let instructions = parse_and_lower(python_code)?;
    println!("✅ Código parseado y convertido a IR. Instrucciones: {:?}", instructions);

    // 5. Backend: Ejecutar el IR
    for instr in instructions {
        match instr {
            pyd_core::Instruction::MatMul { dest, lhs, rhs } => {
                if let Some(ref vk) = vk_engine {
                    vk.execute_matmul(lhs, rhs, dest)?;
                } else {
                    println!("⚠️  [SIMULACIÓN] Ejecutando MatMul en CPU (Vulkan no disponible).");
                }
            }
            _ => {}
        }
    }

    // 6. Limpieza final
    gc.collect();
    println!("\n🎯 Ejecución finalizada. Recursos liberados por el GC.");
    Ok(())
}
