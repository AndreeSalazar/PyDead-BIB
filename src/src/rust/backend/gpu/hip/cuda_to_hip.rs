// ============================================================
// ADead-BIB - CUDA to HIP Translator
// ============================================================
// Traduce código CUDA a HIP para portabilidad
// Permite ejecutar el mismo código en NVIDIA y AMD
//
// Basado en las reglas de hipify-perl/hipify-clang
// ============================================================

use std::collections::HashMap;

/// Traductor CUDA → HIP
pub struct CudaToHipTranslator {
    /// Mapeo de funciones CUDA → HIP
    function_map: HashMap<&'static str, &'static str>,
    /// Mapeo de tipos CUDA → HIP
    type_map: HashMap<&'static str, &'static str>,
    /// Mapeo de macros CUDA → HIP
    macro_map: HashMap<&'static str, &'static str>,
}

impl CudaToHipTranslator {
    pub fn new() -> Self {
        let mut function_map = HashMap::new();
        let mut type_map = HashMap::new();
        let mut macro_map = HashMap::new();

        // === Runtime API ===
        function_map.insert("cudaMalloc", "hipMalloc");
        function_map.insert("cudaFree", "hipFree");
        function_map.insert("cudaMemcpy", "hipMemcpy");
        function_map.insert("cudaMemcpyAsync", "hipMemcpyAsync");
        function_map.insert("cudaMemset", "hipMemset");
        function_map.insert("cudaMemsetAsync", "hipMemsetAsync");
        function_map.insert("cudaDeviceSynchronize", "hipDeviceSynchronize");
        function_map.insert("cudaStreamSynchronize", "hipStreamSynchronize");
        function_map.insert("cudaGetDeviceCount", "hipGetDeviceCount");
        function_map.insert("cudaSetDevice", "hipSetDevice");
        function_map.insert("cudaGetDevice", "hipGetDevice");
        function_map.insert("cudaGetDeviceProperties", "hipGetDeviceProperties");

        // === Memory Types ===
        function_map.insert("cudaMemcpyHostToDevice", "hipMemcpyHostToDevice");
        function_map.insert("cudaMemcpyDeviceToHost", "hipMemcpyDeviceToHost");
        function_map.insert("cudaMemcpyDeviceToDevice", "hipMemcpyDeviceToDevice");
        function_map.insert("cudaMemcpyHostToHost", "hipMemcpyHostToHost");

        // === Streams ===
        function_map.insert("cudaStreamCreate", "hipStreamCreate");
        function_map.insert("cudaStreamDestroy", "hipStreamDestroy");
        function_map.insert("cudaStreamWaitEvent", "hipStreamWaitEvent");

        // === Events ===
        function_map.insert("cudaEventCreate", "hipEventCreate");
        function_map.insert("cudaEventDestroy", "hipEventDestroy");
        function_map.insert("cudaEventRecord", "hipEventRecord");
        function_map.insert("cudaEventSynchronize", "hipEventSynchronize");
        function_map.insert("cudaEventElapsedTime", "hipEventElapsedTime");

        // === Error Handling ===
        function_map.insert("cudaGetLastError", "hipGetLastError");
        function_map.insert("cudaPeekAtLastError", "hipPeekAtLastError");
        function_map.insert("cudaGetErrorString", "hipGetErrorString");
        function_map.insert("cudaGetErrorName", "hipGetErrorName");

        // === Unified Memory ===
        function_map.insert("cudaMallocManaged", "hipMallocManaged");
        function_map.insert("cudaMemPrefetchAsync", "hipMemPrefetchAsync");

        // === Types ===
        type_map.insert("cudaError_t", "hipError_t");
        type_map.insert("cudaSuccess", "hipSuccess");
        type_map.insert("cudaStream_t", "hipStream_t");
        type_map.insert("cudaEvent_t", "hipEvent_t");
        type_map.insert("cudaDeviceProp", "hipDeviceProp_t");
        type_map.insert("cudaMemcpyKind", "hipMemcpyKind");

        // === Macros ===
        macro_map.insert("__CUDA_ARCH__", "__HIP_DEVICE_COMPILE__");
        macro_map.insert("CUDA_VERSION", "HIP_VERSION");

        // === cuBLAS → hipBLAS ===
        function_map.insert("cublasCreate", "hipblasCreate");
        function_map.insert("cublasDestroy", "hipblasDestroy");
        function_map.insert("cublasSgemm", "hipblasSgemm");
        function_map.insert("cublasDgemm", "hipblasDgemm");
        function_map.insert("cublasSaxpy", "hipblasSaxpy");
        function_map.insert("cublasDaxpy", "hipblasDaxpy");

        type_map.insert("cublasHandle_t", "hipblasHandle_t");
        type_map.insert("cublasStatus_t", "hipblasStatus_t");
        type_map.insert("CUBLAS_STATUS_SUCCESS", "HIPBLAS_STATUS_SUCCESS");

        // === cuFFT → hipFFT ===
        function_map.insert("cufftPlan1d", "hipfftPlan1d");
        function_map.insert("cufftPlan2d", "hipfftPlan2d");
        function_map.insert("cufftPlan3d", "hipfftPlan3d");
        function_map.insert("cufftExecC2C", "hipfftExecC2C");
        function_map.insert("cufftExecR2C", "hipfftExecR2C");
        function_map.insert("cufftExecC2R", "hipfftExecC2R");
        function_map.insert("cufftDestroy", "hipfftDestroy");

        type_map.insert("cufftHandle", "hipfftHandle");
        type_map.insert("cufftComplex", "hipfftComplex");
        type_map.insert("cufftReal", "hipfftReal");

        Self {
            function_map,
            type_map,
            macro_map,
        }
    }

