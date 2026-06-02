import { exec } from "child_process";

export interface PortDiscoveryResult {
  ports: string[];
  error?: string;
}

/**
 * 通过 powershell.exe 枚举 Windows COM 口（与 wsl-idf 内部使用相同的 WMI 查询）。
 * 仅适用于 WSL2 / Windows 环境。
 */
export function listSerialPorts(): Promise<PortDiscoveryResult> {
  const cmd =
    "powershell.exe -NoProfile -Command \"Get-WmiObject -Class Win32_SerialPort | Select-Object -ExpandProperty DeviceID\"";

  return new Promise((resolve) => {
    exec(cmd, { timeout: 8000 }, (err, stdout, stderr) => {
      if (err) {
        resolve({ ports: [], error: stderr?.trim() || err.message });
        return;
      }
      const ports = stdout
        .split(/\r?\n/)
        .map((s) => s.trim())
        .filter((s) => /^COM\d+$/i.test(s))
        .map((s) => s.toUpperCase());
      const unique = Array.from(new Set(ports)).sort((a, b) => {
        const na = parseInt(a.replace(/\D/g, ""), 10);
        const nb = parseInt(b.replace(/\D/g, ""), 10);
        return na - nb;
      });
      resolve({ ports: unique });
    });
  });
}
