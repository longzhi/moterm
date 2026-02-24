use std::io::Write;
use std::process::{Command, Stdio};

pub fn paste_from_clipboard() -> Result<String, String> {
    let output = Command::new("pbpaste")
        .stdout(Stdio::piped())
        .output()
        .map_err(|e| format!("启动 pbpaste 失败: {e}"))?;
    String::from_utf8(output.stdout).map_err(|e| format!("剪贴板内容非 UTF-8: {e}"))
}

pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    if text.is_empty() {
        return Ok(());
    }
    let mut child = Command::new("pbcopy")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("启动 pbcopy 失败: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| format!("写入 pbcopy 失败: {e}"))?;
    }
    child.wait().map_err(|e| format!("等待 pbcopy 失败: {e}"))?;
    Ok(())
}
