//! Minimal PostgreSQL Wire Protocol (v3) server for SerinDB.
//! Supports SSL negation, StartupMessage, Simple Query, and basic Extended Query.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use md5::{Digest, Md5};
use crate::auth::{AuthConfig, verify_md5_password};
use bytes::{Buf, BytesMut};
use tracing::{info, instrument};
use serin_metrics::{CONNECTIONS_TOTAL, QUERIES_TOTAL, QUERY_LATENCY_SECS};

const SSL_REQUEST_CODE: u32 = 80877103; // 0x04D2162F
const PROTOCOL_VERSION: u32 = 196608; // 3.0

/// Run a PgWire server on the given address (e.g., "0.0.0.0:5432").
#[instrument(skip(auth_conf))]
pub async fn run_server(addr: &str, auth_conf: Arc<AuthConfig>) -> anyhow::Result<()> {
    info!(%addr, "Starting PgWire server");
    let listener = TcpListener::bind(addr).await?;
    println!("PgWire server listening on {addr}");
    loop {
        let (socket, _) = listener.accept().await?;
        let auth = auth_conf.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_conn(socket, auth).await {
                eprintln!("connection error: {e}");
            }
        });
    }
}

#[instrument(skip(socket, auth))]
async fn handle_conn(mut socket: TcpStream, auth: Arc<AuthConfig>) -> anyhow::Result<()> {
    // Handle SSL negotiation or StartupMessage.
    let mut len_buf = [0u8; 4];
    socket.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len - 4];
    socket.read_exact(&mut buf).await?;
    let mut cursor = &buf[..];
    let code = cursor.get_u32();
    if code == SSL_REQUEST_CODE {
        // Respond 'N' (no SSL) and read next startup msg.
        socket.write_all(b"N").await?;
        socket.read_exact(&mut len_buf).await?;
        let len2 = u32::from_be_bytes(len_buf) as usize;
        buf.resize(len2 - 4, 0);
        socket.read_exact(&mut buf).await?;
        cursor = &buf[..];
    }
    // Parse startup.
    let protocol = code;
    if protocol != PROTOCOL_VERSION {
        send_error(&mut socket, "FATAL", "0A000", "Unsupported protocol").await?;
        return Ok(());
    }
    let mut params = HashMap::new();
    while let Some(pos) = cursor.iter().position(|&b| b == 0) {
        let key = std::str::from_utf8(&cursor[..pos])?.to_string();
        cursor.advance(pos + 1);
        if key.is_empty() { break; }
        let val_pos = cursor.iter().position(|&b| b == 0).ok_or_else(|| anyhow::anyhow!("malformed startup"))?;
        let val = std::str::from_utf8(&cursor[..val_pos])?.to_string();
        cursor.advance(val_pos + 1);
        params.insert(key, val);
    }
    // Password authentication (MD5).
    let user = params.get("user").cloned().unwrap_or_default();
    let salt = rand::random::<[u8; 4]>();
    send_auth_md5(&mut socket, &salt).await?;
    // Read PasswordMessage.
    let mut type_buf = [0u8; 1];
    socket.read_exact(&mut type_buf).await?;
    if type_buf[0] != b'p' {
        send_error(&mut socket, "FATAL", "28P01", "Password required").await?;
        return Ok(());
    }
    socket.read_exact(&mut len_buf).await?;
    let plen = u32::from_be_bytes(len_buf) as usize;
    let mut pbuf = vec![0u8; plen - 4];
    socket.read_exact(&mut pbuf).await?;
    let passwd_cstr = extract_cstr(&pbuf)?;
    let stored_pwd = auth.password(&user).unwrap_or("password");
    if !verify_md5_password(stored_pwd, &user, &passwd_cstr, &salt) {
        send_error(&mut socket, "FATAL", "28P01", "Authentication failed").await?;
        return Ok(());
    }
    send_auth_ok(&mut socket).await?;
    // ParameterStatus.
    send_param_status(&mut socket, "server_version", "13.0").await?;
    send_param_status(&mut socket, "client_encoding", "UTF8").await?;
    // ReadyForQuery.
    send_ready(&mut socket).await?;
    CONNECTIONS_TOTAL.inc();

    // State storage for prepared statements / portals.
    let stmts: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));
    let mut read_buf = BytesMut::with_capacity(8192);
    loop {
        // Read message type.
        let mut typ_buf = [0u8; 1];
        if let Err(_) = socket.read_exact(&mut typ_buf).await { break; }
        let msg_type = typ_buf[0] as char;
        socket.read_exact(&mut len_buf).await?;
        let mlen = u32::from_be_bytes(len_buf) as usize;
        read_buf.resize(mlen - 4, 0);
        socket.read_exact(&mut read_buf).await?;
        match msg_type {
            'Q' => {
                let start = std::time::Instant::now();
                // Simple Query or COPY.
                let q = extract_cstr(&read_buf)?;
                process_simple_query(&mut socket, q).await?;
                QUERIES_TOTAL.inc();
                let dur = start.elapsed();
                QUERY_LATENCY_SECS.observe(dur.as_secs_f64());
            }
            'P' => {
                // Parse
                let (name, query) = parse_parse_msg(&read_buf)?;
                stmts.lock().await.insert(name, query);
                send_parse_complete(&mut socket).await?;
            }
            'B' => {
                // Bind (ignore formats/params)
                let portal_name = parse_bind_msg(&read_buf)?;
                // For simplicity, we reuse query from unnamed statement.
                stmts.lock().await.get("");
                send_bind_complete(&mut socket).await?;
                // store portal not required for demo
            }
            'E' => {
                // Execute (ignore portal)
                process_simple_query(&mut socket, "SELECT 1".into()).await?;
            }
            'S' => {
                // Sync
                send_ready(&mut socket).await?;
            }
            _ => {
                send_error(&mut socket, "ERROR", "42601", "Unsupported message").await?;
            }
        }
    }
    Ok(())
}

