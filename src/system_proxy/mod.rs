use std::process::Command;

pub fn enable_system_proxy(socks_port: u16, http_port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(target_os = "windows")]
    {
        // Set proxy server address (SOCKS & HTTP)
        let proxy_server = format!("http=127.0.0.1:{};https=127.0.0.1:{};socks=127.0.0.1:{}", http_port, http_port, socks_port);
        
        let status1 = Command::new("reg")
            .args(&[
                "add",
                "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings",
                "/v",
                "ProxyServer",
                "/t",
                "REG_SZ",
                "/d",
                &proxy_server,
                "/f",
            ])
            .status()?;
            
        let status2 = Command::new("reg")
            .args(&[
                "add",
                "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings",
                "/v",
                "ProxyEnable",
                "/t",
                "REG_DWORD",
                "/d",
                "1",
                "/f",
            ])
            .status()?;

        if status1.success() && status2.success() {
            println!("[2con] Windows System Proxy Enabled: {}", proxy_server);
            // Refresh internet options so browsers pick it up immediately
            // Calling a tiny powershell script to broadcast settings refresh
            let _ = Command::new("powershell")
                .args(&[
                    "-Command",
                    "[DllImport('wininet.dll')] public static extern bool InternetSetOption(IntPtr hInternet, int dwOption, IntPtr lpBuffer, int dwBufferLength); InternetSetOption(IntPtr.Zero, 39, IntPtr.Zero, 0); InternetSetOption(IntPtr.Zero, 37, IntPtr.Zero, 0);",
                ])
                .status();
        }
    }

    #[cfg(target_os = "macos")]
    {
        // Set HTTP and HTTPS proxies for Wi-Fi interface (standard on macOS)
        let _ = Command::new("networksetup").args(&["-setwebproxy", "Wi-Fi", "127.0.0.1", &http_port.to_string()]).status();
        let _ = Command::new("networksetup").args(&["-setsecurewebproxy", "Wi-Fi", "127.0.0.1", &http_port.to_string()]).status();
        let _ = Command::new("networksetup").args(&["-setwebproxystate", "Wi-Fi", "on"]).status();
        let _ = Command::new("networksetup").args(&["-setsecurewebproxystate", "Wi-Fi", "on"]).status();
    }

    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("gsettings").args(&["set", "org.gnome.system.proxy", "mode", "manual"]).status();
        let _ = Command::new("gsettings").args(&["set", "org.gnome.system.proxy.http", "host", "127.0.0.1"]).status();
        let _ = Command::new("gsettings").args(&["set", "org.gnome.system.proxy.http", "port", &http_port.to_string()]).status();
        let _ = Command::new("gsettings").args(&["set", "org.gnome.system.proxy.socks", "host", "127.0.0.1"]).status();
        let _ = Command::new("gsettings").args(&["set", "org.gnome.system.proxy.socks", "port", &socks_port.to_string()]).status();
    }

    Ok(())
}

pub fn disable_system_proxy() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(target_os = "windows")]
    {
        let status = Command::new("reg")
            .args(&[
                "add",
                "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings",
                "/v",
                "ProxyEnable",
                "/t",
                "REG_DWORD",
                "/d",
                "0",
                "/f",
            ])
            .status()?;
            
        if status.success() {
            println!("[2con] Windows System Proxy Disabled");
            // Refresh internet options
            let _ = Command::new("powershell")
                .args(&[
                    "-Command",
                    "[DllImport('wininet.dll')] public static extern bool InternetSetOption(IntPtr hInternet, int dwOption, IntPtr lpBuffer, int dwBufferLength); InternetSetOption(IntPtr.Zero, 39, IntPtr.Zero, 0); InternetSetOption(IntPtr.Zero, 37, IntPtr.Zero, 0);",
                ])
                .status();
        }
    }

    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("networksetup").args(&["-setwebproxystate", "Wi-Fi", "off"]).status();
        let _ = Command::new("networksetup").args(&["-setsecurewebproxystate", "Wi-Fi", "off"]).status();
    }

    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("gsettings").args(&["set", "org.gnome.system.proxy", "mode", "none"]).status();
    }

    Ok(())
}
