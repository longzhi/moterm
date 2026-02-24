use std::env;
use std::ffi::CString;
use std::fs::File;
use std::io::Read;
use std::os::fd::{FromRawFd, RawFd};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use polling::{Event, Events, Poller};

#[derive(Debug, Clone)]
pub enum PtyEvent {
    Output(Vec<u8>),
    Exit,
}

pub struct PtyHandle {
    master_fd: RawFd,
    pub child_pid: libc::pid_t,
}

impl PtyHandle {
    pub fn spawn<F>(cols: u16, rows: u16, mut emit: F) -> Result<Arc<Mutex<Self>>, String>
    where
        F: FnMut(PtyEvent) + Send + 'static,
    {
        let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        let shell_c = CString::new(shell.clone()).map_err(|e| e.to_string())?;

        let mut master_fd: libc::c_int = -1;
        let mut ws = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        let pid = unsafe {
            libc::forkpty(
                &mut master_fd,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut ws as *mut libc::winsize,
            )
        };

        if pid < 0 {
            return Err("forkpty 失败".to_string());
        }

        if pid == 0 {
            unsafe {
                let term = CString::new("xterm-256color").unwrap();
                let name = CString::new("TERM").unwrap();
                libc::setenv(name.as_ptr(), term.as_ptr(), 1);
                let tp_name = CString::new("TERM_PROGRAM").unwrap();
                let tp_val = CString::new("moterm").unwrap();
                libc::setenv(tp_name.as_ptr(), tp_val.as_ptr(), 1);
                let tpv_name = CString::new("TERM_PROGRAM_VERSION").unwrap();
                let tpv_val = CString::new(env!("CARGO_PKG_VERSION")).unwrap();
                libc::setenv(tpv_name.as_ptr(), tpv_val.as_ptr(), 1);
                let argv = [shell_c.as_ptr(), std::ptr::null()];
                libc::execvp(shell_c.as_ptr(), argv.as_ptr());
                libc::_exit(127);
            }
        }

        unsafe {
            let flags = libc::fcntl(master_fd, libc::F_GETFL);
            if flags >= 0 {
                libc::fcntl(master_fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
            }
        }

        let reader_fd = unsafe { libc::dup(master_fd) };
        if reader_fd < 0 {
            unsafe { libc::close(master_fd) };
            return Err("dup(master_fd) 失败".to_string());
        }

        thread::spawn(move || {
            let mut file = unsafe { File::from_raw_fd(reader_fd) };
            let poller = match Poller::new() {
                Ok(p) => p,
                Err(_) => {
                    emit(PtyEvent::Exit);
                    return;
                }
            };
            let add_result = unsafe { poller.add(&file, Event::readable(1)) };
            if add_result.is_err() {
                emit(PtyEvent::Exit);
                return;
            }
            let mut buf = vec![0u8; 8192];
            loop {
                let mut events = Events::new();
                if poller
                    .wait(&mut events, Some(Duration::from_millis(500)))
                    .is_err()
                {
                    break;
                }
                for _ev in events.iter() {
                    loop {
                        match file.read(&mut buf) {
                            Ok(0) => {
                                emit(PtyEvent::Exit);
                                return;
                            }
                            Ok(n) => emit(PtyEvent::Output(buf[..n].to_vec())),
                            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                            Err(_) => {
                                emit(PtyEvent::Exit);
                                return;
                            }
                        }
                    }
                    // polling 在部分后端是 one-shot 语义，事件处理后需要重新 arm。
                    if poller.modify(&file, Event::readable(1)).is_err() {
                        emit(PtyEvent::Exit);
                        return;
                    }
                }
            }
            emit(PtyEvent::Exit);
        });

        Ok(Arc::new(Mutex::new(Self {
            master_fd,
            child_pid: pid,
        })))
    }

    pub fn write(&self, data: &[u8]) -> Result<(), String> {
        if data.is_empty() {
            return Ok(());
        }
        let mut written = 0usize;
        while written < data.len() {
            let n = unsafe {
                libc::write(
                    self.master_fd,
                    data[written..].as_ptr() as *const libc::c_void,
                    (data.len() - written) as libc::size_t,
                )
            };
            if n < 0 {
                let err = std::io::Error::last_os_error();
                if err.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    thread::sleep(Duration::from_millis(1));
                    continue;
                }
                return Err(format!("PTY 写入失败: {err}"));
            }
            if n == 0 {
                return Err("PTY 写入失败: write 返回 0".to_string());
            }
            written += n as usize;
        }
        Ok(())
    }

    pub fn resize(&self, cols: u16, rows: u16) {
        let ws = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        unsafe {
            libc::ioctl(self.master_fd, libc::TIOCSWINSZ, &ws);
        }
    }
}

impl Drop for PtyHandle {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.master_fd);
        }
    }
}