// Helper functions
fn extract_cstr(buf: &[u8]) -> anyhow::Result<String> {
    if let Some(pos) = buf.iter().position(|&b| b == 0) {
        Ok(std::str::from_utf8(&buf[..pos])?.to_string())
    } else {
        anyhow::bail!("null-terminated string expected");
    }
}

fn parse_parse_msg(buf: &[u8]) -> anyhow::Result<(String, String)> {
    let mut slice = buf;
    let name = extract_cstr(slice)?;
    slice = &slice[name.len() + 1..];
    let query = extract_cstr(slice)?;
    Ok((name, query))
}

fn parse_bind_msg(buf: &[u8]) -> anyhow::Result<String> {
    let portal = extract_cstr(buf)?;
    Ok(portal)
}

async fn process_simple_query(socket: &mut TcpStream, query: String) -> anyhow::Result<()> {
    let q_lower = query.to_lowercase();
    if q_lower.starts_with("copy") {
        handle_copy(socket, &q_lower).await
    } else {
        // Always return single column "?column?" with value 1 (int4).
        send_row_description(socket).await?;
        send_data_row(socket).await?;
        send_command_complete(socket, "SELECT 1").await?;
        send_ready(socket).await?;
        Ok(())
    }
}

async fn handle_copy(socket: &mut TcpStream, query: &str) -> anyhow::Result<()> {
    if query.contains("from stdin") {
        // COPY FROM STDIN
        send_copy_in_response(socket).await?;
        // Read CopyData until CopyDone
        let mut len_buf = [0u8; 4];
        loop {
            let mut typ_buf = [0u8; 1];
            socket.read_exact(&mut typ_buf).await?;
            socket.read_exact(&mut len_buf).await?;
            let mlen = u32::from_be_bytes(len_buf) as usize;
            let mut discard = vec![0u8; mlen - 4];
            socket.read_exact(&mut discard).await?;
            match typ_buf[0] as char {
                'd' => continue, // CopyData: ignore
                'c' => break,     // CopyDone
                'f' => {
                    send_error(socket, "ERROR", "42601", "COPY failed").await?;
                    return Ok(());
                }
                _ => {
                    send_error(socket, "ERROR", "42601", "Unexpected message during COPY").await?;
                    return Ok(());
                }
            }
        }
        send_command_complete(socket, "COPY 0").await?;
        send_ready(socket).await?;
    } else if query.contains("to stdout") {
        // COPY TO STDOUT
        send_copy_out_response(socket).await?;
        // For demo, send no data.
        send_copy_done(socket).await?;
        send_command_complete(socket, "COPY 0").await?;
        send_ready(socket).await?;
    } else {
        send_error(socket, "ERROR", "42601", "Unsupported COPY variant").await?;
    }
    Ok(())
}

async fn send_auth_ok(socket: &mut TcpStream) -> anyhow::Result<()> {
    let mut msg = Vec::new();
    msg.push(b'R');
    msg.extend(&(8u32.to_be_bytes()));
    msg.extend(&(0u32.to_be_bytes()));
    socket.write_all(&msg).await?;
    Ok(())
}

async fn send_auth_md5(socket: &mut TcpStream, salt: &[u8; 4]) -> anyhow::Result<()> {
    socket.write_u8(b'R').await?;
    socket.write_u32(12u32.to_be()).await?;
    socket.write_u32(5u32.to_be()).await?; // auth MD5 code
    socket.write_all(salt).await?;
    Ok(())
}

