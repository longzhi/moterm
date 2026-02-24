mod clipboard;
mod color;
mod config;
mod font;
mod input;
mod pty;
mod renderer;
mod terminal;
mod search;
mod url;
mod vte_handler;

use std::sync::{Arc, Mutex};

use renderer::Renderer;
use terminal::Terminal;
use vte_handler::VteHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{
    ElementState, Event, ModifiersState, MouseButton, MouseScrollDelta, WindowEvent,
};
use winit::event_loop::{ControlFlow, EventLoopBuilder};
use winit::window::WindowBuilder;

use crate::pty::{PtyEvent, PtyHandle};

#[derive(Debug, Clone)]
enum AppEvent {
    PtyOutput(Vec<u8>),
    PtyExit,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-v") {
        println!("moterm {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("moterm {} — A minimal terminal emulator", env!("CARGO_PKG_VERSION"));
        println!();
        println!("USAGE: moterm [OPTIONS]");
        println!();
        println!("OPTIONS:");
        println!("  -v, --version    Print version");
        println!("  -h, --help       Print this help");
        println!();
        println!("CONFIG: ~/.config/moterm/config.toml");
        println!("REPO:   https://github.com/longzhi/moterm");
        return;
    }
    if let Err(e) = run() {
        eprintln!("moterm 启动失败: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let cfg = config::Config::load();

    let (font, font_path) = font::load_monospace_font(&cfg)?;
    eprintln!("使用字体: {}", font_path.display());

    let event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build();
    let window = WindowBuilder::new()
        .with_title("moterm")
        .with_inner_size(LogicalSize::new(
            cfg.window.width as f64,
            cfg.window.height as f64,
        ))
        .with_resizable(true)
        .build(&event_loop)
        .map_err(|e| format!("创建窗口失败: {e}"))?;

    let scale_factor = window.scale_factor();
    let font_size = (cfg.font.size * scale_factor as f32).max(8.0);
    let mut renderer = Renderer::new(font, font_size);
    let size = window.inner_size();
    let (cols, rows) = renderer.grid_size_for_pixels(size.width as usize, size.height as usize);
    let mut term = Terminal::new(cols, rows);
    term.cursor_style = cfg.initial_cursor_style();

    let proxy = event_loop.create_proxy();
    let pty = PtyHandle::spawn(cols as u16, rows as u16, move |ev| {
        let _ = match ev {
            PtyEvent::Output(data) => proxy.send_event(AppEvent::PtyOutput(data)),
            PtyEvent::Exit => proxy.send_event(AppEvent::PtyExit),
        };
    })?;

    let mut parser = vte::Parser::new();
    let mut dirty = true;
    let mut search = search::SearchState::new();
    let mut cursor_visible = true;
    let mut cursor_blink_timer = std::time::Instant::now();
    let mut modifiers = ModifiersState::empty();
    let mut mouse_pos = PhysicalPosition::new(0.0f64, 0.0f64);
    let mut selecting = false;
    let mut last_click_time = std::time::Instant::now();
    let mut click_count: u8 = 0;

    let context = unsafe { softbuffer::Context::new(&window) }
        .map_err(|e| format!("softbuffer context 创建失败: {e}"))?;
    let mut surface = unsafe { softbuffer::Surface::new(&context, &window) }
        .map_err(|e| format!("softbuffer surface 创建失败: {e}"))?;

    event_loop.run(move |event, _, control_flow| {
        // Blink cursor every 530ms
        let blink_interval = std::time::Duration::from_millis(530);
        *control_flow = ControlFlow::WaitUntil(std::time::Instant::now() + blink_interval);

        match event {
            Event::UserEvent(AppEvent::PtyOutput(data)) => {
                {
                    let mut performer = VteHandler::new(&mut term);
                    for b in data {
                        parser.advance(&mut performer, b);
                    }
                }
                if term.bell {
                    term.bell = false;
                    // Visual bell: briefly invert isn't easy without timer,
                    // so we use macOS system beep
                    #[cfg(target_os = "macos")]
                    unsafe { libc::write(libc::STDOUT_FILENO, b"\x07".as_ptr() as _, 1); }
                }
                if term.title_changed {
                    term.title_changed = false;
                    window.set_title(if term.title.is_empty() { "moterm" } else { &term.title });
                }
                dirty = true;
                window.request_redraw();
            }
            Event::UserEvent(AppEvent::PtyExit) => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    if confirm_quit(&pty) {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                WindowEvent::Resized(new_size) => {
                    let (cols, rows) = renderer
                        .grid_size_for_pixels(new_size.width as usize, new_size.height as usize);
                    term.resize(cols, rows);
                    if let Ok(pty) = pty.lock() {
                        pty.resize(cols as u16, rows as u16);
                    }
                    // Immediately resize surface and fill with bg to prevent white flash
                    let (w_nz, h_nz) = renderer::Renderer::nonzero_dims(new_size.width, new_size.height);
                    if surface.resize(w_nz, h_nz).is_ok() {
                        if let Ok(mut buffer) = surface.buffer_mut() {
                            buffer.fill(crate::color::DEFAULT_BG.to_u32());
                            let _ = buffer.present();
                        }
                    }
                    dirty = true;
                    window.request_redraw();
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    let (cols, rows) = renderer.grid_size_for_pixels(
                        new_inner_size.width as usize,
                        new_inner_size.height as usize,
                    );
                    term.resize(cols, rows);
                    if let Ok(pty) = pty.lock() {
                        pty.resize(cols as u16, rows as u16);
                    }
                    dirty = true;
                    window.request_redraw();
                }
                WindowEvent::ModifiersChanged(m) => {
                    modifiers = m;
                }
                WindowEvent::ReceivedCharacter(ch) => {
                    if search.active {
                        if !ch.is_control() && !modifiers.logo() && !modifiers.ctrl() {
                            search.push_char(ch);
                            search.search(&term);
                            dirty = true;
                            window.request_redraw();
                        }
                        return;
                    }
                    if let Some(bytes) = input::map_received_char(ch, modifiers) {
                        cursor_visible = true;
                        cursor_blink_timer = std::time::Instant::now();
                        write_pty(&pty, &bytes);
                    }
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    if input.state != ElementState::Pressed {
                        return;
                    }
                    if let Some(key) = input.virtual_keycode {
                        // Search mode key handling
                        if search.active && !modifiers.logo() {
                            match key {
                                winit::event::VirtualKeyCode::Escape => {
                                    search.close();
                                    dirty = true;
                                    window.request_redraw();
                                }
                                winit::event::VirtualKeyCode::Back => {
                                    search.pop_char();
                                    search.search(&term);
                                    dirty = true;
                                    window.request_redraw();
                                }
                                winit::event::VirtualKeyCode::Return => {
                                    search.next_match();
                                    if let Some(m) = search.current_match() {
                                        let vis_start = term.visible_start_global_row();
                                        let vis_end = vis_start + term.rows();
                                        if m.global_row < vis_start || m.global_row >= vis_end {
                                            let total = term.total_lines();
                                            let scroll = total.saturating_sub(m.global_row + term.rows());
                                            term.view_scroll = scroll;
                                        }
                                    }
                                    dirty = true;
                                    window.request_redraw();
                                }
                                _ => {}
                            }
                            return;
                        }
                        if modifiers.logo() {
                            match key {
                                // Cmd+C: copy
                                winit::event::VirtualKeyCode::C if term.selection_non_empty() => {
                                    let text = term.selection_text_or_empty();
                                    if let Err(e) = clipboard::copy_to_clipboard(&text) {
                                        eprintln!("复制失败: {e}");
                                    }
                                    return;
                                }
                                // Cmd+V: paste (with bracketed paste support)
                                winit::event::VirtualKeyCode::V => {
                                    match clipboard::paste_from_clipboard() {
                                        Ok(text) if !text.is_empty() => {
                                            // Bracketed paste mode
                                            write_pty(&pty, b"\x1b[200~");
                                            write_pty(&pty, text.as_bytes());
                                            write_pty(&pty, b"\x1b[201~");
                                        }
                                        Err(e) => eprintln!("粘贴失败: {e}"),
                                        _ => {}
                                    }
                                    return;
                                }
                                // Cmd+N: new window
                                winit::event::VirtualKeyCode::N => {
                                    let exe = std::env::current_exe().unwrap_or_default();
                                    let _ = std::process::Command::new(exe).spawn();
                                    return;
                                }
                                // Cmd+Q: quit (with confirmation if child running)
                                winit::event::VirtualKeyCode::Q => {
                                    if confirm_quit(&pty) {
                                        *control_flow = ControlFlow::Exit;
                                    }
                                    return;
                                }
                                // Cmd+= / Cmd++: zoom in
                                winit::event::VirtualKeyCode::Equals => {
                                    renderer.adjust_font_size(2.0);
                                    let size = window.inner_size();
                                    let (cols, rows) = renderer.grid_size_for_pixels(size.width as usize, size.height as usize);
                                    term.resize(cols, rows);
                                    if let Ok(pty) = pty.lock() { pty.resize(cols as u16, rows as u16); }
                                    dirty = true;
                                    window.request_redraw();
                                    return;
                                }
                                // Cmd+-: zoom out
                                winit::event::VirtualKeyCode::Minus => {
                                    renderer.adjust_font_size(-2.0);
                                    let size = window.inner_size();
                                    let (cols, rows) = renderer.grid_size_for_pixels(size.width as usize, size.height as usize);
                                    term.resize(cols, rows);
                                    if let Ok(pty) = pty.lock() { pty.resize(cols as u16, rows as u16); }
                                    dirty = true;
                                    window.request_redraw();
                                    return;
                                }
                                // Cmd+0: reset zoom
                                winit::event::VirtualKeyCode::Key0 => {
                                    let default_size = (cfg.font.size * scale_factor as f32).max(8.0);
                                    renderer.set_font_size(default_size);
                                    let size = window.inner_size();
                                    let (cols, rows) = renderer.grid_size_for_pixels(size.width as usize, size.height as usize);
                                    term.resize(cols, rows);
                                    if let Ok(pty) = pty.lock() { pty.resize(cols as u16, rows as u16); }
                                    dirty = true;
                                    window.request_redraw();
                                    return;
                                }
                                // Cmd+K: clear scrollback
                                winit::event::VirtualKeyCode::K => {
                                    term.clear_scrollback();
                                    dirty = true;
                                    window.request_redraw();
                                    return;
                                }
                                // Cmd+A: select all
                                winit::event::VirtualKeyCode::A => {
                                    term.select_all();
                                    dirty = true;
                                    window.request_redraw();
                                    return;
                                }
                                // Cmd+F: toggle search
                                winit::event::VirtualKeyCode::F => {
                                    search.toggle();
                                    dirty = true;
                                    window.request_redraw();
                                    return;
                                }
                                // Cmd+G: next search match
                                winit::event::VirtualKeyCode::G => {
                                    if search.active {
                                        if modifiers.shift() {
                                            search.prev_match();
                                        } else {
                                            search.next_match();
                                        }
                                        // Scroll to current match
                                        if let Some(m) = search.current_match() {
                                            let vis_start = term.visible_start_global_row();
                                            let vis_end = vis_start + term.rows();
                                            if m.global_row < vis_start || m.global_row >= vis_end {
                                                let total = term.total_lines();
                                                let scroll = total.saturating_sub(m.global_row + term.rows());
                                                term.view_scroll = scroll;
                                            }
                                        }
                                        dirty = true;
                                        window.request_redraw();
                                    }
                                    return;
                                }
                                _ => {}
                            }
                        }

                        match key {
                            winit::event::VirtualKeyCode::PageUp if modifiers.shift() => {
                                term.scroll_view_page(1);
                                dirty = true;
                                window.request_redraw();
                            }
                            winit::event::VirtualKeyCode::PageDown if modifiers.shift() => {
                                term.scroll_view_page(-1);
                                dirty = true;
                                window.request_redraw();
                            }
                            winit::event::VirtualKeyCode::Home if modifiers.shift() => {
                                term.set_view_scroll(term.max_view_scroll() as isize);
                                dirty = true;
                                window.request_redraw();
                            }
                            winit::event::VirtualKeyCode::End if modifiers.shift() => {
                                term.scroll_view_to_bottom();
                                dirty = true;
                                window.request_redraw();
                            }
                            _ => {
                                if let Some(bytes) = input::map_special_key(key, modifiers) {
                                    write_pty(&pty, &bytes);
                                }
                            }
                        }
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    mouse_pos = position;
                    if selecting {
                        if let Some((view_row, col)) = pixel_to_cell(&renderer, &window, mouse_pos)
                        {
                            term.set_selection_focus_from_view(view_row, col);
                            dirty = true;
                            window.request_redraw();
                        }
                    }
                }
                WindowEvent::MouseInput {
                    state,
                    button: MouseButton::Left,
                    ..
                } => match state {
                    ElementState::Pressed => {
                        if let Some((view_row, col)) = pixel_to_cell(&renderer, &window, mouse_pos)
                        {
                            // Cmd+click: open URL
                            if modifiers.logo() {
                                if let Some(row) = term.visible_line(view_row) {
                                    let line_text: String = row.cells.iter().map(|c| c.ch).collect();
                                    for (start, end, u) in url::detect_urls(&line_text) {
                                        if col >= start && col < end {
                                            eprintln!("打开 URL: {u}");
                                            url::open_url(&u);
                                            return;
                                        }
                                    }
                                }
                            }
                            // Track click count for double/triple click
                            let now = std::time::Instant::now();
                            if now.duration_since(last_click_time).as_millis() < 400 {
                                click_count = (click_count + 1).min(3);
                            } else {
                                click_count = 1;
                            }
                            last_click_time = now;

                            match click_count {
                                2 => term.select_word_at_view(view_row, col),
                                3 => term.select_line_at_view(view_row),
                                _ => {
                                    selecting = true;
                                    term.start_selection_from_view(view_row, col);
                                }
                            }
                            dirty = true;
                            window.request_redraw();
                        }
                    }
                    ElementState::Released => {
                        selecting = false;
                    }
                },
                WindowEvent::MouseWheel { delta, .. } => {
                    let lines = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y.round() as isize,
                        MouseScrollDelta::PixelDelta(p) => {
                            (p.y / renderer.atlas.cell_height as f64).round() as isize
                        }
                    };
                    if lines != 0 {
                        term.set_view_scroll(-lines);
                        dirty = true;
                        window.request_redraw();
                    }
                }
                _ => {}
            },
            Event::RedrawRequested(_) => {
                if !dirty {
                    return;
                }
                let size = window.inner_size();
                let (w_nz, h_nz) = renderer::Renderer::nonzero_dims(size.width, size.height);
                if let Err(e) = surface.resize(w_nz, h_nz) {
                    eprintln!("surface resize 失败: {e}");
                    *control_flow = ControlFlow::Exit;
                    return;
                }

                renderer.cursor_visible = cursor_visible;
                if search.active {
                    renderer.render_with_search(&term, &search, size.width as usize, size.height as usize);
                } else {
                    renderer.render(&term, size.width as usize, size.height as usize);
                }

                match surface.buffer_mut() {
                    Ok(mut buffer) => {
                        let bg = crate::color::DEFAULT_BG.to_u32();
                        if buffer.len() == renderer.canvas.pixels.len() {
                            buffer.copy_from_slice(&renderer.canvas.pixels);
                        } else {
                            // Fill entire buffer with bg first, then copy canvas
                            buffer.fill(bg);
                            for (dst, src) in buffer
                                .iter_mut()
                                .zip(renderer.canvas.pixels.iter().copied())
                            {
                                *dst = src;
                            }
                        }
                        if let Err(e) = buffer.present() {
                            eprintln!("present 失败: {e}");
                            *control_flow = ControlFlow::Exit;
                            return;
                        }
                    }
                    Err(e) => {
                        eprintln!("获取绘制缓冲区失败: {e}");
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                }
                dirty = false;
            }
            Event::MainEventsCleared => {
                let now = std::time::Instant::now();
                if now.duration_since(cursor_blink_timer).as_millis() >= 530 {
                    cursor_visible = !cursor_visible;
                    cursor_blink_timer = now;
                    dirty = true;
                    window.request_redraw();
                }
            }
            _ => {}
        }
    });
}

fn pixel_to_cell(
    renderer: &Renderer,
    window: &winit::window::Window,
    pos: PhysicalPosition<f64>,
) -> Option<(usize, usize)> {
    let size = window.inner_size();
    if size.width == 0 || size.height == 0 {
        return None;
    }
    let x = pos.x.max(0.0) as usize;
    let y = pos.y.max(0.0) as usize;
    if x < renderer.padding_x || y < renderer.padding_y {
        return None;
    }
    let gx = x - renderer.padding_x;
    let gy = y - renderer.padding_y;
    Some((
        gy / renderer.atlas.cell_height,
        gx / renderer.atlas.cell_width,
    ))
}

fn confirm_quit(pty: &Arc<Mutex<PtyHandle>>) -> bool {
    // Check if child process has sub-processes running
    let has_children = if let Ok(pty) = pty.lock() {
        let pid = pty.child_pid;
        // Check if shell has child processes (commands running)
        let output = std::process::Command::new("pgrep")
            .args(["-P", &pid.to_string()])
            .output();
        matches!(output, Ok(o) if !o.stdout.is_empty())
    } else {
        false
    };

    if !has_children {
        return true;
    }

    // Show macOS native confirmation dialog
    let result = std::process::Command::new("osascript")
        .args([
            "-e",
            r#"display dialog "有进程正在运行，确定要关闭 Moterm 吗？" buttons {"取消", "关闭"} default button "取消" with icon caution with title "Moterm""#,
        ])
        .output();

    match result {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            stdout.contains("关闭")
        }
        Err(_) => true, // If dialog fails, allow quit
    }
}

fn write_pty(pty: &Arc<Mutex<PtyHandle>>, bytes: &[u8]) {
    match pty.lock() {
        Ok(pty) => {
            if let Err(e) = pty.write(bytes) {
                eprintln!("写入 PTY 失败 ({} bytes): {}", bytes.len(), e);
            }
        }
        Err(e) => {
            eprintln!("PTY 锁失败: {e}");
        }
    }
}
