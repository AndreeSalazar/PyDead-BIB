# PyDead-BIB C ABI Tests — struct.pack/unpack
# Tests de empaquetado binario

import struct

def test_pack_int():
    """Pack a single integer"""
    data = struct.pack("i", 42)
    return len(data) == 4  # Expected: True (4 bytes for int)

def test_pack_multiple():
    """Pack multiple values"""
    data = struct.pack("iif", 10, 20, 3.14)
    return len(data) == 12  # Expected: True (4+4+4 bytes)

def test_unpack_int():
    """Unpack a single integer"""
    data = struct.pack("i", 12345)
    value = struct.unpack("i", data)[0]
    return value == 12345

def test_unpack_multiple():
    """Unpack multiple values"""
    data = struct.pack("ii", 100, 200)
    a, b = struct.unpack("ii", data)
    return a == 100 and b == 200

def test_pack_formats():
    """Test various format characters"""
    # b = signed char (1 byte)
    # h = short (2 bytes)
    # i = int (4 bytes)
    # q = long long (8 bytes)
    # f = float (4 bytes)
    # d = double (8 bytes)
    
    b_data = struct.pack("b", 127)
    h_data = struct.pack("h", 32767)
    i_data = struct.pack("i", 2147483647)
    q_data = struct.pack("q", 9223372036854775807)
    f_data = struct.pack("f", 3.14)
    d_data = struct.pack("d", 2.718281828)
    
    return (len(b_data) == 1 and 
            len(h_data) == 2 and 
            len(i_data) == 4 and 
            len(q_data) == 8 and 
            len(f_data) == 4 and 
            len(d_data) == 8)

def test_endianness():
    """Test little-endian and big-endian"""
    # < = little-endian
    # > = big-endian
    le_data = struct.pack("<i", 0x12345678)
    be_data = struct.pack(">i", 0x12345678)
    return le_data != be_data  # Expected: True (different byte order)

def test_calcsize():
    """Test struct.calcsize"""
    size_i = struct.calcsize("i")
    size_ii = struct.calcsize("ii")
    size_iif = struct.calcsize("iif")
    return size_i == 4 and size_ii == 8 and size_iif == 12

if __name__ == "__main__":
    print("test_pack_int:", test_pack_int())
    print("test_pack_multiple:", test_pack_multiple())
    print("test_unpack_int:", test_unpack_int())
    print("test_unpack_multiple:", test_unpack_multiple())
    print("test_pack_formats:", test_pack_formats())
    print("test_endianness:", test_endianness())
    print("test_calcsize:", test_calcsize())
