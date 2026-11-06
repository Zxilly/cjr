import probelib.*
main(): Int64 {
    if (answer() != 42) { return 21 }
    if (greeting() != "动态-library-ok") { return 22 }
    println("PASS dynamic=${greeting()} answer=${answer()}")
    return 0
}
