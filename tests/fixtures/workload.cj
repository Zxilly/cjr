import std.collection.*
import std.sync.*

main(args: Array<String>): Int64 {
    let values = ArrayList<String>()
    for (i in 0..20000) { values.add("value-${i}") }
    if (values.size != 20000 || values[123] != "value-123") { return 11 }
    var caught = false
    try { throw Exception("expected") } catch (e: Exception) { caught = e.message == "expected" }
    if (!caught) { return 12 }
    let worker = spawn { => 21 * 2 }
    if (worker.get() != 42) { return 13 }
    var count: Int64 = 0
    for (batch in 0..100) {
        let garbage = ArrayList<String>()
        for (i in 0..2000) { garbage.add("allocation-${batch}-${i}") }
        count += garbage.size
    }
    if (count != 200000) { return 14 }
    for (arg in args) { println("ARG=[${arg}]") }
    println("PASS collections exceptions threads allocations")
    sleep(Duration.second)
    return 37
}
