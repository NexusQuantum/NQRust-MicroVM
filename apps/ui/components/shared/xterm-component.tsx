"use client"

import { useEffect, useRef } from "react"
import { Terminal } from "xterm"
import { FitAddon } from "xterm-addon-fit"
import "xterm/css/xterm.css"

interface XTermComponentProps {
  vmId?: string
  containerId?: string
}

export function XtermComponent({ vmId, containerId }: XTermComponentProps) {
  const terminalRef = useRef<HTMLDivElement>(null)
  const xtermRef = useRef<Terminal | null>(null)
  const resourceId = vmId || containerId || "unknown"

  useEffect(() => {
    if (!terminalRef.current) return

    const term = new Terminal({
      cursorBlink: true,
      fontSize: 14,
      fontFamily: 'Menlo, Monaco, "Courier New", monospace',
      theme: {
        background: "#000000",
        foreground: "#00ff00",
        cursor: "#00ff00",
      },
      rows: 30,
    })

    const fitAddon = new FitAddon()
    term.loadAddon(fitAddon)
    term.open(terminalRef.current)
    fitAddon.fit()

    xtermRef.current = term

    term.writeln("Welcome to NQR-MicroVM Terminal")
    term.writeln(`Connected to ${vmId ? "VM" : "Container"}: ${resourceId}`)
    term.writeln("")
    term.write("root@vm:~# ")

    let currentLine = ""

    term.onData((data) => {
      if (data === "\r") {
        // Enter key
        term.write("\r\n")
        if (currentLine.trim()) {
          handleCommand(currentLine.trim(), term)
        }
        currentLine = ""
        term.write("root@vm:~# ")
      } else if (data === "\u007F") {
        // Backspace
        if (currentLine.length > 0) {
          currentLine = currentLine.slice(0, -1)
          term.write("\b \b")
        }
      } else if (data >= String.fromCharCode(0x20) && data <= String.fromCharCode(0x7e)) {
        // Printable characters
        currentLine += data
        term.write(data)
      }
    })

    const handleResize = () => {
      fitAddon.fit()
    }
    window.addEventListener("resize", handleResize)

    return () => {
      window.removeEventListener("resize", handleResize)
      term.dispose()
    }
  }, [resourceId, vmId])

  const handleCommand = (cmd: string, term: Terminal) => {
    const commands: Record<string, string[]> = {
      help: ["Available commands:", "  ls, pwd, whoami, date, uname, free, df, ps, clear, help"],
      ls: ["bin  boot  dev  etc  home  lib  media  mnt  opt  proc  root  run  sbin  srv  sys  tmp  usr  var"],
      pwd: ["/root"],
      whoami: ["root"],
      date: [new Date().toString()],
      uname: ["Linux vm-firecracker 5.10.0 #1 SMP x86_64 GNU/Linux"],
      free: [
        "              total        used        free      shared  buff/cache   available",
        "Mem:        2048000      512000     1024000       16000      512000     1400000",
        "Swap:             0           0           0",
      ],
      df: [
        "Filesystem     1K-blocks    Used Available Use% Mounted on",
        "/dev/vda1       10485760 2097152   8388608  20% /",
        "tmpfs            1024000       0   1024000   0% /dev/shm",
      ],
      ps: [
        "  PID TTY          TIME CMD",
        "    1 ?        00:00:01 systemd",
        "  123 ?        00:00:00 sshd",
        "  456 pts/0    00:00:00 bash",
      ],
      clear: ["CLEAR"],
    }

    if (cmd === "clear") {
      term.clear()
    } else if (commands[cmd]) {
      commands[cmd].forEach((line) => term.writeln(line))
    } else {
      term.writeln(`bash: ${cmd}: command not found`)
    }
  }

  return <div ref={terminalRef} className="rounded-lg overflow-hidden" />
}
