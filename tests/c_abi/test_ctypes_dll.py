# PyDead-BIB C ABI Tests — DLL Loading
# Tests de carga de DLLs y llamadas a funciones C

import ctypes

def test_load_kernel32():
    """Test loading kernel32.dll"""
    kernel32 = ctypes.CDLL("kernel32.dll")
    return kernel32 is not None

def test_get_process_id():
    """Test GetCurrentProcessId from kernel32"""
    kernel32 = ctypes.CDLL("kernel32.dll")
    GetCurrentProcessId = kernel32.GetCurrentProcessId
    GetCurrentProcessId.restype = ctypes.c_ulong
    pid = GetCurrentProcessId()
    return pid > 0  # Expected: True

def test_get_tick_count():
    """Test GetTickCount from kernel32"""
    kernel32 = ctypes.CDLL("kernel32.dll")
    GetTickCount = kernel32.GetTickCount
    GetTickCount.restype = ctypes.c_ulong
    ticks = GetTickCount()
    return ticks > 0  # Expected: True

def test_load_user32():
    """Test loading user32.dll"""
    user32 = ctypes.CDLL("user32.dll")
    return user32 is not None

def test_message_beep():
    """Test MessageBeep from user32"""
    user32 = ctypes.CDLL("user32.dll")
    MessageBeep = user32.MessageBeep
    MessageBeep.argtypes = [ctypes.c_uint]
    MessageBeep.restype = ctypes.c_int
    result = MessageBeep(0)
    return result  # Expected: non-zero on success

def test_c_types():
    """Test basic C types"""
    i = ctypes.c_int(42)
    f = ctypes.c_float(3.14)
    d = ctypes.c_double(2.718281828)
    l = ctypes.c_longlong(9223372036854775807)
    return i.value == 42 and f.value > 3.0

def test_c_char_p():
    """Test c_char_p (C string)"""
    s = ctypes.c_char_p(b"Hello from C!")
    return s.value == b"Hello from C!"

def test_c_void_p():
    """Test c_void_p (generic pointer)"""
    ptr = ctypes.c_void_p(0x12345678)
    return ptr.value == 0x12345678

if __name__ == "__main__":
    print("test_load_kernel32:", test_load_kernel32())
    print("test_get_process_id:", test_get_process_id())
    print("test_get_tick_count:", test_get_tick_count())
    print("test_load_user32:", test_load_user32())
    print("test_c_types:", test_c_types())
    print("test_c_char_p:", test_c_char_p())
    print("test_c_void_p:", test_c_void_p())
