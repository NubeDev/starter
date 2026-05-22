// Memory DoS — string-size cap (64 KiB).
let s = "x";
let i = 0;
while i < 32 {
    s += s;
    i += 1;
}
s
