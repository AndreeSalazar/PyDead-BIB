# ============================================================
# PyDead-BIB Python_base — Nivel 13: Training Loop (SGD)
# Tests que PyDead-BIB debe compilar → Runtime 2.0 optim
# ============================================================

def test_sgd_step():
    """SGD básico: param -= lr * grad"""
    param = 5.0
    grad = 2.0
    lr = 0.1
    param = param - lr * grad  # 5.0 - 0.2 = 4.8
    return param == 4.8


def test_sgd_multiple_steps():
    """SGD convergencia: param debería acercarse a 0"""
    param = 10.0
    lr = 0.1
    for i in range(20):
        grad = param * 2.0  # d/dx(x^2) = 2x
        param = param - lr * grad
    # Después de 20 pasos, param debe estar cerca de 0
    return param < 0.5 and param > -0.5


def test_gradient_descent_quadratic():
    """Minimizar f(x) = (x-3)^2 con SGD"""
    x = 0.0
    lr = 0.1
    for i in range(50):
        grad = 2.0 * (x - 3.0)  # d/dx (x-3)^2 = 2(x-3)
        x = x - lr * grad
    # x debería converger a 3.0
    return x > 2.9 and x < 3.1


def test_two_param_optimization():
    """Optimizar 2 parámetros: f(a,b) = (a-1)^2 + (b-2)^2"""
    a = 0.0
    b = 0.0
    lr = 0.1
    for i in range(50):
        grad_a = 2.0 * (a - 1.0)
        grad_b = 2.0 * (b - 2.0)
        a = a - lr * grad_a
        b = b - lr * grad_b
    ok = (a > 0.9 and a < 1.1) and (b > 1.9 and b < 2.1)
    return ok


def test_loss_decreases():
    """Verificar que el loss disminuye durante entrenamiento"""
    x = 5.0
    lr = 0.1
    initial_loss = (x - 2.0) * (x - 2.0)
    for i in range(30):
        grad = 2.0 * (x - 2.0)
        x = x - lr * grad
    final_loss = (x - 2.0) * (x - 2.0)
    return final_loss < initial_loss


def test_weight_update_momentum():
    """SGD con momentum manual"""
    param = 5.0
    velocity = 0.0
    lr = 0.01
    momentum = 0.9
    for i in range(100):
        grad = 2.0 * param
        velocity = momentum * velocity + grad
        param = param - lr * velocity
    # After 100 steps with small lr, should converge near 0
    return param < 0.5 and param > -0.5


if __name__ == "__main__":
    print(test_sgd_step())
    print(test_sgd_multiple_steps())
    print(test_gradient_descent_quadratic())
    print(test_two_param_optimization())
    print(test_loss_decreases())
    print(test_weight_update_momentum())
