/*
 * 07_struct_by_value.c — by-value struct parameters and padding waste.
 *
 * Expected findings:
 *   PERF-C-TYPE-001  warn  large struct(s) passed by value, ONE finding per
 *                          function: transform() reports its two parameters
 *                          (struct Vec3, struct Big) in a single finding; a
 *                          second finding covers use_tight() (struct Tight sits
 *                          exactly on the 32-byte floor)
 *   PERF-C-TYPE-002  info  struct member ordering leaves >= 8 bytes of padding
 *                          (struct Padded: 11 bytes)
 *
 * Size model is the documented LP64 approximation: pointer/long/double = 8,
 * int/float = 4, short = 2, char = 1, tail-rounded to the widest member.
 */

#include <stdio.h>

/* 40 bytes: 3 doubles (24) + long (8) + int (4) + int (4). */
struct Vec3 {
    double x;
    double y;
    double z;
    long id;
    int flags;
    int tag;
};

/* ~48 bytes aligned (44 packed + 4 tail padding): counted by TYPE-001. */
struct Big {
    double a;
    double b;
    double c;
    double d;
    long handle;
    int kind;
};

/* Packed sum 13 bytes, but alignment pushes the footprint to 24 -> 11 bytes
 * of padding: the TYPE-002 shape. */
struct Padded {
    char tag;
    double value;
    int count;
};

/* Two large structs copied by value on every call. */
void transform(struct Vec3 v, struct Big b)
{
    printf("%f %f %f %ld %d %d\n", v.x, v.y, v.z, v.id, v.flags, v.tag);
    printf("%f %f %f %f %ld %d\n", b.a, b.b, b.c, b.d, b.handle, b.kind);
}

struct Padded make_padded(char tag, double value, int count)
{
    struct Padded p;

    p.tag = tag;
    p.value = value;
    p.count = count;
    return p;
}

/* 24 bytes estimated: below the 32-byte by-value floor -> silence. */
void print_padded(struct Padded p)
{
    printf("%c %f %d\n", p.tag, p.value, p.count);
}

/* 32 bytes packed: below the padding floor, but exactly ON the 32-byte
 * by-value floor, so use_tight() reports TYPE-001 while TYPE-002 stays silent. */
struct Tight {
    double a;
    double b;
    long c;
    int d;
    int e;
};

void use_tight(struct Tight t)
{
    printf("%f %ld %d %d\n", t.a + t.b, t.c, t.d, t.e);
}
