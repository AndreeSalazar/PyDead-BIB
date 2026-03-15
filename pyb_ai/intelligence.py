intel_steps = 0
intel_positive = 0
intel_negative = 0

def intel_get_topic(tid):
    if tid == 0:
        return 0
    if tid == 1:
        return 1
    if tid == 2:
        return 2
    if tid == 3:
        return 3
    if tid == 4:
        return 4
    if tid == 5:
        return 5
    if tid == 6:
        return 6
    if tid == 7:
        return 7
    if tid == 8:
        return 8
    if tid == 9:
        return 9
    if tid == 10:
        return 10
    if tid == 11:
        return 11
    if tid == 12:
        return 12
    return 0

def intel_detect_intent(h):
    return h % 8

def intel_sentiment(h):
    global intel_positive
    global intel_negative
    score = h % 100
    if score > 60:
        intel_positive = intel_positive + 1
        return 1
    if score < 30:
        intel_negative = intel_negative + 1
        return -1
    return 0

def intel_think(input_hash):
    global intel_steps
    intel_steps = intel_steps + 1
    topic = intel_get_topic(input_hash % 13)
    intent = intel_detect_intent(input_hash)
    mood = intel_sentiment(input_hash)
    confidence = 70 + (input_hash % 30)
    print(f"think #{intel_steps}: topic={topic} intent={intent} mood={mood} conf={confidence}")
    return confidence

def intel_stats():
    print(f"pasos: {intel_steps}")
    print(f"positivo: {intel_positive}")
    print(f"negativo: {intel_negative}")

intel_think(42)
intel_think(137)
intel_think(256)
intel_think(500)
intel_think(1000)
intel_stats()
print("intelligence ok")
