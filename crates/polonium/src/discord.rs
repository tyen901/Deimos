use serde::Deserialize;
use serde_json::{Value, json};
use std::io::{Read, Write};

#[derive(Debug, Deserialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
}

fn open_ipc_socket() -> std::io::Result<impl Read + Write> {
    #[cfg(windows)]
    {
        use std::fs::OpenOptions;
        for i in 0..10u8 {
            let path = format!(r"\\.\pipe\discord-ipc-{i}");
            if let Ok(f) = OpenOptions::new().read(true).write(true).open(&path) {
                return Ok(f);
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Discord IPC pipe not found",
        ))
    }

    #[cfg(unix)]
    {
        use std::os::unix::net::UnixStream;

        let candidates: Vec<String> = {
            let mut v = Vec::new();

            if let Ok(xdg) = std::env::var("XDG_RUNTIME_DIR") {
                for i in 0..10u8 {
                    v.push(format!("{xdg}/discord-ipc-{i}"));
                }
            }

            if let Ok(xdg) = std::env::var("XDG_RUNTIME_DIR") {
                for i in 0..10u8 {
                    v.push(format!("{xdg}/app/com.discordapp.Discord/discord-ipc-{i}"));
                }
            }

            for i in 0..10u8 {
                v.push(format!("/tmp/discord-ipc-{i}"));
            }
            v
        };

        for path in &candidates {
            if let Ok(s) = UnixStream::connect(path) {
                return Ok(s);
            }
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Discord IPC socket not found",
        ))
    }
}

fn write_frame(sock: &mut impl Write, opcode: u32, payload: &[u8]) -> std::io::Result<()> {
    let len = payload.len() as u32;
    sock.write_all(&opcode.to_le_bytes())?;
    sock.write_all(&len.to_le_bytes())?;
    sock.write_all(payload)?;
    Ok(())
}

fn read_frame(sock: &mut impl Read) -> std::io::Result<(u32, Vec<u8>)> {
    let mut header = [0u8; 8];
    sock.read_exact(&mut header)?;
    let opcode = u32::from_le_bytes(header[0..4].try_into().unwrap());
    let len = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
    let mut payload = vec![0u8; len];
    sock.read_exact(&mut payload)?;
    Ok((opcode, payload))
}

pub fn get_discord_user(client_id: &str) -> anyhow::Result<DiscordUser> {
    let mut sock = open_ipc_socket()?;

    let handshake = json!({ "v": 1, "client_id": client_id }).to_string();
    write_frame(&mut sock, 0, handshake.as_bytes())?;

    let (opcode, payload) = read_frame(&mut sock)?;
    if opcode == 2 {
        let msg = String::from_utf8_lossy(&payload);
        anyhow::bail!("Discord closed connection: {msg}");
    }

    let value: Value = serde_json::from_slice(&payload)?;
    let user = value
        .pointer("/data/user")
        .ok_or_else(|| anyhow::anyhow!("No user in READY payload:\n{value:#}"))?;

    Ok(serde_json::from_value(user.clone())?)
}
