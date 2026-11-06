import std.collection.*

func churn(seed: Int64): Int64 {
    let values = ArrayList<String>()
    for (i in 0..2000) {
        values.add("仓颉-runtime-${seed}-${i}")
    }
    var sum: Int64 = 0
    for (value in values) { sum += value.size }
    return sum
}

main(): Int64 {
    let values = ArrayList<Int64>()
    for (i in 0..10000) { values.add(i) }
    var sum: Int64 = 0
    for (value in values) { sum += value }
    if (sum != 49995000) { return 11 }
    var caught = false
    try { throw Exception("cjr-exception") } catch (e: Exception) {
        caught = e.message == "cjr-exception"
    }
    if (!caught) { return 12 }
    var allocated: Int64 = 0
    for (round in 0..300) { allocated += churn(round) }
    if (allocated <= 0) { return 13 }
    let task = spawn { => 21 * 2 }
    if (task.get() != 42) { return 14 }
    println("PASS sum=${sum} allocated=${allocated} thread=42 exception=true")
    return 0
}
