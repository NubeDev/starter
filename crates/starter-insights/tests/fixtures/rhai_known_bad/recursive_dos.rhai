// Stack/expression-depth DoS — recurse deeper than max_expr_depth.
fn r(n) {
    r(n + 1)
}
r(0)
