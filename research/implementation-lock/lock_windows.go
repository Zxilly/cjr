package main

import (
    "os"
    "syscall"
    "unsafe"
)

func lock(f *os.File) error {
    var overlapped syscall.Overlapped
    proc := syscall.NewLazyDLL("kernel32.dll").NewProc("LockFileEx")
    ok, _, err := proc.Call(f.Fd(), 3, 0, 1, 0, uintptr(unsafe.Pointer(&overlapped)))
    if ok == 0 { return err }
    return nil
}
