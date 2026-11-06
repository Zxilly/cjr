import std.collection.*
import std.sync.*

main() {
    let values = ArrayList<String>()
    for (i in 0..20000) {
        values.add("value-${i}")
    }
    if (values.size != 20000 || values[123] != "value-123") {
        throw Exception("collection verification failed")
    }
    var caught = false
    try {
        throw Exception("expected-probe-exception")
    } catch (e: Exception) {
        caught = e.message == "expected-probe-exception"
    }
    if (!caught) { throw Exception("exception verification failed") }
    let worker = spawn { =>
        var total: Int64 = 0
        for (i in 0..10000) { total += i }
        total
    }
    if (worker.get() != 49995000) { throw Exception("thread verification failed") }
    var checksum: Int64 = 0
    for (batch in 0..100) {
        let garbage = ArrayList<String>()
        for (i in 0..2000) { garbage.add("allocation-${batch}-${i}") }
        checksum += garbage.size
    }
    if (checksum != 200000) { throw Exception("allocation verification failed") }
    println("PASS collections exceptions threads allocations")
    sleep(Duration.second)
    0
}
