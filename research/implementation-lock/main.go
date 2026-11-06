package main

import (
    "fmt"
    "os"
)

// Cross-language probe for cjv's ordered lifecycle/lookup OS-lock protocol.
func main() {
    for _, name := range os.Args[1:] {
        file, err := os.OpenFile(name, os.O_RDWR, 0)
        if err != nil { panic(err) }
        defer file.Close()
        if err := lock(file); err != nil { panic(err) }
    }
    fmt.Println("locked")
    var b [1]byte
    _, _ = os.Stdin.Read(b[:])
}
