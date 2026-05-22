// Memory DoS — array-size cap (10_000).
let a = [];
let i = 0;
while i < 20000 {
    a.push(i);
    i += 1;
}
a.len()
