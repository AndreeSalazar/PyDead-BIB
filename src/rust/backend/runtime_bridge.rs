// ============================================================
// PyDead-BIB Runtime 2.0 Bridge — IA Native Runtime Integration
// ============================================================
// Maps Runtime 2.0 C functions to x86-64 CALL instructions.
// The Runtime is compiled separately (GCC/MSVC) and linked
// via DLL or static linkage. This module declares all symbols.
// ============================================================

/// All Runtime 2.0 C function symbols that the compiler can emit CALLs to.
/// These correspond to the .h declarations in Runtime_2.0/
pub const RT_SYMBOLS: &[RuntimeSymbol] = &[
    // ── tensor/tensor.h ──────────────────────────────────────
    RuntimeSymbol { name: "tensor_create",    params: 2, returns: ReturnKind::Ptr,   module: "tensor" },
    RuntimeSymbol { name: "tensor_zeros",     params: 2, returns: ReturnKind::Ptr,   module: "tensor" },
    RuntimeSymbol { name: "tensor_ones",      params: 2, returns: ReturnKind::Ptr,   module: "tensor" },
    RuntimeSymbol { name: "tensor_free",      params: 1, returns: ReturnKind::Void,  module: "tensor" },
    RuntimeSymbol { name: "tensor_print",     params: 1, returns: ReturnKind::Void,  module: "tensor" },
    RuntimeSymbol { name: "tensor_get_2d",    params: 3, returns: ReturnKind::Float, module: "tensor" },
    RuntimeSymbol { name: "tensor_set_2d",    params: 4, returns: ReturnKind::Void,  module: "tensor" },
    RuntimeSymbol { name: "tensor_get_1d",    params: 2, returns: ReturnKind::Float, module: "tensor" },
    RuntimeSymbol { name: "tensor_set_1d",    params: 3, returns: ReturnKind::Void,  module: "tensor" },
    RuntimeSymbol { name: "tensor_add",       params: 2, returns: ReturnKind::Ptr,   module: "tensor" },
    RuntimeSymbol { name: "tensor_sub",       params: 2, returns: ReturnKind::Ptr,   module: "tensor" },
    RuntimeSymbol { name: "tensor_mul",       params: 2, returns: ReturnKind::Ptr,   module: "tensor" },
    RuntimeSymbol { name: "tensor_div",       params: 2, returns: ReturnKind::Ptr,   module: "tensor" },
    RuntimeSymbol { name: "tensor_scale",     params: 2, returns: ReturnKind::Ptr,   module: "tensor" },
    RuntimeSymbol { name: "tensor_matmul",    params: 2, returns: ReturnKind::Ptr,   module: "tensor" },
    RuntimeSymbol { name: "tensor_transpose", params: 1, returns: ReturnKind::Ptr,   module: "tensor" },
    RuntimeSymbol { name: "tensor_relu",      params: 1, returns: ReturnKind::Ptr,   module: "tensor" },
    RuntimeSymbol { name: "tensor_sigmoid",   params: 1, returns: ReturnKind::Ptr,   module: "tensor" },
    RuntimeSymbol { name: "tensor_tanh_act",  params: 1, returns: ReturnKind::Ptr,   module: "tensor" },
    RuntimeSymbol { name: "tensor_softmax",   params: 1, returns: ReturnKind::Ptr,   module: "tensor" },
    RuntimeSymbol { name: "tensor_sum",       params: 1, returns: ReturnKind::Float, module: "tensor" },
    RuntimeSymbol { name: "tensor_mean",      params: 1, returns: ReturnKind::Float, module: "tensor" },
    RuntimeSymbol { name: "tensor_max",       params: 1, returns: ReturnKind::Float, module: "tensor" },
    RuntimeSymbol { name: "tensor_min",       params: 1, returns: ReturnKind::Float, module: "tensor" },
    RuntimeSymbol { name: "tensor_reshape",   params: 3, returns: ReturnKind::Ptr,   module: "tensor" },

    // ── math_ops/nn_ops.h ────────────────────────────────────
    RuntimeSymbol { name: "nn_gelu",                params: 1, returns: ReturnKind::Ptr,   module: "nn_ops" },
    RuntimeSymbol { name: "nn_leaky_relu",          params: 2, returns: ReturnKind::Ptr,   module: "nn_ops" },
    RuntimeSymbol { name: "nn_silu",                params: 1, returns: ReturnKind::Ptr,   module: "nn_ops" },
    RuntimeSymbol { name: "nn_mse_loss",            params: 2, returns: ReturnKind::Float, module: "nn_ops" },
    RuntimeSymbol { name: "nn_cross_entropy_loss",  params: 2, returns: ReturnKind::Float, module: "nn_ops" },
    RuntimeSymbol { name: "nn_layer_norm",          params: 3, returns: ReturnKind::Ptr,   module: "nn_ops" },
    RuntimeSymbol { name: "nn_fill_random",         params: 2, returns: ReturnKind::Void,  module: "nn_ops" },
    RuntimeSymbol { name: "nn_add_bias",            params: 2, returns: ReturnKind::Ptr,   module: "nn_ops" },

    // ── nn/linear.h ──────────────────────────────────────────
    RuntimeSymbol { name: "linear_create",   params: 2, returns: ReturnKind::Ptr,  module: "linear" },
    RuntimeSymbol { name: "linear_forward",  params: 2, returns: ReturnKind::Ptr,  module: "linear" },
    RuntimeSymbol { name: "linear_free",     params: 1, returns: ReturnKind::Void, module: "linear" },
    RuntimeSymbol { name: "mlp_create",      params: 3, returns: ReturnKind::Ptr,  module: "linear" },
    RuntimeSymbol { name: "mlp_forward",     params: 2, returns: ReturnKind::Ptr,  module: "linear" },
    RuntimeSymbol { name: "mlp_free",        params: 1, returns: ReturnKind::Void, module: "linear" },

    // ── memory/arena.h ───────────────────────────────────────
    RuntimeSymbol { name: "arena_create",       params: 1, returns: ReturnKind::Ptr,  module: "arena" },
    RuntimeSymbol { name: "arena_alloc",        params: 2, returns: ReturnKind::Ptr,  module: "arena" },
    RuntimeSymbol { name: "arena_alloc_tensor", params: 3, returns: ReturnKind::Ptr,  module: "arena" },
    RuntimeSymbol { name: "arena_reset",        params: 1, returns: ReturnKind::Void, module: "arena" },
    RuntimeSymbol { name: "arena_free",         params: 1, returns: ReturnKind::Void, module: "arena" },

    // ── autograd/autograd.h ──────────────────────────────────
    RuntimeSymbol { name: "tape_create",    params: 0, returns: ReturnKind::Ptr,  module: "autograd" },
    RuntimeSymbol { name: "tape_free",      params: 1, returns: ReturnKind::Void, module: "autograd" },
    RuntimeSymbol { name: "tape_backward",  params: 1, returns: ReturnKind::Void, module: "autograd" },
    RuntimeSymbol { name: "tape_zero_grad", params: 1, returns: ReturnKind::Void, module: "autograd" },
    RuntimeSymbol { name: "ag_matmul",      params: 3, returns: ReturnKind::Ptr,  module: "autograd" },
    RuntimeSymbol { name: "ag_add",         params: 3, returns: ReturnKind::Ptr,  module: "autograd" },
    RuntimeSymbol { name: "ag_relu",        params: 2, returns: ReturnKind::Ptr,  module: "autograd" },
    RuntimeSymbol { name: "ag_softmax_ce",  params: 4, returns: ReturnKind::Ptr,  module: "autograd" },

    // ── optim/optim.h ────────────────────────────────────────
    RuntimeSymbol { name: "optim_sgd_create",  params: 3, returns: ReturnKind::Ptr,  module: "optim" },
    RuntimeSymbol { name: "optim_adam_create", params: 4, returns: ReturnKind::Ptr,  module: "optim" },
    RuntimeSymbol { name: "optim_sgd_step",    params: 4, returns: ReturnKind::Void, module: "optim" },
    RuntimeSymbol { name: "optim_adam_step",   params: 4, returns: ReturnKind::Void, module: "optim" },
    RuntimeSymbol { name: "optim_zero_grad",   params: 2, returns: ReturnKind::Void, module: "optim" },
    RuntimeSymbol { name: "optim_sgd_free",    params: 1, returns: ReturnKind::Void, module: "optim" },
    RuntimeSymbol { name: "optim_adam_free",   params: 1, returns: ReturnKind::Void, module: "optim" },

    // ── io/tensor_io.h ───────────────────────────────────────
    RuntimeSymbol { name: "tensor_save",       params: 2, returns: ReturnKind::Int, module: "tensor_io" },
    RuntimeSymbol { name: "tensor_load",       params: 1, returns: ReturnKind::Ptr, module: "tensor_io" },
    RuntimeSymbol { name: "tensor_save_multi", params: 3, returns: ReturnKind::Int, module: "tensor_io" },
    RuntimeSymbol { name: "tensor_load_multi", params: 3, returns: ReturnKind::Int, module: "tensor_io" },
    RuntimeSymbol { name: "tensor_save_raw",   params: 2, returns: ReturnKind::Int, module: "tensor_io" },
    RuntimeSymbol { name: "tensor_load_raw",   params: 3, returns: ReturnKind::Ptr, module: "tensor_io" },
];

