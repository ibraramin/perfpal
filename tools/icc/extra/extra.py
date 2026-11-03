def p_nested(n):
    total = 0
    for i in range(n):
        while total < i:
            total += 1
    return total


def p_comp(n):
    total = 0
    for i in range(n):
        total += sum(x * 2 for x in range(i))
    return total
