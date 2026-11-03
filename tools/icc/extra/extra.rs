fn r_nested(n: i32) -> i32 {
    let mut s = 0;
    for i in 0..n {
        while s < i {
            s += 1;
        }
    }
    s
}

fn r_closure(n: i32) -> i32 {
    let mut s = 0;
    let f = || {
        for i in 0..n {
            s += i;
        }
    };
    f();
    s
}
