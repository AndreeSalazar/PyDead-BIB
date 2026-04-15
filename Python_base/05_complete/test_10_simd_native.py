def test_simd_float_vector():
    # SIMD representativo
    arr1 = [1.0, 2.0, 3.0, 4.0]
    arr2 = [1.0, 2.0, 3.0, 4.0]
    out = [0.0, 0.0, 0.0, 0.0]
    for i in range(4):
        out[i] = arr1[i] + arr2[i]
    return sum(out) == 20.0

def test_opt_const_folding():
    x = 100 * 200 + 300 - 50
    return x == 20250

if __name__ == "__main__":
    print(test_simd_float_vector())
    print(test_opt_const_folding())
