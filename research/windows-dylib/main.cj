import bridgeprobe.*

main(): Int64 {
    println(getAnswer())
    let error = makeError()
    println(error.message)
    sleep(Duration.second)
    0
}
