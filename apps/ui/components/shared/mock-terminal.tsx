"use client"

import type React from "react"

import { useState, useRef, useEffect } from "react"
import { Terminal } from "lucide-react"

interface MockTerminalProps {
  vmId?: string
  containerId?: string
}

export function MockTerminal({ vmId, containerId }: MockTerminalProps) {
  const [lines, setLines] = useState<string[]>([])
  const [currentInput, setCurrentInput] = useState("")
  const [commandHistory, setCommandHistory] = useState<string[]>([])
  const [historyIndex, setHistoryIndex] = useState(-1)
  const terminalRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLInputElement>(null)
  const resourceId = vmId || containerId || "unknown"

  useEffect(() => {
    setLines([
      "Welcome to NQR-MicroVM Terminal",
      `Connected to ${vmId ? "VM" : "Container"}: ${resourceId}`,
      "",
      'Type "help" for available commands',
      "",
    ])
  }, [resourceId, vmId])

  useEffect(() => {
    // Auto-scroll to bottom
    if (terminalRef.current) {
      terminalRef.current.scrollTop = terminalRef.current.scrollHeight
    }
  }, [lines])

  const handleCommand = (cmd: string) => {
    const trimmedCmd = cmd.trim()
    if (!trimmedCmd) return

    // Add command to history
    setCommandHistory((prev) => [...prev, trimmedCmd])
    setHistoryIndex(-1)

    // Add command to output
    setLines((prev) => [...prev, `root@${resourceId}:~# ${trimmedCmd}`])

    // Process command
    const commands: Record<string, string[]> = {
      help: [
        "Available commands:",
        "  ls       - List directory contents",
        "  pwd      - Print working directory",
        "  whoami   - Print current user",
        "  date     - Display current date and time",
        "  uname    - Print system information",
        "  free     - Display memory usage",
        "  df       - Display disk usage",
        "  ps       - Display running processes",
        "  top      - Display system resources",
        "  cat      - Display file contents",
        "  echo     - Print text",
        "  clear    - Clear terminal",
        "  help     - Show this help message",
      ],
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
        "  789 pts/0    00:00:00 ps",
      ],
      top: [
        "top - " + new Date().toLocaleTimeString() + " up 2 days, 3:45, 1 user, load average: 0.15, 0.20, 0.18",
        "Tasks: 95 total,   1 running,  94 sleeping,   0 stopped,   0 zombie",
        "%Cpu(s):  2.3 us,  1.2 sy,  0.0 ni, 96.1 id,  0.3 wa,  0.0 hi,  0.1 si,  0.0 st",
        "MiB Mem :   2000.0 total,   1000.0 free,    500.0 used,    500.0 buff/cache",
        "",
        "  PID USER      PR  NI    VIRT    RES    SHR S  %CPU  %MEM     TIME+ COMMAND",
        "    1 root      20   0  169564  13940   8844 S   0.0   0.7   0:01.23 systemd",
        "  123 root      20   0   12345   4567   3456 S   0.0   0.2   0:00.45 sshd",
      ],
      clear: ["CLEAR"],
    }

    if (trimmedCmd === "clear") {
      setLines([])
    } else if (trimmedCmd.startsWith("echo ")) {
      const text = trimmedCmd.substring(5)
      setLines((prev) => [...prev, text])
    } else if (trimmedCmd.startsWith("cat ")) {
      const filename = trimmedCmd.substring(4)
      setLines((prev) => [
        ...prev,
        `# ${filename}`,
        "This is a mock file content.",
        "In a real terminal, this would show the actual file contents.",
        "",
      ])
    } else if (commands[trimmedCmd]) {
      setLines((prev) => [...prev, ...commands[trimmedCmd]])
    } else {
      setLines((prev) => [...prev, `bash: ${trimmedCmd}: command not found`])
    }

    setCurrentInput("")
  }

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      handleCommand(currentInput)
    } else if (e.key === "ArrowUp") {
      e.preventDefault()
      if (commandHistory.length > 0) {
        const newIndex = historyIndex === -1 ? commandHistory.length - 1 : Math.max(0, historyIndex - 1)
        setHistoryIndex(newIndex)
        setCurrentInput(commandHistory[newIndex])
      }
    } else if (e.key === "ArrowDown") {
      e.preventDefault()
      if (historyIndex !== -1) {
        const newIndex = historyIndex + 1
        if (newIndex >= commandHistory.length) {
          setHistoryIndex(-1)
          setCurrentInput("")
        } else {
          setHistoryIndex(newIndex)
          setCurrentInput(commandHistory[newIndex])
        }
      }
    } else if (e.key === "Tab") {
      e.preventDefault()
      // Simple tab completion for common commands
      const commands = [
        "help",
        "ls",
        "pwd",
        "whoami",
        "date",
        "uname",
        "free",
        "df",
        "ps",
        "top",
        "cat",
        "echo",
        "clear",
      ]
      const matches = commands.filter((cmd) => cmd.startsWith(currentInput))
      if (matches.length === 1) {
        setCurrentInput(matches[0])
      }
    }
  }

  return (
    <div
      className="bg-black rounded-lg p-4 font-mono text-sm h-[500px] flex flex-col cursor-text"
      onClick={() => inputRef.current?.focus()}
    >
      <div className="flex items-center gap-2 mb-3 pb-3 border-b border-gray-800">
        <Terminal className="h-4 w-4 text-green-500" />
        <span className="text-green-500 text-xs">
          {vmId ? "VM" : "Container"} Terminal - {resourceId}
        </span>
      </div>

      <div ref={terminalRef} className="flex-1 overflow-y-auto text-green-500 space-y-1">
        {lines.map((line, i) => (
          <div key={i} className="whitespace-pre-wrap break-all">
            {line}
          </div>
        ))}

        <div className="flex items-center gap-2">
          <span className="text-green-400">root@{resourceId}:~#</span>
          <input
            ref={inputRef}
            type="text"
            value={currentInput}
            onChange={(e) => setCurrentInput(e.target.value)}
            onKeyDown={handleKeyDown}
            className="flex-1 bg-transparent outline-none text-green-500 caret-green-500"
            autoFocus
            spellCheck={false}
          />
        </div>
      </div>

      <div className="text-xs text-gray-600 mt-2 pt-2 border-t border-gray-800">
        Press Tab for autocomplete • ↑↓ for history • Type "help" for commands
      </div>
    </div>
  )
}
