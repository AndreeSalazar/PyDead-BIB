import os

def fix_mods():
    # Fix middle/mod.rs
    middle_mod = "src/rust/middle/mod.rs"
    if os.path.exists(middle_mod):
        with open(middle_mod, "r", encoding="utf-8") as f:
            content = f.read()
        content = content.replace("pub mod ir;", "pub mod ir_old;\npub use ir_old as ir;\npub mod ir_scaffold { pub mod opcodes; pub mod cfg; }")
        with open(middle_mod, "w", encoding="utf-8") as f:
            f.write(content)

    # Fix backend/mod.rs
    backend_mod = "src/rust/backend/mod.rs"
    if os.path.exists(backend_mod):
        with open(backend_mod, "r", encoding="utf-8") as f:
            content = f.read()
        content = content.replace("pub mod isa;", "\npub mod isa_old;\npub use isa_old as isa;\n//pub mod isa;")
        content = content.replace("pub mod jit;", "\npub mod jit_old;\npub use jit_old as jit;\n//pub mod jit;")
        with open(backend_mod, "w", encoding="utf-8") as f:
            f.write(content)

if __name__ == "__main__":
    fix_mods()
    print("Mod files patched for backwards compatibility compilation!")
