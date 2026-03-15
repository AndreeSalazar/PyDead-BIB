ctx_interactions = 0
ctx_interests = 0
ctx_facts = 0

def ctx_update():
    global ctx_interactions
    ctx_interactions = ctx_interactions + 1

def ctx_add_interest():
    global ctx_interests
    ctx_interests = ctx_interests + 1
    print(f"interes #{ctx_interests} registrado")

def ctx_add_fact():
    global ctx_facts
    ctx_facts = ctx_facts + 1
    print(f"hecho #{ctx_facts} aprendido")

def ctx_greeting(count):
    if count == 0:
        print("Hola! Soy Metal-Dead para PyDead-BIB")
    elif count < 3:
        print("Hola de nuevo! En que puedo ayudarte?")
    elif count < 10:
        print("Me alegra verte de nuevo!")
    else:
        print("Que bueno verte otra vez!")

def ctx_show():
    print(f"interacciones: {ctx_interactions}")
    print(f"intereses: {ctx_interests}")
    print(f"hechos: {ctx_facts}")

ctx_greeting(0)
ctx_update()
ctx_update()
ctx_update()
ctx_add_interest()
ctx_add_interest()
ctx_add_fact()
ctx_greeting(ctx_interactions)
ctx_show()
print("context ok")