#[derive(Debug, Clone, Copy)]
pub enum ReturnKind {
    Void,
    Int,
    Float,
    Ptr,
}

#[derive(Debug, Clone, Copy)]
pub struct RuntimeSymbol {
    pub name: &'static str,
    pub params: usize,
    pub returns: ReturnKind,
    pub module: &'static str,
}

/// Check if a function name is a Runtime 2.0 symbol
pub fn is_runtime_symbol(name: &str) -> bool {
    RT_SYMBOLS.iter().any(|s| s.name == name)
}

/// Get symbol info for a Runtime 2.0 function
pub fn get_runtime_symbol(name: &str) -> Option<&'static RuntimeSymbol> {
    RT_SYMBOLS.iter().find(|s| s.name == name)
}

/// Get all unique module names used by Runtime 2.0
pub fn runtime_modules() -> Vec<&'static str> {
    let mut modules: Vec<&str> = RT_SYMBOLS.iter().map(|s| s.module).collect();
    modules.sort();
    modules.dedup();
    modules
}

/// Runtime 2.0 DLL name for dynamic loading
pub const RUNTIME_DLL: &str = "pydead_runtime.dll";

/// Get the .o/.obj file paths for static linking
pub fn runtime_object_files(build_dir: &str, use_msvc: bool) -> Vec<String> {
    let ext = if use_msvc { "obj" } else { "o" };
    let modules = ["tensor", "arena", "nn_ops", "linear", "autograd", "optim", "tensor_io"];
    modules.iter().map(|m| format!("{}/{}.{}", build_dir, m, ext)).collect()
}
