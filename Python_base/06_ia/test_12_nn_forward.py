# ============================================================
# PyDead-BIB Python_base — Nivel 12: Neural Network Forward Pass
# Tests que PyDead-BIB debe compilar → Runtime 2.0 nn ops
# ============================================================

def test_linear_forward_1x3():
    """Linear layer forward: y = x @ W^T + bias (manual)"""
    # Input: [1.0, 2.0] (1x2)
    # Weight: [[0.5, 0.3], [0.1, 0.4], [0.2, 0.6]] (3x2)
    # Bias: [0.1, 0.2, 0.3]
    # y[0] = 1.0*0.5 + 2.0*0.3 + 0.1 = 1.2
    # y[1] = 1.0*0.1 + 2.0*0.4 + 0.2 = 1.1
    # y[2] = 1.0*0.2 + 2.0*0.6 + 0.3 = 1.7
    x0 = 1.0
    x1 = 2.0
    w00 = 0.5
    w01 = 0.3
    w10 = 0.1
    w11 = 0.4
    w20 = 0.2
    w21 = 0.6
    b0 = 0.1
    b1 = 0.2
    b2 = 0.3
    y0 = x0 * w00 + x1 * w01 + b0
    y1 = x0 * w10 + x1 * w11 + b1
    y2 = x0 * w20 + x1 * w21 + b2
    ok = (y0 > 1.19 and y0 < 1.21)
    ok = ok and (y1 > 1.09 and y1 < 1.11)
    ok = ok and (y2 > 1.69 and y2 < 1.71)
    return ok


def test_relu_activation():
    """ReLU: max(0, x) applied element-wise"""
    v0 = -1.5
    v1 = 0.0
    v2 = 2.3
    v3 = -0.001
    if v0 < 0.0:
        v0 = 0.0
    if v1 < 0.0:
        v1 = 0.0
    if v2 < 0.0:
        v2 = 0.0
    if v3 < 0.0:
        v3 = 0.0
    return v0 == 0.0 and v1 == 0.0 and v2 == 2.3 and v3 == 0.0


def test_sigmoid_manual():
    """Sigmoid approximation: 1/(1+e^-x) for x=0 should be 0.5"""
    # For x=0: sigmoid(0) = 1/(1+1) = 0.5
    # Using a linear approximation for small x: sigmoid(x) ~ 0.5 + 0.25*x
    x = 0.0
    sig = 0.5 + 0.25 * x  # approx for x near 0
    return sig == 0.5


def test_mlp_2layer():
    """MLP 2 layers: input(2) -> hidden(2) -> output(1)"""
    # Input
    x0 = 1.0
    x1 = 0.0
    # Layer 1 weights (2x2) + bias
    w1_00 = 0.5
    w1_01 = -0.3
    w1_10 = 0.2
    w1_11 = 0.8
    b1_0 = 0.1
    b1_1 = -0.1
    # Layer 1 forward
    h0 = x0 * w1_00 + x1 * w1_01 + b1_0  # 0.6
    h1 = x0 * w1_10 + x1 * w1_11 + b1_1  # 0.1
    # ReLU
    if h0 < 0.0:
        h0 = 0.0
    if h1 < 0.0:
        h1 = 0.0
    # Layer 2 weights (1x2) + bias
    w2_00 = 0.4
    w2_01 = 0.6
    b2_0 = 0.0
    # Layer 2 forward
    out = h0 * w2_00 + h1 * w2_01 + b2_0  # 0.6*0.4 + 0.1*0.6 = 0.24+0.06 = 0.30
    return out > 0.29 and out < 0.31


def test_softmax_manual_2class():
    """Softmax for 2 classes using exp approximation"""
    # For logits [1.0, 2.0]:
    # exp(1) ~ 2.718, exp(2) ~ 7.389
    # sum = 10.107
    # softmax = [0.2689, 0.7311]
    # We just verify the relationship
    logit0 = 1.0
    logit1 = 2.0
    # Softmax property: higher logit gets higher probability
    return logit1 > logit0  # basic check: class 1 should be more probable


def test_mse_loss():
    """Mean Squared Error: MSE = mean((pred - target)^2)"""
    p0 = 1.0
    p1 = 2.0
    p2 = 3.0
    t0 = 1.5
    t1 = 2.5
    t2 = 3.5
    # (0.25 + 0.25 + 0.25) / 3 = 0.25
    e0 = (p0 - t0) * (p0 - t0)
    e1 = (p1 - t1) * (p1 - t1)
    e2 = (p2 - t2) * (p2 - t2)
    mse = (e0 + e1 + e2) / 3.0
    return mse == 0.25


if __name__ == "__main__":
    print(test_linear_forward_1x3())
    print(test_relu_activation())
    print(test_sigmoid_manual())
    print(test_mlp_2layer())
    print(test_softmax_manual_2class())
    print(test_mse_loss())
