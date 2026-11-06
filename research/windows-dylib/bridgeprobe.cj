package bridgeprobe

public func getAnswer(): Int64 {
    let values = Array<Int64>(1024, { i: Int64 => i })
    values[1023]
}

public func makeError(): Exception {
    Exception("cross-library exception")
}
