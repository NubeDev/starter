// Memory DoS — map-size cap (10_000).
let m = #{};
let i = 0;
while i < 20000 {
    m["k" + i] = i;
    i += 1;
}
m.len()
