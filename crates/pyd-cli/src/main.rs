//! Punto de entrada unificado de PyDead-BIB v6.0
//! Demuestra el flujo completo: Python -> IR -> Vulkan GPU -> Resultado CPU.

use anyhow::Result;
use pyd_core::Instruction;
use pyd_parser::parse_and_lower;
use pyd_vulkan::VulkanEngine;

fn main() -> Result<()> {
    env_logger::init();
    println!("💀🦈 PyDead-BIB v6.0 - Enfoque Total en IA + Vulkan");

    // 1. Inicializar el Motor Vulkan
    let mut vk_engine = VulkanEngine::new()?;
    println!("✅ Motor Vulkan inicializado correctamente.");

    // 2. Código Python de prueba (IA: Multiplicación de matrices 2x2)
    let python_code = r#"
a = Tensor([2, 2])
b = Tensor([2, 2])
c = a @ b
"#;

    println!("\n📜 Código Python a compilar:\n{}", python_code);

    // 3. Frontend: Parsear y bajar a IR
    let instructions = parse_and_lower(python_code)?;
    println!("✅ Código parseado y convertido a IR. Total instrucciones: {}", instructions.len());

    // 4. Backend: Ejecutar el IR en la GPU
    for instr in instructions {
        match instr {
            Instruction::CreateTensor { meta } => {
                println!("\n📦 Creando Tensor en GPU: '{}' con forma {:?}", meta.name, meta.shape);
                
                // Datos de prueba: Matrices 2x2
                // A = [[1.0, 2.0], [3.0, 4.0]]
                // B = [[5.0, 6.0], [7.0, 8.0]]
                let dummy_data = match meta.name.as_str() {
                    "a" => vec![1.0, 2.0, 3.0, 4.0],
                    "b" => vec![5.0, 6.0, 7.0, 8.0],
                    _ => vec![0.0; meta.shape.iter().product()], // Tensor de salida (c)
                };
                
                vk_engine.create_tensor_buffer(&meta, &dummy_data)?;
            }
            Instruction::MatMul { dest_name, lhs_name, rhs_name } => {
                println!("\n🔥 Ejecutando operación: {} = {} @ {}", dest_name, lhs_name, rhs_name);
                vk_engine.execute_matmul(&lhs_name, &rhs_name, &dest_name)?;
            }
        }
    }

    // 5. Leer el resultado de la GPU de vuelta a la CPU
    println!("\n📥 Leyendo resultado del Tensor 'c' desde la GPU...");
    match vk_engine.read_tensor("c") {
        Ok(result) => {
            println!("✅ Resultado de MatMul (C = A @ B):");
            println!("   [ {:.1}, {:.1} ]", result[0], result[1]);
            println!("   [ {:.1}, {:.1} ]", result[2], result[3]);
            
            // Verificación matemática:
            // A = [[1, 2], [3, 4]]
            // B = [[5, 6], [7, 8]]
            // C = [[1*5+2*7, 1*6+2*8], [3*5+4*7, 3*6+4*8]] = [[19, 22], [43, 50]]
            if result[0] == 19.0 && result[1] == 22.0 && result[2] == 43.0 && result[3] == 50.0 {
                println!("\n🎉 ¡MATEMÁTICAS CORRECTAS! La GPU calculó la multiplicación de matrices perfectamente.");
            } else {
                println!("\n⚠️  El resultado no coincide con la expectativa. Revisa el shader.");
            }
        }
        Err(e) => {
            eprintln!("⚠️  No se pudo leer el tensor (¿El shader no se ejecutó?): {}", e);
        }
    }

    println!("\n🎯 Pipeline de Vulkan ejecutado. Tensores creados y calculados en memoria de GPU.");
    Ok(())
}