    /// Traduce código CUDA a HIP
    pub fn translate(&self, cuda_code: &str) -> String {
        let mut hip_code = cuda_code.to_string();

        // Reemplazar includes
        hip_code = hip_code.replace("#include <cuda_runtime.h>", "#include <hip/hip_runtime.h>");
        hip_code = hip_code.replace("#include <cuda.h>", "#include <hip/hip_runtime.h>");
        hip_code = hip_code.replace("#include <cublas_v2.h>", "#include <hipblas/hipblas.h>");
        hip_code = hip_code.replace("#include <cufft.h>", "#include <hipfft/hipfft.h>");

        // Reemplazar funciones (ordenar por longitud para evitar reemplazos parciales)
        let mut functions: Vec<_> = self.function_map.iter().collect();
        functions.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

        for (cuda, hip) in functions {
            hip_code = hip_code.replace(cuda, hip);
        }

        // Reemplazar tipos
        let mut types: Vec<_> = self.type_map.iter().collect();
        types.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

        for (cuda, hip) in types {
            hip_code = hip_code.replace(cuda, hip);
        }

        // Reemplazar macros
        for (cuda, hip) in &self.macro_map {
            hip_code = hip_code.replace(cuda, hip);
        }

        // Traducir kernel launch syntax
        hip_code = self.translate_kernel_launches(&hip_code);

        hip_code
    }

    /// Traduce la sintaxis de lanzamiento de kernels
    /// kernel<<<grid, block, shMem, stream>>>(args) → hipLaunchKernelGGL(...)
    fn translate_kernel_launches(&self, code: &str) -> String {
        // Regex simple para detectar kernel launches
        // En producción usaríamos un parser más robusto
        let mut result = code.to_string();

        // Buscar patrones <<<...>>>
        // Por simplicidad, dejamos la sintaxis CUDA que HIP también soporta
        // Solo añadimos un comentario
        if result.contains("<<<") {
            result = format!(
                "// Note: HIP supports CUDA kernel launch syntax (<<<>>>)\n{}",
                result
            );
        }

        result
    }

    /// Genera un header de compatibilidad para usar código CUDA en HIP
    pub fn generate_compat_header(&self) -> String {
        let mut header = String::new();

        header.push_str("// ADead-BIB CUDA/HIP Compatibility Header\n");
        header.push_str("// Allows CUDA code to compile with HIP\n\n");

        header.push_str("#ifndef ADEAD_CUDA_HIP_COMPAT_H\n");
        header.push_str("#define ADEAD_CUDA_HIP_COMPAT_H\n\n");

        header.push_str("#ifdef __HIP_PLATFORM_AMD__\n");
        header.push_str("    #include <hip/hip_runtime.h>\n");
        header.push_str("#else\n");
        header.push_str("    #include <cuda_runtime.h>\n");
        header.push_str("#endif\n\n");

        // Macros de compatibilidad
        header.push_str("// Compatibility macros\n");
        header.push_str("#ifdef __HIP_PLATFORM_AMD__\n");

        for (cuda, hip) in &self.function_map {
            header.push_str(&format!("    #define {} {}\n", cuda, hip));
        }

        header.push_str("#endif\n\n");

        header.push_str("#endif // ADEAD_CUDA_HIP_COMPAT_H\n");

        header
    }
}

