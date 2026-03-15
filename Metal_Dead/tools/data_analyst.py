def da_hash(length):
    h = length * 31 + 7
    h = h * 17 + 13
    h = h % 65536
    return h

def da_mean(n):
    total = 0
    i = 0
    while i < n:
        total = total + i
        i = i + 1
    if n == 0:
        return 0
    return total // n

def da_variance(n, mean):
    total = 0
    i = 0
    while i < n:
        diff = i - mean
        total = total + diff * diff
        i = i + 1
    if n == 0:
        return 0
    return total // n

def da_min_max(n):
    mn = 0
    mx = 0
    i = 0
    while i < n:
        val = da_hash(i + 1) % 1000
        if i == 0:
            mn = val
            mx = val
        else:
            if val < mn:
                mn = val
            if val > mx:
                mx = val
        i = i + 1
    return mn

def da_count_above(n, threshold):
    count = 0
    i = 0
    while i < n:
        val = da_hash(i + 1) % 1000
        if val > threshold:
            count = count + 1
        i = i + 1
    return count

def da_histogram(n, bins):
    counts = [0, 0, 0, 0, 0]
    i = 0
    while i < n:
        val = da_hash(i + 1) % 1000
        bin_idx = val * bins // 1000
        if bin_idx >= bins:
            bin_idx = bins - 1
        if bin_idx < 5:
            counts[bin_idx] = counts[bin_idx] + 1
        i = i + 1
    return counts[0]

def da_correlate(n):
    sum_xy = 0
    i = 0
    while i < n:
        x = da_hash(i + 1) % 100
        y = da_hash(i + 50) % 100
        sum_xy = sum_xy + x * y
        i = i + 1
    return sum_xy

def da_analyze(data_len):
    m = da_mean(data_len)
    v = da_variance(data_len, m)
    mn = da_min_max(data_len)
    above = da_count_above(data_len, 500)
    print(f"analisis: n={data_len} mean={m} var={v} min_region={mn}")
    print(f"  valores > 500: {above}")
    return m

print("============================================================")
print("   Data Analyst para PyDead-BIB v3.0")
print("   Analisis de datos — Compilado NATIVO")
print("============================================================")
da_analyze(50)
da_analyze(100)
da_analyze(200)
h = da_histogram(100, 5)
print(f"histograma bin[0]: {h}")
c = da_correlate(50)
print(f"correlacion: {c}")
print("")
print("data_analyst ok")
