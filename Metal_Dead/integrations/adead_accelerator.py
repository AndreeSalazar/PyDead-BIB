"""
ADead-BIB Accelerator para Metal-Dead
======================================
Author: Eddi Andreé Salazar Matos
Made with ❤️ in Peru 🇵🇪
"""

import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).parent.parent))
from Metal_Dead.core.metal_dead import MetalDead, MetalDeadConfig


class ADeadAccelerator:
    """Acelerador usando ADead-BIB."""
    
    def __init__(self):
        self.available = False
        try:
            sys.path.insert(0, str(Path(__file__).parent.parent.parent / "python"))
            from adead_ffi import ADeadFFI
            self.ffi = ADeadFFI()
            self.available = True
            print("⚡ ADead-BIB Accelerator disponible")
        except:
            print("⚠️ ADead-BIB Accelerator no disponible")
    
    def fast_sum(self, arr):
        if self.available:
            return self.ffi.fast_sum(arr)
        return np.sum(arr)
    
    def fast_max(self, arr):
        if self.available:
            return self.ffi.fast_max(arr)
        return np.max(arr)


class MetalDeadADead(MetalDead):
    """Metal-Dead con aceleración ADead-BIB."""
    
    def __init__(self, config: MetalDeadConfig = None):
        self.accelerator = ADeadAccelerator()
        super().__init__(config)
        print(f"⚡ ADead-BIB: {'Activo' if self.accelerator.available else 'No disponible'}")