impl Default for CudaToHipTranslator {
    fn default() -> Self {
        Self::new()
    }
}

/// Traduce un archivo CUDA a HIP
pub fn translate_cuda_file(cuda_code: &str) -> String {
    let translator = CudaToHipTranslator::new();
    translator.translate(cuda_code)
}

/// Genera código que funciona en ambos backends
pub fn generate_portable_code(
    kernel_name: &str,
    kernel_body: &str,
    params: &[(&str, &str)],
) -> String {
    let mut code = String::new();

    // Header portable
    code.push_str("// ADead-BIB Portable GPU Code\n");
    code.push_str("// Works on both CUDA (NVIDIA) and HIP (AMD)\n\n");

    code.push_str("#if defined(__HIPCC__) || defined(__HIP_PLATFORM_AMD__)\n");
    code.push_str("    #include <hip/hip_runtime.h>\n");
    code.push_str("    #define ADEAD_DEVICE __device__\n");
    code.push_str("    #define ADEAD_GLOBAL __global__\n");
    code.push_str("    #define ADEAD_SHARED __shared__\n");
    code.push_str("    #define ADEAD_SYNCTHREADS() __syncthreads()\n");
    code.push_str("#else\n");
    code.push_str("    #include <cuda_runtime.h>\n");
    code.push_str("    #define ADEAD_DEVICE __device__\n");
    code.push_str("    #define ADEAD_GLOBAL __global__\n");
    code.push_str("    #define ADEAD_SHARED __shared__\n");
    code.push_str("    #define ADEAD_SYNCTHREADS() __syncthreads()\n");
    code.push_str("#endif\n\n");

    // Kernel
    code.push_str("ADEAD_GLOBAL void ");
    code.push_str(kernel_name);
    code.push_str("(");

    let param_strs: Vec<String> = params
        .iter()
        .map(|(ty, name)| format!("{} {}", ty, name))
        .collect();
    code.push_str(&param_strs.join(", "));

    code.push_str(") {\n");
    code.push_str(kernel_body);
    code.push_str("}\n");

    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_basic() {
        let translator = CudaToHipTranslator::new();

        let cuda = "cudaMalloc(&ptr, size);";
        let hip = translator.translate(cuda);

        assert!(hip.contains("hipMalloc"));
        assert!(!hip.contains("cudaMalloc"));
    }

    #[test]
    fn test_translate_includes() {
        let translator = CudaToHipTranslator::new();

        let cuda = "#include <cuda_runtime.h>\n#include <cublas_v2.h>";
        let hip = translator.translate(cuda);

        assert!(hip.contains("hip/hip_runtime.h"));
        assert!(hip.contains("hipblas/hipblas.h"));
    }

    #[test]
    fn test_translate_types() {
        let translator = CudaToHipTranslator::new();

        let cuda = "cudaError_t err = cudaSuccess;";
        let hip = translator.translate(cuda);

        assert!(hip.contains("hipError_t"));
        assert!(hip.contains("hipSuccess"));
    }

    #[test]
    fn test_portable_code() {
        let code = generate_portable_code(
            "vectorAdd",
            "    int i = blockDim.x * blockIdx.x + threadIdx.x;\n    C[i] = A[i] + B[i];\n",
            &[("float*", "A"), ("float*", "B"), ("float*", "C")],
        );

        assert!(code.contains("ADEAD_GLOBAL"));
        assert!(code.contains("vectorAdd"));
    }

    #[test]
    fn test_full_translation() {
        let cuda_code = r#"
#include <cuda_runtime.h>
#include <stdio.h>

__global__ void vectorAdd(float *A, float *B, float *C, int n) {
    int i = blockDim.x * blockIdx.x + threadIdx.x;
    if (i < n) C[i] = A[i] + B[i];
}

int main() {
    float *d_A, *d_B, *d_C;
    cudaMalloc(&d_A, 1024);
    cudaMalloc(&d_B, 1024);
    cudaMalloc(&d_C, 1024);
    
    vectorAdd<<<4, 256>>>(d_A, d_B, d_C, 1024);
    cudaDeviceSynchronize();
    
    cudaFree(d_A);
    cudaFree(d_B);
    cudaFree(d_C);
    return 0;
}
"#;

        let hip_code = translate_cuda_file(cuda_code);

        assert!(hip_code.contains("hip/hip_runtime.h"));
        assert!(hip_code.contains("hipMalloc"));
        assert!(hip_code.contains("hipFree"));
        assert!(hip_code.contains("hipDeviceSynchronize"));
    }
}
