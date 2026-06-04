//! Garbage Collector nativo en Rust para PyDead-BIB.

use pyd_core::PyObject;
use std::cell::RefCell;
use std::rc::Rc;

pub struct GcArena {
    objects: Vec<Rc<RefCell<PyObject>>>,
    gc_counter: usize,
}

impl GcArena {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            gc_counter: 0,
        }
    }

    /// Asigna un nuevo objeto en el heap gestionado por el GC.
    pub fn allocate(&mut self, obj: PyObject) -> Rc<RefCell<PyObject>> {
        let rc_obj = Rc::new(RefCell::new(obj));
        self.objects.push(Rc::clone(&rc_obj));
        self.gc_counter += 1;
        
        // Disparar recolección cada 100 asignaciones (simulado)
        if self.gc_counter % 100 == 0 {
            self.collect();
        }
        rc_obj
    }

    /// Recolección de basura (Mark & Sweep simplificado).
    /// En una implementación completa, esto marcaría desde las raíces (stack).
    pub fn collect(&mut self) {
        log::debug!("🧹 GC: Iniciando recolección de basura...");
        // Aquí iría la lógica de mark() desde las raíces.
        // Por ahora, simulamos la limpieza de objetos no referenciados.
        // En Rust, Rc::strong_count nos ayuda a saber si algo está vivo.
        self.objects.retain(|obj| Rc::strong_count(obj) > 1);
        log::debug!("🧹 GC: Recolección completada. Objetos vivos: {}", self.objects.len());
    }
}