fn md5_hex(data: &[u8]) -> String {
    let mut hasher = Md5::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

fn verify_md5_password(user: &str, client_resp: &str, salt: &[u8; 4]) -> bool {
    // In real system, get user password from catalog. Here we use "password" for all.
    let stored_pwd = "password";
    let mut inner = Vec::new();
    inner.extend_from_slice(stored_pwd.as_bytes());
    inner.extend_from_slice(user.as_bytes());
    let hash1 = md5_hex(&inner);
    let mut outer = Vec::new();
    outer.extend_from_slice(hash1.as_bytes());
    outer.extend_from_slice(salt);
    let hash2 = md5_hex(&outer);
    let expected = format!("md5{}", hash2);
    expected == client_resp
}

async fn send_param_status(socket: &mut TcpStream, key: &str, val: &str) -> anyhow::Result<()> {
    let len = (4 + key.len() + 1 + val.len() + 1) as u32;
    socket.write_u8(b'S').await?;
    socket.write_u32(len.to_be()).await?;
    socket.write_all(key.as_bytes()).await?;
    socket.write_u8(0).await?;
    socket.write_all(val.as_bytes()).await?;
    socket.write_u8(0).await?;
    Ok(())
}

async fn send_ready(socket: &mut TcpStream) -> anyhow::Result<()> {
    socket.write_u8(b'Z').await?;
    socket.write_u32(5u32.to_be()).await?;
    socket.write_u8(b'I').await?; // idle
    Ok(())
}

async fn send_row_description(socket: &mut TcpStream) -> anyhow::Result<()> {
    let field_name = b"?column?\0";
    let len = 4 + 2 + field_name.len() + 18; // 18 bytes of fixed fields
    socket.write_u8(b'T').await?;
    socket.write_u32((len as u32).to_be()).await?;
    socket.write_u16(1u16.to_be()).await?; // 1 field
    socket.write_all(field_name).await?;
    socket.write_u32(0u32.to_be()).await?; // table oid
    socket.write_u16(0u16.to_be()).await?; // attr num
    socket.write_u32(23u32.to_be()).await?; // int4 oid
    socket.write_u16(4u16.to_be()).await?; // size
    socket.write_u32((-1i32) as u32).await?; // type modifier
    socket.write_u16(0u16.to_be()).await?; // text format
    Ok(())
}

async fn send_data_row(socket: &mut TcpStream) -> anyhow::Result<()> {
    let val_bytes = b"1";
    let len = 4 + 2 + 4 + val_bytes.len();
    socket.write_u8(b'D').await?;
    socket.write_u32((len as u32).to_be()).await?;
    socket.write_u16(1u16.to_be()).await?;
    socket.write_u32((val_bytes.len() as u32).to_be()).await?;
    socket.write_all(val_bytes).await?;
    Ok(())
}

async fn send_command_complete(socket: &mut TcpStream, tag: &str) -> anyhow::Result<()> {
    let len = 4 + tag.len() + 1;
    socket.write_u8(b'C').await?;
    socket.write_u32((len as u32).to_be()).await?;
    socket.write_all(tag.as_bytes()).await?;
    socket.write_u8(0).await?;
    Ok(())
}

async fn send_parse_complete(socket: &mut TcpStream) -> anyhow::Result<()> {
    socket.write_u8(b'1').await?;
    socket.write_u32(4u32.to_be()).await?;
    Ok(())
}

async fn send_bind_complete(socket: &mut TcpStream) -> anyhow::Result<()> {
    socket.write_u8(b'2').await?;
    socket.write_u32(4u32.to_be()).await?;
    Ok(())
}

// === COPY protocol helpers ===
async fn send_copy_in_response(socket: &mut TcpStream) -> anyhow::Result<()> {
    // CopyInResponse: 'G' | len | 0=text format | 0 columns
    socket.write_u8(b'G').await?;
    socket.write_u32(7u32.to_be()).await?; // length
    socket.write_u8(0).await?; // text format
    socket.write_u16(0u16.to_be()).await?; // no column-specific formats
    Ok(())
}

async fn send_copy_out_response(socket: &mut TcpStream) -> anyhow::Result<()> {
    // CopyOutResponse: 'H'
    socket.write_u8(b'H').await?;
    socket.write_u32(7u32.to_be()).await?;
    socket.write_u8(0).await?; // text
    socket.write_u16(0u16.to_be()).await?;
    Ok(())
}

async fn send_copy_data(socket: &mut TcpStream, data: &[u8]) -> anyhow::Result<()> {
    socket.write_u8(b'd').await?;
    socket.write_u32(((4 + data.len()) as u32).to_be()).await?;
    socket.write_all(data).await?;
    Ok(())
}

async fn send_copy_done(socket: &mut TcpStream) -> anyhow::Result<()> {
    socket.write_u8(b'c').await?;
    socket.write_u32(4u32.to_be()).await?;
    Ok(())
}

async fn send_error(socket: &mut TcpStream, severity: &str, code: &str, message: &str) -> anyhow::Result<()> {
    let len = 4 + 1 + severity.len() + 1 + 1 + code.len() + 1 + 1 + message.len() + 1 + 1;
    socket.write_u8(b'E').await?;
    socket.write_u32((len as u32).to_be()).await?;
    socket.write_u8(b'S').await?;
    socket.write_all(severity.as_bytes()).await?;
    socket.write_u8(0).await?;
    socket.write_u8(b'C').await?;
    socket.write_all(code.as_bytes()).await?;
    socket.write_u8(0).await?;
    socket.write_u8(b'M').await?;
    socket.write_all(message.as_bytes()).await?;
    socket.write_u8(0).await?;
    socket.write_u8(0).await?; // terminator
    Ok(())
} 